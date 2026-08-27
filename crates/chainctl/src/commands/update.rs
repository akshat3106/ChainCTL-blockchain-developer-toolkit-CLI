use std::time::Duration;

use chainctl_core::ChainctlError;

use super::Context;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Refreshes `~/.chainctl/registry.json`.
///
/// If a registry source URL is configured and reachable, fetches and
/// validates it. Otherwise (no source configured yet, or the network is
/// unavailable) this falls back to writing out the snapshot embedded in the
/// binary, so `chainctl update` always leaves the user with a working,
/// editable local registry file rather than a hard failure — the hosted
/// registry endpoint itself is being set up separately (see ARCHITECTURE.md
/// "Notes on Scope").
pub async fn run(ctx: &Context) -> Result<(), ChainctlError> {
    let source = super::config::get_str(ctx, "registry.source")?.filter(|s| !s.is_empty());

    let (raw, outcome) = match &source {
        Some(url) => match chainctl_provider::http::fetch_text(url, DEFAULT_TIMEOUT).await {
            Ok(body) => (body, UpdateOutcome::Fetched),
            Err(_) => (
                chainctl_registry::embedded_snapshot().to_string(),
                UpdateOutcome::SourceUnreachable,
            ),
        },
        None => (
            chainctl_registry::embedded_snapshot().to_string(),
            UpdateOutcome::NoSourceConfigured,
        ),
    };

    // Validate before writing so a corrupt fetch never clobbers a working local copy.
    chainctl_registry::parse_str(&raw)?;
    chainctl_provider::storage::write_atomic(&ctx.paths.registry_file, &raw)?;

    if !ctx.quiet {
        match outcome {
            UpdateOutcome::Fetched => println!("Registry updated from configured source."),
            UpdateOutcome::SourceUnreachable => {
                println!(
                    "{} was unreachable or returned an invalid registry — wrote the bundled snapshot to {} instead.",
                    source.expect("SourceUnreachable implies a source was set"),
                    ctx.paths.registry_file.display()
                );
            }
            UpdateOutcome::NoSourceConfigured => {
                println!(
                    "No registry source configured — wrote the bundled snapshot to {}.",
                    ctx.paths.registry_file.display()
                );
                println!("Set one with `chainctl config set registry.source <url>` once a registry is published.");
            }
        }
    }
    Ok(())
}

enum UpdateOutcome {
    Fetched,
    SourceUnreachable,
    NoSourceConfigured,
}
