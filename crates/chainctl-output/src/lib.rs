mod json;
mod table;
mod theme;

use chainctl_core::{Chain, ChainctlError, Faucet, HealthStatus, RpcCheckResult, RpcLatencyStats, Score};
pub use theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Plain,
}

/// One row of `chainctl faucet status` output — deliberately owned/flattened
/// rather than borrowing `Chain`/`Faucet` so callers can assemble rows from
/// multiple chains without fighting lifetimes.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthRow {
    pub chain: String,
    pub faucet_name: String,
    pub url: String,
    pub status: HealthStatus,
}

pub fn render_chains(chains: &[Chain], format: OutputFormat, theme: &Theme) -> String {
    match format {
        OutputFormat::Json => json::chains(chains),
        OutputFormat::Table | OutputFormat::Plain => table::chains(chains, theme),
    }
}

pub fn render_faucets(chain: &Chain, faucets: &[&Faucet], format: OutputFormat, theme: &Theme) -> String {
    match format {
        OutputFormat::Json => json::faucets(chain, faucets),
        OutputFormat::Table | OutputFormat::Plain => table::faucets(chain, faucets, theme),
    }
}

pub fn render_faucet_info(chain: &Chain, faucet: &Faucet, format: OutputFormat, theme: &Theme) -> String {
    match format {
        OutputFormat::Json => json::faucet_info(chain, faucet),
        OutputFormat::Table | OutputFormat::Plain => table::faucet_info(chain, faucet, theme),
    }
}

pub fn render_recommendation(
    chain: &Chain,
    faucet: &Faucet,
    score: &Score,
    explain: bool,
    format: OutputFormat,
    theme: &Theme,
) -> String {
    match format {
        OutputFormat::Json => json::recommendation(chain, faucet, score),
        OutputFormat::Table | OutputFormat::Plain => {
            table::recommendation(chain, faucet, score, explain, theme)
        }
    }
}

pub fn render_health(rows: &[HealthRow], format: OutputFormat, theme: &Theme) -> String {
    match format {
        OutputFormat::Json => json::health(rows),
        OutputFormat::Table | OutputFormat::Plain => table::health(rows, theme),
    }
}

/// One row of `chainctl rpc list` — one per (chain, RPC URL) pair.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcListRow {
    pub chain: String,
    pub chain_id: u64,
    pub url: String,
}

/// One row of `chainctl rpc test` — one per (chain, RPC URL) pair, paired
/// with the probe result.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcTestRow {
    pub chain: String,
    pub result: RpcCheckResult,
}

pub fn render_rpc_list(rows: &[RpcListRow], format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => json::rpc_list(rows),
        OutputFormat::Table | OutputFormat::Plain => table::rpc_list(rows),
    }
}

pub fn render_rpc_test(rows: &[RpcTestRow], format: OutputFormat, theme: &Theme) -> String {
    match format {
        OutputFormat::Json => json::rpc_test(rows),
        OutputFormat::Table | OutputFormat::Plain => table::rpc_test(rows, theme),
    }
}

pub fn render_rpc_latency(
    chain: &Chain,
    stats: &[RpcLatencyStats],
    format: OutputFormat,
    theme: &Theme,
) -> String {
    match format {
        OutputFormat::Json => json::rpc_latency(chain, stats),
        OutputFormat::Table | OutputFormat::Plain => table::rpc_latency(chain, stats, theme),
    }
}

pub fn render_error(err: &ChainctlError, format: OutputFormat, theme: &Theme) -> String {
    match format {
        OutputFormat::Json => json::error(err),
        OutputFormat::Table | OutputFormat::Plain => table::error(err, theme),
    }
}
