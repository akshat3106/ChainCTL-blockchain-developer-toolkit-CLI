use std::path::Path;

use chainctl_core::{Chain, ChainctlError, Registry};

/// The registry snapshot bundled into the binary at compile time, so ChainCTL
/// works with zero network access on first run (see ARCHITECTURE.md §7).
const EMBEDDED_SNAPSHOT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../configs/registry.snapshot.json"
));

/// Loads the registry, preferring a user-cached copy (post `chainctl update`)
/// over the embedded snapshot when one exists on disk.
pub fn load(user_registry_path: Option<&Path>) -> Result<Registry, ChainctlError> {
    if let Some(path) = user_registry_path {
        if path.exists() {
            let raw = std::fs::read_to_string(path)?;
            return parse(&raw);
        }
    }
    load_embedded()
}

/// Parses and returns the embedded fallback registry directly.
pub fn load_embedded() -> Result<Registry, ChainctlError> {
    parse(EMBEDDED_SNAPSHOT)
}

/// Loads the base registry (§`load`) and layers `registry.override.json` on
/// top — a chain in the override file replaces a base chain with the same
/// `id`, or is appended as a new one (the `chainctl network add/remove`
/// module writes this file; every other command reads through it here so
/// custom networks show up everywhere, not just in `network list`).
pub fn load_with_overrides(
    user_registry_path: Option<&Path>,
    override_path: Option<&Path>,
) -> Result<Registry, ChainctlError> {
    let base = load(user_registry_path)?;
    let overrides = match override_path {
        Some(p) => load_overrides(p)?,
        None => Vec::new(),
    };
    Ok(merge_overrides(base, overrides))
}

/// Reads just the override file's chains — used by `chainctl network list`.
/// An absent or empty file means "no custom networks yet," not an error.
pub fn load_overrides(path: &Path) -> Result<Vec<Chain>, ChainctlError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&raw)
        .map_err(|e| ChainctlError::RegistryCorrupted(format!("registry.override.json: {e}")))
}

fn merge_overrides(mut registry: Registry, overrides: Vec<Chain>) -> Registry {
    for chain in overrides {
        match registry.chains.iter_mut().find(|c| c.id == chain.id) {
            Some(existing) => *existing = chain,
            None => registry.chains.push(chain),
        }
    }
    registry
}

/// Raw JSON text of the embedded fallback registry.
pub fn embedded_snapshot() -> &'static str {
    EMBEDDED_SNAPSHOT
}

/// Parses raw registry JSON text, validating it in the process.
pub fn parse_str(raw: &str) -> Result<Registry, ChainctlError> {
    parse(raw)
}

fn parse(raw: &str) -> Result<Registry, ChainctlError> {
    serde_json::from_str(raw).map_err(|e| ChainctlError::RegistryCorrupted(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_snapshot_parses() {
        let registry = load_embedded().expect("embedded snapshot must parse");
        assert!(!registry.chains.is_empty());
    }

    #[test]
    fn finds_chain_by_alias() {
        let registry = load_embedded().unwrap();
        assert!(registry.find_chain("base").is_some());
        assert!(registry.find_chain("does-not-exist").is_none());
    }
}
