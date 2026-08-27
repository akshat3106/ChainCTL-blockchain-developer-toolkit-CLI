use std::time::Duration;

use serde_json::json;

use crate::{abi, jsonrpc};

/// Encodes `sig(args...)`, calls it via `eth_call`, and decodes the result
/// against `sig`'s declared output types.
pub async fn call_read(
    url: &str,
    address: &str,
    sig: &abi::FunctionSig,
    args: &[String],
    timeout: Duration,
) -> Result<Vec<String>, String> {
    let params = abi::encode_params(&sig.inputs, args)?;
    let mut calldata = sig.selector().to_vec();
    calldata.extend_from_slice(&params);

    let call_obj = json!({ "to": address, "data": abi::encode_hex(&calldata) });
    let result = jsonrpc::call(url, "eth_call", json!([call_obj, "latest"]), timeout).await?;
    let hex = result
        .as_str()
        .ok_or_else(|| format!("expected a hex string result, got {result}"))?;
    let bytes = abi::decode_hex(hex)?;

    if sig.outputs.is_empty() {
        return Ok(Vec::new());
    }
    abi::decode_params(&sig.outputs, &bytes)
}
