use chainctl_core::ChainctlError;

/// Opens `url` in the user's default browser. On environments where that's
/// not possible (headless CI, SSH without display forwarding), the caller
/// should catch the error and fall back to printing the URL — see
/// ARCHITECTURE.md §15 (cross-platform browser-opening edge cases).
pub fn open_url(url: &str) -> Result<(), ChainctlError> {
    open::that(url).map_err(|e| ChainctlError::BrowserLaunchFailed(url.to_string(), e.to_string()))
}
