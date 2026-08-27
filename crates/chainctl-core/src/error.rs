#[derive(thiserror::Error, Debug)]
pub enum ChainctlError {
    #[error("unknown chain '{0}'")]
    ChainNotFound(String),

    #[error("no faucets found for '{0}'")]
    NoFaucetsFound(String),

    #[error("no RPC endpoints found for '{0}'")]
    NoRpcEndpoints(String),

    #[error("local registry failed validation: {0}")]
    RegistryCorrupted(String),

    #[error("no network connection and no cached registry")]
    Offline,

    #[error("could not open a browser for {0}: {1}")]
    BrowserLaunchFailed(String, String),

    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl ChainctlError {
    /// Process exit code, following the convention documented in ARCHITECTURE.md §11.
    pub fn exit_code(&self) -> i32 {
        match self {
            ChainctlError::ChainNotFound(_)
            | ChainctlError::NoFaucetsFound(_)
            | ChainctlError::NoRpcEndpoints(_) => 4,
            ChainctlError::RegistryCorrupted(_) => 4,
            ChainctlError::Offline => 3,
            ChainctlError::BrowserLaunchFailed(_, _) => 3,
            ChainctlError::Config(_) => 4,
            ChainctlError::Io(_) => 1,
        }
    }

    /// A short, actionable hint shown beneath the error message.
    pub fn hint(&self) -> Option<String> {
        match self {
            ChainctlError::ChainNotFound(_) => {
                Some("Run `chainctl chains` to see supported chains.".into())
            }
            ChainctlError::NoFaucetsFound(_) => {
                Some("Run `chainctl update` to refresh the faucet registry.".into())
            }
            ChainctlError::NoRpcEndpoints(_) => {
                Some("This chain's registry entry has no rpcUrls configured yet.".into())
            }
            ChainctlError::RegistryCorrupted(_) => {
                Some("Run `chainctl update --force` to re-fetch a clean registry.".into())
            }
            ChainctlError::Offline => {
                Some("Connect to the network and run `chainctl update`.".into())
            }
            ChainctlError::BrowserLaunchFailed(url, _) => Some(format!("Open manually: {url}")),
            _ => None,
        }
    }
}
