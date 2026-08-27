use std::time::Duration;

use serde_json::json;

use crate::jsonrpc;

/// Reads an address's native-token balance in wei via `eth_getBalance`.
/// Read-only, by design — see ARCHITECTURE.md §15 on why ChainCTL never
/// touches private keys.
pub async fn get_balance(url: &str, address: &str, timeout: Duration) -> Result<u128, String> {
    let result = jsonrpc::call(url, "eth_getBalance", json!([address, "latest"]), timeout).await?;
    jsonrpc::parse_hex_u128(&result)
}
