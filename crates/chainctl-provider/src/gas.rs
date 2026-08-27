use std::time::Duration;

use serde_json::{json, Map, Value};

use crate::jsonrpc;

pub async fn gas_price(url: &str, timeout: Duration) -> Result<u128, String> {
    let result = jsonrpc::call(url, "eth_gasPrice", json!([]), timeout).await?;
    jsonrpc::parse_hex_u128(&result)
}

#[derive(Debug, Default)]
pub struct EstimateRequest {
    pub from: Option<String>,
    pub to: Option<String>,
    /// Hex-quantity string (e.g. `"0xde0b6b3a7640000"`), already validated by the caller.
    pub value_hex: Option<String>,
    /// Hex calldata (e.g. `"0xa9059cbb..."`), already validated by the caller.
    pub data_hex: Option<String>,
}

pub async fn estimate_gas(url: &str, req: &EstimateRequest, timeout: Duration) -> Result<u128, String> {
    let mut tx = Map::new();
    if let Some(from) = &req.from {
        tx.insert("from".to_string(), json!(from));
    }
    if let Some(to) = &req.to {
        tx.insert("to".to_string(), json!(to));
    }
    if let Some(value) = &req.value_hex {
        tx.insert("value".to_string(), json!(value));
    }
    if let Some(data) = &req.data_hex {
        tx.insert("data".to_string(), json!(data));
    }

    let result = jsonrpc::call(url, "eth_estimateGas", json!([Value::Object(tx)]), timeout).await?;
    jsonrpc::parse_hex_u128(&result)
}
