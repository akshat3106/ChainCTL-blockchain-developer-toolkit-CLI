use chainctl_core::{Chain, ChainctlError, Faucet, RpcLatencyStats, Score};
use serde_json::json;

use crate::{HealthRow, RpcListRow, RpcTestRow};

pub fn chains(chains: &[Chain]) -> String {
    pretty(&json!(chains))
}

pub fn faucets(chain: &Chain, faucets: &[&Faucet]) -> String {
    pretty(&json!({ "chain": chain.id, "faucets": faucets }))
}

pub fn faucet_info(chain: &Chain, faucet: &Faucet) -> String {
    pretty(&json!({ "chain": chain.id, "faucet": faucet }))
}

pub fn recommendation(chain: &Chain, faucet: &Faucet, score: &Score) -> String {
    pretty(&json!({ "chain": chain.id, "recommended": faucet, "score": score }))
}

pub fn health(rows: &[HealthRow]) -> String {
    pretty(&json!(rows))
}

pub fn rpc_list(rows: &[RpcListRow]) -> String {
    pretty(&json!(rows))
}

pub fn rpc_test(rows: &[RpcTestRow]) -> String {
    pretty(&json!(rows))
}

pub fn rpc_latency(chain: &Chain, stats: &[RpcLatencyStats]) -> String {
    pretty(&json!({ "chain": chain.id, "latency": stats }))
}

pub fn error(err: &ChainctlError) -> String {
    pretty(&json!({
        "error": {
            "message": err.to_string(),
            "hint": err.hint(),
        }
    }))
}

fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}
