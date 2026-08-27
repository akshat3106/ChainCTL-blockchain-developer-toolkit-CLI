use chainctl_core::{Chain, ChainctlError};
use chainctl_output::{RpcListRow, RpcTestRow};
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct RpcCmd {
    #[command(subcommand)]
    action: RpcAction,
}

#[derive(Subcommand)]
enum RpcAction {
    /// List known RPC endpoints for a chain (or all chains).
    List { chain: Option<String> },
    /// Probe RPC endpoints: reachability, latency, and chain-id correctness.
    Test { chain: Option<String> },
    /// Benchmark latency for one chain's RPC endpoint(s) over repeated calls.
    Latency {
        chain: String,
        /// Number of eth_chainId calls to sample per endpoint.
        #[arg(long, default_value_t = 5)]
        samples: usize,
    },
}

pub async fn run(ctx: &Context, cmd: RpcCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        RpcAction::List { chain } => list(ctx, chain.as_deref()),
        RpcAction::Test { chain } => test(ctx, chain.as_deref()).await,
        RpcAction::Latency { chain, samples } => latency(ctx, &chain, samples).await,
    }
}

fn select_chains<'a>(
    ctx: &Context,
    registry: &'a chainctl_core::Registry,
    chain_query: Option<&str>,
) -> Result<Vec<&'a Chain>, ChainctlError> {
    match chain_query {
        Some(q) => Ok(vec![ctx.resolve_chain(registry, q)?]),
        None => Ok(registry.chains.iter().collect()),
    }
}

fn list(ctx: &Context, chain_query: Option<&str>) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chains = select_chains(ctx, &registry, chain_query)?;

    let rows: Vec<RpcListRow> = chains
        .iter()
        .flat_map(|c| {
            c.rpc_urls.iter().map(move |url| RpcListRow {
                chain: c.id.clone(),
                chain_id: c.chain_id,
                url: url.clone(),
            })
        })
        .collect();

    if rows.is_empty() {
        return Err(ChainctlError::NoRpcEndpoints(
            chain_query.unwrap_or("registry").to_string(),
        ));
    }

    println!("{}", chainctl_output::render_rpc_list(&rows, ctx.output));
    Ok(())
}

async fn test(ctx: &Context, chain_query: Option<&str>) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chains = select_chains(ctx, &registry, chain_query)?;

    // (chain id label, url, expected chain id) — flattened across every
    // selected chain so a bare `chainctl rpc test` checks the whole registry
    // in one concurrent batch rather than one chain at a time.
    let targets: Vec<(String, String, u64)> = chains
        .iter()
        .flat_map(|c| c.rpc_urls.iter().map(move |url| (c.id.clone(), url.clone(), c.chain_id)))
        .collect();

    if targets.is_empty() {
        return Err(ChainctlError::NoRpcEndpoints(
            chain_query.unwrap_or("registry").to_string(),
        ));
    }

    let (concurrency, timeout) = super::config::get_health_settings(ctx)?;
    let probe_targets: Vec<(String, u64)> = targets
        .iter()
        .map(|(_, url, chain_id)| (url.clone(), *chain_id))
        .collect();

    if !ctx.quiet && ctx.output == chainctl_output::OutputFormat::Table {
        eprintln!("Checking RPC endpoints...");
    }

    let results = chainctl_provider::rpc::check_all(&probe_targets, timeout, concurrency).await;

    let chain_by_url: std::collections::HashMap<&str, &str> = targets
        .iter()
        .map(|(chain, url, _)| (url.as_str(), chain.as_str()))
        .collect();

    let mut rows: Vec<RpcTestRow> = results
        .into_iter()
        .map(|result| RpcTestRow {
            chain: chain_by_url.get(result.url.as_str()).copied().unwrap_or("").to_string(),
            result,
        })
        .collect();
    rows.sort_by(|a, b| a.chain.cmp(&b.chain).then(a.result.url.cmp(&b.result.url)));

    println!("{}", chainctl_output::render_rpc_test(&rows, ctx.output, &ctx.theme));
    Ok(())
}

async fn latency(ctx: &Context, chain_query: &str, samples: usize) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;

    if chain.rpc_urls.is_empty() {
        return Err(ChainctlError::NoRpcEndpoints(chain.id.clone()));
    }

    let (_, timeout) = super::config::get_health_settings(ctx)?;
    let samples = samples.max(1);

    if !ctx.quiet && ctx.output == chainctl_output::OutputFormat::Table {
        eprintln!("Sampling {samples} calls per endpoint (this takes a few seconds)...");
    }

    let mut stats = Vec::with_capacity(chain.rpc_urls.len());
    for url in &chain.rpc_urls {
        stats.push(chainctl_provider::rpc::benchmark(url, samples, timeout).await);
    }

    println!(
        "{}",
        chainctl_output::render_rpc_latency(chain, &stats, ctx.output, &ctx.theme)
    );
    Ok(())
}
