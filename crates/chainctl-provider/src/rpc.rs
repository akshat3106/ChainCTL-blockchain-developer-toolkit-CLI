use std::sync::Arc;
use std::time::{Duration, Instant};

use chainctl_core::{RpcCheckResult, RpcLatencyStats};
use serde_json::json;
use tokio::sync::Semaphore;

use crate::jsonrpc;

/// Delay between successive samples in `benchmark`, so a `--samples 20` run
/// doesn't fire 20 requests back to back at a single RPC provider.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(200);

/// Calls `eth_chainId` once and returns `(latency_ms, chain_id)`. This is
/// the one JSON-RPC method every EVM chain in the registry supports, is
/// side-effect-free, and — unlike a bare HTTP reachability check — actually
/// confirms the endpoint is serving the chain it claims to.
async fn call_eth_chain_id(url: &str, timeout: Duration) -> Result<(u64, u64), String> {
    let start = Instant::now();
    let result = jsonrpc::call(url, "eth_chainId", json!([]), timeout).await?;
    let latency_ms = start.elapsed().as_millis() as u64;

    let hex = result
        .as_str()
        .ok_or_else(|| format!("expected a hex string, got {result}"))?;
    let chain_id = u64::from_str_radix(hex.trim_start_matches("0x"), 16)
        .map_err(|e| format!("unparseable chainId '{hex}': {e}"))?;

    Ok((latency_ms, chain_id))
}

async fn check(url: &str, expected_chain_id: u64, timeout: Duration) -> RpcCheckResult {
    match call_eth_chain_id(url, timeout).await {
        Ok((latency_ms, chain_id_actual)) => RpcCheckResult {
            url: url.to_string(),
            chain_id_expected: expected_chain_id,
            chain_id_actual: Some(chain_id_actual),
            chain_id_matches: chain_id_actual == expected_chain_id,
            reachable: true,
            latency_ms: Some(latency_ms),
            error: None,
        },
        Err(error) => RpcCheckResult {
            url: url.to_string(),
            chain_id_expected: expected_chain_id,
            chain_id_actual: None,
            chain_id_matches: false,
            reachable: false,
            latency_ms: None,
            error: Some(error),
        },
    }
}

/// Checks every `(url, expected_chain_id)` target concurrently, bounded by
/// `concurrency`. Unlike faucets, RPC nodes are built to serve frequent
/// traffic, so — deliberately, unlike `health::check_all` — there's no
/// additional per-host throttle here.
pub async fn check_all(
    targets: &[(String, u64)],
    timeout: Duration,
    concurrency: usize,
) -> Vec<RpcCheckResult> {
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut set = tokio::task::JoinSet::new();

    for (url, chain_id) in targets.iter().cloned() {
        let semaphore = semaphore.clone();
        set.spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .expect("semaphore is never closed");
            check(&url, chain_id, timeout).await
        });
    }

    let mut results = Vec::with_capacity(targets.len());
    while let Some(joined) = set.join_next().await {
        if let Ok(status) = joined {
            results.push(status);
        }
    }
    results
}

/// Samples `eth_chainId` latency `samples` times against `url`, spaced by
/// `SAMPLE_INTERVAL`, and reports min/avg/max over the successful calls.
pub async fn benchmark(url: &str, samples: usize, timeout: Duration) -> RpcLatencyStats {
    let mut latencies = Vec::with_capacity(samples);
    let mut failures = 0usize;

    for i in 0..samples {
        match call_eth_chain_id(url, timeout).await {
            Ok((latency_ms, _)) => latencies.push(latency_ms),
            Err(_) => failures += 1,
        }
        if i + 1 < samples {
            tokio::time::sleep(SAMPLE_INTERVAL).await;
        }
    }

    RpcLatencyStats {
        url: url.to_string(),
        samples,
        failures,
        min_ms: latencies.iter().min().copied(),
        max_ms: latencies.iter().max().copied(),
        avg_ms: (!latencies.is_empty())
            .then(|| latencies.iter().sum::<u64>() / latencies.len() as u64),
    }
}
