use chainctl_core::ChainctlError;
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct EnsCmd {
    #[command(subcommand)]
    action: EnsAction,
}

#[derive(Subcommand)]
enum EnsAction {
    /// Resolve an ENS name to an address (e.g. `vitalik.eth`).
    Resolve { name: String },
    /// Resolve an address to its primary ENS name, if set.
    Reverse { address: String },
}

pub async fn run(ctx: &Context, cmd: EnsCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        EnsAction::Resolve { name } => resolve(ctx, &name).await,
        EnsAction::Reverse { address } => reverse(ctx, &address).await,
    }
}

async fn resolve(ctx: &Context, name: &str) -> Result<(), ChainctlError> {
    let url = super::config::get_ens_rpc_url(ctx)?;
    let (_, timeout) = super::config::get_health_settings(ctx)?;

    let result = chainctl_provider::ens::resolve(&url, name, timeout)
        .await
        .map_err(ChainctlError::Config)?;

    match (ctx.output, &result) {
        (chainctl_output::OutputFormat::Json, _) => {
            println!("{}", serde_json::json!({ "name": name, "address": result }));
        }
        (_, Some(address)) => println!("{name} -> {address}"),
        (_, None) => println!("{name} has no address record"),
    }
    Ok(())
}

async fn reverse(ctx: &Context, address: &str) -> Result<(), ChainctlError> {
    let url = super::config::get_ens_rpc_url(ctx)?;
    let (_, timeout) = super::config::get_health_settings(ctx)?;

    let result = chainctl_provider::ens::reverse(&url, address, timeout)
        .await
        .map_err(ChainctlError::Config)?;

    match (ctx.output, &result) {
        (chainctl_output::OutputFormat::Json, _) => {
            println!("{}", serde_json::json!({ "address": address, "name": result }));
        }
        (_, Some(name)) => println!("{address} -> {name}"),
        (_, None) => println!("{address} has no primary ENS name set"),
    }
    Ok(())
}
