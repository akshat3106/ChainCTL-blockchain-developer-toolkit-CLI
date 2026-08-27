use std::path::PathBuf;

use chainctl_core::ChainctlError;
use directories::BaseDirs;

/// Resolved `~/.chainctl/` layout (ARCHITECTURE.md §7). Overridable via the
/// `$CHAINCTL_HOME` environment variable or an explicit `--config-dir` flag.
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
    pub config_file: PathBuf,
    pub registry_file: PathBuf,
    pub registry_override_file: PathBuf,
    pub cache_dir: PathBuf,
    pub health_cache_file: PathBuf,
    pub logs_dir: PathBuf,
}

impl Paths {
    pub fn resolve(override_dir: Option<&str>) -> Result<Self, ChainctlError> {
        let root = if let Some(dir) = override_dir {
            PathBuf::from(dir)
        } else if let Ok(dir) = std::env::var("CHAINCTL_HOME") {
            PathBuf::from(dir)
        } else {
            let base = BaseDirs::new().ok_or_else(|| {
                ChainctlError::Config("could not determine home directory".to_string())
            })?;
            base.home_dir().join(".chainctl")
        };

        let cache_dir = root.join("cache");
        Ok(Self {
            config_file: root.join("config.yaml"),
            registry_file: root.join("registry.json"),
            registry_override_file: root.join("registry.override.json"),
            health_cache_file: cache_dir.join("health.json"),
            cache_dir,
            logs_dir: root.join("logs"),
            root,
        })
    }

    pub fn ensure_dirs(&self) -> Result<(), ChainctlError> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.logs_dir)?;
        Ok(())
    }
}

/// Write-temp-then-rename so a crash or concurrent `chainctl` invocation
/// never observes a partially-written file (ARCHITECTURE.md §2/§7).
pub fn write_atomic(path: &std::path::Path, contents: &str) -> Result<(), ChainctlError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, contents)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}
