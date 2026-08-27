use std::time::Duration;

use chainctl_core::TxStatus;
use serde_json::json;

use crate::jsonrpc;

pub async fn status(url: &str, hash: &str, timeout: Duration) -> Result<TxStatus, String> {
    let tx = jsonrpc::call(url, "eth_getTransactionByHash", json!([hash]), timeout).await?;
    if tx.is_null() {
        return Ok(TxStatus {
            hash: hash.to_string(),
            found: false,
            pending: false,
            success: None,
            block_number: None,
            gas_used: None,
            from: None,
            to: None,
        });
    }

    let from = tx.get("from").and_then(|v| v.as_str()).map(str::to_string);
    let to = tx.get("to").and_then(|v| v.as_str()).map(str::to_string);

    let receipt = jsonrpc::call(url, "eth_getTransactionReceipt", json!([hash]), timeout).await?;
    if receipt.is_null() {
        return Ok(TxStatus {
            hash: hash.to_string(),
            found: true,
            pending: true,
            success: None,
            block_number: None,
            gas_used: None,
            from,
            to,
        });
    }

    let success = receipt
        .get("status")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches("0x") == "1");
    let block_number = receipt
        .get("blockNumber")
        .and_then(|v| v.as_str())
        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());
    let gas_used = receipt
        .get("gasUsed")
        .and_then(|v| v.as_str())
        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());

    Ok(TxStatus {
        hash: hash.to_string(),
        found: true,
        pending: false,
        success,
        block_number,
        gas_used,
        from,
        to,
    })
}
