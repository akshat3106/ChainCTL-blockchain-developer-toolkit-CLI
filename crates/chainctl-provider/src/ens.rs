//! ENS name resolution — this is the one module in ChainCTL that talks to
//! Ethereum *mainnet* regardless of which testnet the rest of the tool is
//! pointed at, since that's the only network ENS is deployed on. Uses the
//! registry's `ens.rpcUrl` config (a public mainnet RPC by default) rather
//! than anything from the testnet-focused chain registry.

use std::time::Duration;

use serde_json::json;

use crate::{abi, jsonrpc};

/// ENS Registry with Fallback — verified against Etherscan, Bloxy, and
/// Bitquery (see chainctl commit history / PR discussion for the lookup)
/// rather than typed from memory, since one wrong hex digit here would
/// silently break every resolution.
const ENS_REGISTRY: &str = "0x00000000000c2e074ec69a0dfb2997ba6c7d2e1e";

fn selector(sig: &str) -> [u8; 4] {
    let hash = abi::keccak256(sig.as_bytes());
    [hash[0], hash[1], hash[2], hash[3]]
}

/// The standard ENS namehash algorithm (EIP-137): recursively hash each
/// label from TLD inward, so `namehash("vitalik.eth")` =
/// `keccak256(namehash("eth") ++ keccak256("vitalik"))`.
pub fn namehash(name: &str) -> [u8; 32] {
    let mut node = [0u8; 32];
    if name.is_empty() {
        return node;
    }
    for label in name.split('.').collect::<Vec<_>>().iter().rev() {
        let label_hash = abi::keccak256(label.as_bytes());
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&node);
        buf[32..].copy_from_slice(&label_hash);
        node = abi::keccak256(&buf);
    }
    node
}

async fn eth_call(url: &str, to: &str, calldata: &[u8], timeout: Duration) -> Result<Vec<u8>, String> {
    let call_obj = json!({ "to": to, "data": abi::encode_hex(calldata) });
    let result = jsonrpc::call(url, "eth_call", json!([call_obj, "latest"]), timeout).await?;
    let hex = result
        .as_str()
        .ok_or_else(|| format!("expected a hex string result, got {result}"))?;
    abi::decode_hex(hex)
}

async fn resolver_for(url: &str, node: &[u8; 32], timeout: Duration) -> Result<Option<String>, String> {
    let mut calldata = selector("resolver(bytes32)").to_vec();
    calldata.extend_from_slice(node);
    let result = eth_call(url, ENS_REGISTRY, &calldata, timeout).await?;
    if result.len() < 32 {
        return Err("unexpected resolver() response length".to_string());
    }
    let resolver_bytes = &result[12..32];
    if resolver_bytes.iter().all(|b| *b == 0) {
        return Ok(None);
    }
    Ok(Some(abi::encode_hex(resolver_bytes)))
}

/// Forward resolution: `name.eth` -> `0xabc...`. `Ok(None)` means the name
/// has no resolver set (unregistered, or registered with no address record)
/// — a legitimate "nothing to resolve" result, not an error.
pub async fn resolve(url: &str, name: &str, timeout: Duration) -> Result<Option<String>, String> {
    let node = namehash(name);
    let Some(resolver) = resolver_for(url, &node, timeout).await? else {
        return Ok(None);
    };

    let mut calldata = selector("addr(bytes32)").to_vec();
    calldata.extend_from_slice(&node);
    let result = eth_call(url, &resolver, &calldata, timeout).await?;
    if result.len() < 32 {
        return Err("unexpected addr() response length".to_string());
    }
    let addr_bytes = &result[12..32];
    if addr_bytes.iter().all(|b| *b == 0) {
        return Ok(None);
    }
    let mut addr20 = [0u8; 20];
    addr20.copy_from_slice(addr_bytes);
    Ok(Some(abi::checksum_address(&addr20)))
}

/// Reverse resolution: `0xabc...` -> primary ENS name, via the standard
/// `<address>.addr.reverse` namehash convention.
pub async fn reverse(url: &str, address: &str, timeout: Duration) -> Result<Option<String>, String> {
    let addr_bytes = abi::decode_hex(address)?;
    if addr_bytes.len() != 20 {
        return Err(format!("'{address}' is not a 20-byte address"));
    }
    let addr_hex = abi::encode_hex(&addr_bytes)[2..].to_string();
    let node = namehash(&format!("{addr_hex}.addr.reverse"));

    let Some(resolver) = resolver_for(url, &node, timeout).await? else {
        return Ok(None);
    };

    let mut calldata = selector("name(bytes32)").to_vec();
    calldata.extend_from_slice(&node);
    let result = eth_call(url, &resolver, &calldata, timeout).await?;
    let name = abi::decode_params(&["string".to_string()], &result)?
        .into_iter()
        .next()
        .unwrap_or_default();

    Ok((!name.is_empty()).then_some(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namehash_of_empty_is_zero() {
        assert_eq!(namehash(""), [0u8; 32]);
    }

    #[test]
    fn namehash_is_deterministic_and_label_order_sensitive() {
        let a = namehash("vitalik.eth");
        let b = namehash("vitalik.eth");
        assert_eq!(a, b);
        assert_ne!(a, namehash("eth.vitalik"));
    }
}
