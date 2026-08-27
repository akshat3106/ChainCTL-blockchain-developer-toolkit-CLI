use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use chainctl_core::{ChainctlError, HealthStatus};
use chrono::Utc;

/// On-disk shape of `~/.chainctl/cache/health.json` — a flat map of
/// faucet id -> last known `HealthStatus`, so `recommend`/`open`/`info`
/// can read through it without re-probing faucets on every invocation
/// (ARCHITECTURE.md §9 — cache aggressively).
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct HealthCacheFile {
    entries: HashMap<String, HealthStatus>,
}

pub fn load(path: &Path) -> HashMap<String, HealthStatus> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<HealthCacheFile>(&raw).ok())
        .map(|f| f.entries)
        .unwrap_or_default()
}

pub fn save(path: &Path, entries: &HashMap<String, HealthStatus>) -> Result<(), ChainctlError> {
    let file = HealthCacheFile { entries: entries.clone() };
    let raw = serde_json::to_string_pretty(&file).map_err(|e| ChainctlError::Config(e.to_string()))?;
    crate::storage::write_atomic(path, &raw)
}

pub fn is_fresh(status: &HealthStatus, ttl: Duration) -> bool {
    match Utc::now().signed_duration_since(status.checked_at).to_std() {
        Ok(age) => age < ttl,
        Err(_) => false, // checked_at is in the future — treat as stale rather than trust it
    }
}
