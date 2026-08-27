//! A deliberately small hand-rolled ABI encoder/decoder — cast-style
//! (`"transfer(address,uint256)"`) rather than Etherscan-ABI-JSON-style, so
//! `abi`/`contract`/`ens` need no API key and no full alloy/ethers
//! dependency. Supports the types that cover the overwhelming majority of
//! real "call a getter" usage: `address`, `bool`, `string`, `bytes`,
//! `bytesN` (1-32), `uintN`/`intN` (8-256, non-negative `int` only).
//! Arrays, tuples, and negative `int` values are explicitly unsupported —
//! callers get a clear error, not a silently wrong encoding.

use sha3::{Digest, Keccak256};

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if !s.len().is_multiple_of(2) {
        return Err(format!("'{s}' is not valid hex (odd length)"));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

pub fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(2 + bytes.len() * 2);
    s.push_str("0x");
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// EIP-55 mixed-case checksum for a 20-byte address.
pub fn checksum_address(bytes: &[u8; 20]) -> String {
    let lower = encode_hex(bytes)[2..].to_string();
    let hash = keccak256(lower.as_bytes());
    let mut out = String::with_capacity(42);
    out.push_str("0x");
    for (i, c) in lower.chars().enumerate() {
        if c.is_ascii_digit() {
            out.push(c);
            continue;
        }
        let hash_byte = hash[i / 2];
        let nibble = if i % 2 == 0 { hash_byte >> 4 } else { hash_byte & 0x0f };
        out.push(if nibble >= 8 { c.to_ascii_uppercase() } else { c });
    }
    out
}

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String,
    pub inputs: Vec<String>,
    /// Present for `contract read`'s `"name(in)(out)"` form; empty for
    /// `abi encode`/`abi decode`'s plain `"name(in)"` form.
    pub outputs: Vec<String>,
}

impl FunctionSig {
    pub fn selector(&self) -> [u8; 4] {
        let sig = format!("{}({})", self.name, self.inputs.join(","));
        let hash = keccak256(sig.as_bytes());
        [hash[0], hash[1], hash[2], hash[3]]
    }
}

/// Parses `"name(type,type)"` or `"name(type,type)(type,type)"`.
pub fn parse_signature(sig: &str) -> Result<FunctionSig, String> {
    let sig = sig.trim();
    let first_paren = sig.find('(').ok_or("signature must contain '(' — expected e.g. \"balanceOf(address)\"")?;
    let name = sig[..first_paren].trim().to_string();
    if name.is_empty() {
        return Err("signature is missing a function name".to_string());
    }

    let (inputs_str, remainder) = split_paren_group(&sig[first_paren..])?;
    let inputs = split_types(inputs_str);
    for t in &inputs {
        validate_type(t)?;
    }

    let outputs = if remainder.trim().is_empty() {
        Vec::new()
    } else {
        let (outputs_str, _) = split_paren_group(remainder.trim())?;
        let outputs = split_types(outputs_str);
        for t in &outputs {
            validate_type(t)?;
        }
        outputs
    };

    Ok(FunctionSig { name, inputs, outputs })
}

fn split_paren_group(s: &str) -> Result<(&str, &str), String> {
    if !s.starts_with('(') {
        return Err(format!("expected '(' at start of '{s}'"));
    }
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((&s[1..i], &s[i + 1..]));
                }
            }
            _ => {}
        }
    }
    Err(format!("unbalanced parentheses in '{s}'"))
}

