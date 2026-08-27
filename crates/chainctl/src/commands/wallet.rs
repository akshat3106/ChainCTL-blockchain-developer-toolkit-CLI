use chainctl_core::ChainctlError;
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct WalletCmd {
    #[command(subcommand)]
    action: WalletAction,
}

#[derive(Subcommand)]
enum WalletAction {
    /// Look up an address's native-token balance (read-only).
    Balance { chain: String, address: String },
}

pub async fn run(ctx: &Context, cmd: WalletCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        WalletAction::Balance { chain, address } => balance(ctx, &chain, &address).await,
    }
}

async fn balance(ctx: &Context, chain_query: &str, address: &str) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;
    let url = ctx.primary_rpc(chain)?;
    let (_, timeout) = super::config::get_health_settings(ctx)?;

    let wei = chainctl_provider::wallet::get_balance(url, address, timeout)
        .await
        .map_err(ChainctlError::Config)?;
    let native = wei as f64 / 1e18;

    match ctx.output {
        chainctl_output::OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "chain": chain.id,
                    "address": address,
                    "wei": wei.to_string(),
                    "balance": native,
                    "symbol": chain.symbol,
                })
            );
        }
        _ => println!("{address}: {native:.6} {} ({wei} wei)", chain.symbol),
    }
    Ok(())
}
