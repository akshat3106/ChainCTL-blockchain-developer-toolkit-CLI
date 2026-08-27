use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chainctl_core::{Faucet, HealthState, HealthStatus};
use chrono::Utc;
use reqwest::Method;
use tokio::sync::{Mutex, Semaphore};

use crate::http::USER_AGENT;
use crate::tls;

/// Minimum time between two requests to the *same host*, enforced
/// independently of `concurrency` — protects a host that happens to serve
/// several faucets (e.g. two faucets both on alchemy.com) from being hit by
/// every worker in the pool simultaneously (ARCHITECTURE.md §9).
const MIN_HOST_INTERVAL: Duration = Duration::from_millis(2000);

/// Probes every faucet in `faucets` concurrently, bounded to at most
/// `concurrency` requests in flight at once, with the per-host throttle
/// above layered on top. Order of the returned `Vec` is not guaranteed to
/// match `faucets` — match on `HealthStatus::faucet_id`.
pub async fn check_all(faucets: &[Faucet], timeout: Duration, concurrency: usize) -> Vec<HealthStatus> {
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let host_throttle: Arc<Mutex<HashMap<String, Instant>>> = Arc::new(Mutex::new(HashMap::new()));

    let mut set = tokio::task::JoinSet::new();
    for faucet in faucets.iter().cloned() {
        let semaphore = semaphore.clone();
        let host_throttle = host_throttle.clone();
        set.spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .expect("semaphore is never closed");
            throttle_host(&host_throttle, &faucet.health_check.endpoint).await;
            check_one(&faucet, timeout).await
        });
    }

    let mut results = Vec::with_capacity(faucets.len());
    while let Some(joined) = set.join_next().await {
        if let Ok(status) = joined {
            results.push(status);
        }
    }
    results
}

async fn throttle_host(throttle: &Mutex<HashMap<String, Instant>>, url: &str) {
    let Some(host) = extract_host(url) else { return };

    let wait = {
        let mut reserved = throttle.lock().await;
        let now = Instant::now();
        let wait = reserved
            .get(&host)
            .and_then(|last| MIN_HOST_INTERVAL.checked_sub(now.saturating_duration_since(*last)));
        // Reserve this host's next slot up front (not after sleeping) so two
        // tasks racing for the same host don't both read the same "last"
        // value and both decide they're free to go immediately.
        reserved.insert(host, now + wait.unwrap_or(Duration::ZERO));
        wait
    };

    if let Some(wait) = wait {
        tokio::time::sleep(wait).await;
    }
}

fn extract_host(url: &str) -> Option<String> {
    let without_scheme = url.split("://").nth(1)?;
    let host = without_scheme.split(['/', ':']).next()?;
    (!host.is_empty()).then(|| host.to_lowercase())
}

async fn check_one(faucet: &Faucet, timeout: Duration) -> HealthStatus {
    if faucet.exclude_from_health_check {
        return unknown(faucet, "opted out of health checks");
    }

    let client = match reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(USER_AGENT)
        .build()
    {
        Ok(c) => c,
        Err(e) => return offline(faucet, e.to_string()),
    };

    let method = if faucet.health_check.method.eq_ignore_ascii_case("GET") {
        Method::GET
    } else {
        Method::HEAD
    };
    let is_https = faucet.health_check.endpoint.starts_with("https://");

    let start = Instant::now();
    match client.request(method, &faucet.health_check.endpoint).send().await {
        Ok(response) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let http_status = response.status().as_u16();
            let status = if faucet.health_check.expected_status.contains(&http_status) {
                HealthState::Online
            } else {
                HealthState::Degraded
            };

            let ssl_expires_at = if is_https {
                match extract_host(&faucet.health_check.endpoint) {
                    Some(host) => tls::certificate_expiry(&host, timeout).await,
                    None => None,
                }
            } else {
                None
            };

            HealthStatus {
                faucet_id: faucet.id.clone(),
                checked_at: Utc::now(),
                status,
                http_status: Some(http_status),
                latency_ms: Some(latency_ms),
                ssl_valid: Some(is_https),
                ssl_expires_at,
                error: None,
            }
        }
        Err(e) => offline(faucet, e.to_string()),
    }
}

fn offline(faucet: &Faucet, error: String) -> HealthStatus {
    HealthStatus {
        faucet_id: faucet.id.clone(),
        checked_at: Utc::now(),
        status: HealthState::Offline,
        http_status: None,
        latency_ms: None,
        ssl_valid: None,
        ssl_expires_at: None,
        error: Some(error),
    }
}

fn unknown(faucet: &Faucet, reason: &str) -> HealthStatus {
    HealthStatus {
        faucet_id: faucet.id.clone(),
        checked_at: Utc::now(),
        status: HealthState::Unknown,
        http_status: None,
        latency_ms: None,
        ssl_valid: None,
        ssl_expires_at: None,
        error: Some(reason.to_string()),
    }
}