fn split_types(s: &str) -> Vec<String> {
    s.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

fn fixed_bytes_len(t: &str) -> Option<usize> {
    let n: usize = t.strip_prefix("bytes")?.parse().ok()?;
    (1..=32).contains(&n).then_some(n)
}

fn is_dynamic_type(t: &str) -> bool {
    t == "string" || t == "bytes"
}

fn validate_type(t: &str) -> Result<(), String> {
    let ok = matches!(t, "address" | "bool" | "string" | "bytes")
        || fixed_bytes_len(t).is_some()
        || is_uint_or_int(t);
    if ok {
        Ok(())
    } else {
        Err(format!(
            "unsupported type '{t}' (supported: address, bool, string, bytes, bytes1-32, uint8-256, int8-256; arrays/tuples aren't)"
        ))
    }
}

fn is_uint_or_int(t: &str) -> bool {
    let digits = t.strip_prefix("uint").or_else(|| t.strip_prefix("int"));
    match digits {
        Some(d) => d.parse::<u32>().map(|n| n % 8 == 0 && (8..=256).contains(&n)).unwrap_or(false),
        None => false,
    }
}

/// Encodes `values` (as user-supplied strings) against `types` into a single
/// ABI parameter blob (head words + dynamic tail), per the standard
/// Solidity ABI layout.
pub fn encode_params(types: &[String], values: &[String]) -> Result<Vec<u8>, String> {
    if types.len() != values.len() {
        return Err(format!("expected {} argument(s), got {}", types.len(), values.len()));
    }

    let mut heads: Vec<[u8; 32]> = Vec::with_capacity(types.len());
    let mut tails: Vec<Vec<u8>> = Vec::with_capacity(types.len());

    for (t, v) in types.iter().zip(values) {
        if is_dynamic_type(t) {
            heads.push([0u8; 32]); // placeholder, offset filled in below
            tails.push(encode_dynamic(t, v)?);
        } else {
            heads.push(encode_static(t, v)?);
            tails.push(Vec::new());
        }
    }

    let head_size = 32 * types.len();
    let mut out = Vec::with_capacity(head_size);
    let mut tail_data = Vec::new();
    let mut offset = head_size;

    for (i, t) in types.iter().enumerate() {
        if is_dynamic_type(t) {
            out.extend_from_slice(&u256_be(offset as u128));
            offset += tails[i].len();
            tail_data.extend_from_slice(&tails[i]);
        } else {
            out.extend_from_slice(&heads[i]);
        }
    }
    out.extend_from_slice(&tail_data);
    Ok(out)
}

fn encode_static(t: &str, v: &str) -> Result<[u8; 32], String> {
    if t == "address" {
        let bytes = decode_hex(v)?;
        if bytes.len() != 20 {
            return Err(format!("'{v}' is not a 20-byte address"));
        }
        let mut word = [0u8; 32];
        word[12..].copy_from_slice(&bytes);
        return Ok(word);
    }
    if t == "bool" {
        let mut word = [0u8; 32];
        word[31] = if matches!(v, "true" | "1") { 1 } else { 0 };
        return Ok(word);
    }
    if let Some(len) = fixed_bytes_len(t) {
        let bytes = decode_hex(v)?;
        if bytes.len() != len {
            return Err(format!("'{v}' is not {len} byte(s) for type {t}"));
        }
        let mut word = [0u8; 32];
        word[..len].copy_from_slice(&bytes);
        return Ok(word);
    }
    if is_uint_or_int(t) {
        if let Some(stripped) = v.strip_prefix('-') {
            let _ = stripped;
            return Err(format!("negative values aren't supported yet (got '{v}' for {t})"));
        }
        let value: u128 = if let Some(hex) = v.strip_prefix("0x") {
            u128::from_str_radix(hex, 16).map_err(|e| e.to_string())?
        } else {
            v.parse().map_err(|_| format!("'{v}' is not a valid non-negative integer"))?
        };
        return Ok(u256_be(value));
    }
    Err(format!("'{t}' is not a static/supported type"))
}

fn encode_dynamic(t: &str, v: &str) -> Result<Vec<u8>, String> {
    let bytes = match t {
        "string" => v.as_bytes().to_vec(),
        "bytes" => decode_hex(v)?,
        other => return Err(format!("'{other}' is not a dynamic type")),
    };
    let mut out = Vec::new();
    out.extend_from_slice(&u256_be(bytes.len() as u128));
    out.extend_from_slice(&bytes);
    let padding = (32 - (bytes.len() % 32)) % 32;
    out.extend(std::iter::repeat_n(0u8, padding));
    Ok(out)
}

fn u256_be(value: u128) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[16..].copy_from_slice(&value.to_be_bytes());
    word
}

/// Decodes an ABI parameter blob against `types`, returning one
/// human-readable string per value.
pub fn decode_params(types: &[String], data: &[u8]) -> Result<Vec<String>, String> {
    let mut out = Vec::with_capacity(types.len());
    for (i, t) in types.iter().enumerate() {
        let head_start = i * 32;
        let head = data
            .get(head_start..head_start + 32)
            .ok_or_else(|| format!("data too short to contain argument {i} ('{t}')"))?;

        if is_dynamic_type(t) {
            let offset = usize_from_word(head)?;
            let len_word = data
                .get(offset..offset + 32)
                .ok_or_else(|| format!("bad dynamic offset for argument {i}"))?;
            let len = usize_from_word(len_word)?;
            let bytes = data
                .get(offset + 32..offset + 32 + len)
                .ok_or_else(|| format!("dynamic data for argument {i} runs past the end"))?;
            out.push(match t.as_str() {
                "string" => String::from_utf8_lossy(bytes).to_string(),
                _ => encode_hex(bytes),
            });
        } else {
            out.push(decode_static(t, head)?);
        }
    }
    Ok(out)
}

fn decode_static(t: &str, word: &[u8]) -> Result<String, String> {
    if t == "address" {
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&word[12..32]);
        return Ok(checksum_address(&addr));
    }
    if t == "bool" {
        return Ok(if word[31] != 0 { "true" } else { "false" }.to_string());
    }
    if let Some(len) = fixed_bytes_len(t) {
        return Ok(encode_hex(&word[..len]));
    }
    if t.starts_with("uint") {
        return decode_uint(word).map_err(|e| format!("{t}: {e}"));
    }
    if t.starts_with("int") {
        return decode_int(word).map_err(|e| format!("{t}: {e}"));
    }
    Err(format!("'{t}' is not a static/supported type"))
}

fn decode_uint(word: &[u8]) -> Result<String, String> {
    if word[..16].iter().any(|b| *b != 0) {
        // Doesn't fit u128 — show raw hex rather than guess/crash.
        return Ok(encode_hex(word));
    }
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&word[16..32]);
    Ok(u128::from_be_bytes(buf).to_string())
}

fn decode_int(word: &[u8]) -> Result<String, String> {
    let negative = word[0] & 0x80 != 0;
    let sign_bytes: &[u8] = if negative { &[0xff; 16] } else { &[0x00; 16] };
    if word[..16] != *sign_bytes {
        return Ok(encode_hex(word)); // out of i128 range — show raw hex
    }
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&word[16..32]);
    let unsigned = u128::from_be_bytes(buf);
    if negative {
        let magnitude = (!unsigned).wrapping_add(1);
        Ok(format!("-{magnitude}"))
    } else {
        Ok(unsigned.to_string())
    }
}

fn usize_from_word(word: &[u8]) -> Result<usize, String> {
    if word[..16].iter().any(|b| *b != 0) {
        return Err("offset/length too large".to_string());
    }
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&word[16..32]);
    usize::try_from(u128::from_be_bytes(buf)).map_err(|e| e.to_string())
}
