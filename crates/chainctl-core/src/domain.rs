use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub version: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    pub chains: Vec<Chain>,
}

impl Registry {
    /// Looks up a chain by its id or any of its aliases (case-insensitive).
    pub fn find_chain(&self, query: &str) -> Option<&Chain> {
        let query = query.to_lowercase();
        self.chains.iter().find(|c| {
            c.id.to_lowercase() == query
                || c.aliases.iter().any(|a| a.to_lowercase() == query)
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub id: String,
    pub name: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    pub symbol: String,
    pub network: NetworkKind,
    #[serde(rename = "parentChain")]
    pub parent_chain: String,
    #[serde(rename = "explorerUrl")]
    pub explorer_url: String,
    #[serde(rename = "rpcUrls")]
    pub rpc_urls: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub faucets: Vec<Faucet>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkKind {
    Testnet,
    Mainnet,
    Devnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faucet {
    pub id: String,
    pub name: String,
    pub url: String,
    pub source: FaucetSource,
    pub provider: String,
    pub requirements: Requirements,
    pub cooldown: Cooldown,
    #[serde(rename = "dailyLimit")]
    pub daily_limit: Option<Amount>,
    #[serde(rename = "amountPerClaim")]
    pub amount_per_claim: Amount,
    pub priority: u32,
    #[serde(rename = "excludeFromHealthCheck", default)]
    pub exclude_from_health_check: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "healthCheck")]
    pub health_check: HealthCheckConfig,
    pub metadata: FaucetMetadata,
    /// Crowdsourced 0-100 rating, curated via registry PRs (Phase 2 — no
    /// voting infrastructure yet). Absent means "no rating yet," which the
    /// scoring engine treats as neutral (50), not zero.
    #[serde(rename = "communityRating", default)]
    pub community_rating: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FaucetSource {
    Official,
    Partner,
    Community,
}

impl FaucetSource {
    /// Base 0-100 contribution used by the official-priority scoring factor.
    pub fn base_score(&self) -> f64 {
        match self {
            FaucetSource::Official => 100.0,
            FaucetSource::Partner => 60.0,
            FaucetSource::Community => 30.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirements {
    #[serde(rename = "githubAuth", default)]
    pub github_auth: bool,
    #[serde(rename = "discordAuth", default)]
    pub discord_auth: bool,
    #[serde(default)]
    pub captcha: bool,
    #[serde(rename = "walletConnect", default)]
    pub wallet_connect: bool,
    #[serde(rename = "minMainnetBalance")]
    pub min_mainnet_balance: Option<MinBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinBalance {
    pub chain: String,
    pub amount: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cooldown {
    pub amount: u32,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    pub amount: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub method: String,
    pub endpoint: String,
    #[serde(rename = "expectedStatus")]
    pub expected_status: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetMetadata {
    #[serde(rename = "addedAt")]
    pub added_at: DateTime<Utc>,
    #[serde(rename = "lastVerifiedAt")]
    pub last_verified_at: DateTime<Utc>,
    pub maintainer: String,
}

/// Result of an active reachability probe against a faucet's `healthCheck.endpoint`.
/// Exhaustive by design (no `Unknown` catch-all beyond the explicit variant below) so
/// every consumer (scoring, rendering) is forced to handle new states at compile time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub faucet_id: String,
    pub checked_at: DateTime<Utc>,
    pub status: HealthState,
    pub http_status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub ssl_valid: Option<bool>,
    pub ssl_expires_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    Online,
    Offline,
    Degraded,
    Maintenance,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub faucet_id: String,
    pub total: f64,
    pub breakdown: std::collections::HashMap<String, f64>,
}

/// Result of a single `eth_chainId` probe against an RPC endpoint — checks
/// both reachability and that the endpoint actually serves the chain the
/// registry says it does (a wrong/stale RPC URL is a real, common failure
/// mode this catches that a plain HTTP reachability check would miss).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCheckResult {
    pub url: String,
    #[serde(rename = "chainIdExpected")]
    pub chain_id_expected: u64,
    #[serde(rename = "chainIdActual")]
    pub chain_id_actual: Option<u64>,
    #[serde(rename = "chainIdMatches")]
    pub chain_id_matches: bool,
    pub reachable: bool,
    #[serde(rename = "latencyMs")]
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Result of looking up a transaction by hash — combines
/// `eth_getTransactionByHash` (existence) and `eth_getTransactionReceipt`
/// (finality/success) into one status a developer actually wants to read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxStatus {
    pub hash: String,
    pub found: bool,
    pub pending: bool,
    pub success: Option<bool>,
    #[serde(rename = "blockNumber")]
    pub block_number: Option<u64>,
    #[serde(rename = "gasUsed")]
    pub gas_used: Option<u64>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Latency benchmark over repeated `eth_chainId` calls to one RPC endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcLatencyStats {
    pub url: String,
    pub samples: usize,
    pub failures: usize,
    #[serde(rename = "minMs")]
    pub min_ms: Option<u64>,
    #[serde(rename = "avgMs")]
    pub avg_ms: Option<u64>,
    #[serde(rename = "maxMs")]
    pub max_ms: Option<u64>,
}
