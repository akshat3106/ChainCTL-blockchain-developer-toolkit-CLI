use std::time::Duration;

use serde_json::{json, Value};

use crate::http::USER_AGENT;

/// Calls `method` with `params` against a JSON-RPC endpoint and returns the
/// `result` field. Used by `rpc`, `gas`, `wallet`, `tx`, and `ens` — every
/// EVM read in ChainCTL goes through this one function.
pub async fn call(url: &str, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});

    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let value: Value = response.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = value.get("error") {
        return Err(format!("RPC error: {err}"));
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| "response had no 'result' field".to_string())
}

/// Parses a `"0x..."` quantity string into `u128` — covers everything from
/// chain IDs to wei balances without needing a bigint type for values this
/// tool only ever displays, never does arithmetic on beyond formatting.
pub fn parse_hex_u128(value: &Value) -> Result<u128, String> {
    let s = value
        .as_str()
        .ok_or_else(|| format!("expected a hex string, got {value}"))?;
    u128::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| format!("unparseable quantity '{s}': {e}"))
}
