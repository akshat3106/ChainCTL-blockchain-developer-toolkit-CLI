use std::time::Duration;

use chainctl_core::ChainctlError;
use clap::{Args, Subcommand};
use serde_yaml::{Mapping, Value};

use super::Context;

const DEFAULT_CONFIG_YAML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/config.default.yaml"));

#[derive(Args)]
pub struct ConfigCmd {
    #[command(subcommand)]
    action: ConfigAction,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Print the value at a dot-path key (e.g. `registry.source`).
    Get { key: String },
    /// Set the value at a dot-path key.
    Set { key: String, value: String },
    /// Print the full resolved config.
    List,
    /// Open `config.yaml` in the default editor/application.
    Edit,
}

pub fn run(ctx: &Context, cmd: ConfigCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        ConfigAction::Get { key } => {
            let value = load_value(ctx)?;
            match get_path(&value, &key) {
                Some(v) => println!("{}", scalar_to_string(v)),
                None => println!("(not set)"),
            }
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut root = load_value(ctx)?;
            let parsed: Value = serde_yaml::from_str(&value).unwrap_or(Value::String(value.clone()));
            set_path(&mut root, &key, parsed);
            save_value(ctx, &root)?;
            println!("{key} = {value}");
            Ok(())
        }
        ConfigAction::List => {
            let value = load_value(ctx)?;
            print!(
                "{}",
                serde_yaml::to_string(&value).map_err(|e| ChainctlError::Config(e.to_string()))?
            );
            Ok(())
        }
        ConfigAction::Edit => {
            if !ctx.paths.config_file.exists() {
                save_value(ctx, &load_value(ctx)?)?;
            }
            let path = ctx.paths.config_file.to_string_lossy().to_string();
            println!("Opening {path}");
            chainctl_provider::browser::open_url(&path)
        }
    }
}

/// Used by `chainctl update` to read `registry.source` without duplicating
/// the config-loading logic.
pub(crate) fn get_str(ctx: &Context, key: &str) -> Result<Option<String>, ChainctlError> {
    let value = load_value(ctx)?;
    Ok(get_path(&value, key).and_then(|v| v.as_str().map(str::to_string)))
}

/// Reads `recommend.weights.*`, falling back to `chainctl-scoring`'s
/// defaults for any key that's absent — so an empty or partial config never
/// breaks scoring (ARCHITECTURE.md §10).
pub(crate) fn get_weights(ctx: &Context) -> Result<chainctl_scoring::Weights, ChainctlError> {
    let value = load_value(ctx)?;
    let mut weights = chainctl_scoring::Weights::default();
    let get_f64 = |key: &str| get_path(&value, key).and_then(Value::as_f64);

    if let Some(v) = get_f64("recommend.weights.official") {
        weights.official = v;
    }
    if let Some(v) = get_f64("recommend.weights.availability") {
        weights.availability = v;
    }
    if let Some(v) = get_f64("recommend.weights.latency") {
        weights.latency = v;
    }
    if let Some(v) = get_f64("recommend.weights.community") {
        weights.community = v;
    }
    if let Some(v) = get_f64("recommend.weights.recentFailures") {
        weights.recent_failures = v;
    }
    Ok(weights)
}

/// Reads `cache.ttlMinutes`, defaulting to 30 (ARCHITECTURE.md §6.3) if
/// unset or malformed.
pub(crate) fn get_cache_ttl_minutes(ctx: &Context) -> Result<u64, ChainctlError> {
    let value = load_value(ctx)?;
    Ok(get_path(&value, "cache.ttlMinutes")
        .and_then(Value::as_u64)
        .unwrap_or(30))
}

/// Reads `health.concurrency` (min 1, default 5) and `health.timeoutSeconds`
/// (min 1, default 5) for the Phase 3 concurrent health checker.
pub(crate) fn get_health_settings(ctx: &Context) -> Result<(usize, Duration), ChainctlError> {
    let value = load_value(ctx)?;
    let concurrency = get_path(&value, "health.concurrency")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .max(1) as usize;
    let timeout_secs = get_path(&value, "health.timeoutSeconds")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .max(1);
    Ok((concurrency, Duration::from_secs(timeout_secs)))
}

/// Default mirrors `configs/config.default.yaml`; used both for a fresh
/// config and as a fallback for an existing `config.yaml` written before
/// this key existed — same reasoning as every other getter in this file.
const DEFAULT_ENS_RPC_URL: &str = "https://ethereum-rpc.publicnode.com";

/// Reads `ens.rpcUrl` — ENS only exists on Ethereum mainnet, so this is
/// intentionally independent of whatever testnet the rest of ChainCTL is
/// pointed at.
pub(crate) fn get_ens_rpc_url(ctx: &Context) -> Result<String, ChainctlError> {
    let value = load_value(ctx)?;
    Ok(get_path(&value, "ens.rpcUrl")
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| DEFAULT_ENS_RPC_URL.to_string()))
}

fn load_value(ctx: &Context) -> Result<Value, ChainctlError> {
    if ctx.paths.config_file.exists() {
        let raw = std::fs::read_to_string(&ctx.paths.config_file)?;
        serde_yaml::from_str(&raw).map_err(|e| ChainctlError::Config(e.to_string()))
    } else {
        serde_yaml::from_str(DEFAULT_CONFIG_YAML).map_err(|e| ChainctlError::Config(e.to_string()))
    }
}

fn save_value(ctx: &Context, value: &Value) -> Result<(), ChainctlError> {
    let rendered = serde_yaml::to_string(value).map_err(|e| ChainctlError::Config(e.to_string()))?;
    chainctl_provider::storage::write_atomic(&ctx.paths.config_file, &rendered)
}

fn get_path<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in key.split('.') {
        current = current.as_mapping()?.get(Value::String(part.to_string()))?;
    }
    Some(current)
}

fn set_path(value: &mut Value, key: &str, new_value: Value) {
    let parts: Vec<&str> = key.split('.').collect();
    set_path_rec(value, &parts, new_value);
}

fn set_path_rec(value: &mut Value, parts: &[&str], new_value: Value) {
    if !value.is_mapping() {
        *value = Value::Mapping(Mapping::new());
    }
    let map = value.as_mapping_mut().expect("just ensured this is a mapping");
    let key = Value::String(parts[0].to_string());

    if parts.len() == 1 {
        map.insert(key, new_value);
        return;
    }

    let child = map
        .entry(key)
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    set_path_rec(child, &parts[1..], new_value);
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => serde_yaml::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}
