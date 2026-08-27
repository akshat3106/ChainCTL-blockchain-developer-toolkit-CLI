use std::time::Duration;

use chainctl_core::ChainctlError;

pub(crate) const USER_AGENT: &str = concat!(
    "chainctl/",
    env!("CARGO_PKG_VERSION"),
    " (+",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

/// Fetches `url` as text with a short timeout and an honest User-Agent
/// (ARCHITECTURE.md §9 — health/registry probes must identify themselves).
pub async fn fetch_text(url: &str, timeout: Duration) -> Result<String, ChainctlError> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| ChainctlError::Config(e.to_string()))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| ChainctlError::Offline)?;

    if !response.status().is_success() {
        return Err(ChainctlError::RegistryCorrupted(format!(
            "unexpected HTTP status {}",
            response.status()
        )));
    }

    response
        .text()
        .await
        .map_err(|e| ChainctlError::RegistryCorrupted(e.to_string()))
}
