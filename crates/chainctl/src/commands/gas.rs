use chainctl_core::ChainctlError;
use chainctl_provider::gas::EstimateRequest;
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct GasCmd {
    #[command(subcommand)]
    action: GasAction,
}

#[derive(Subcommand)]
enum GasAction {
    /// Current gas price via eth_gasPrice.
    Price { chain: String },
    /// Estimate gas for a call via eth_estimateGas.
    Estimate {
        chain: String,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        from: Option<String>,
        /// Value in wei, decimal or 0x-hex.
        #[arg(long)]
        value: Option<String>,
        /// Calldata as 0x-hex.
        #[arg(long)]
        data: Option<String>,
    },
}

pub async fn run(ctx: &Context, cmd: GasCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        GasAction::Price { chain } => price(ctx, &chain).await,
        GasAction::Estimate { chain, to, from, value, data } => {
            estimate(ctx, &chain, to, from, value, data).await
        }
    }
}

/// Accepts either a decimal string or a `0x`-prefixed hex string and
/// normalizes to a hex quantity, matching the eth_estimateGas over user's
/// most likely input (a plain wei amount) without forcing them to hex-encode
/// it themselves.
fn to_hex_quantity(s: &str) -> Result<String, ChainctlError> {
    if let Some(hex) = s.strip_prefix("0x") {
        if hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(format!("0x{hex}"));
        }
        return Err(ChainctlError::Config(format!("'{s}' is not valid hex")));
    }
    let value: u128 = s
        .parse()
        .map_err(|_| ChainctlError::Config(format!("'{s}' is not a valid decimal or 0x-hex number")))?;
    Ok(format!("0x{value:x}"))
}

async fn price(ctx: &Context, chain_query: &str) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;
    let url = ctx.primary_rpc(chain)?;
    let (_, timeout) = super::config::get_health_settings(ctx)?;

    let wei = chainctl_provider::gas::gas_price(url, timeout)
        .await
        .map_err(ChainctlError::Config)?;
    let gwei = wei as f64 / 1_000_000_000.0;

    match ctx.output {
        chainctl_output::OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({ "chain": chain.id, "wei": wei.to_string(), "gwei": gwei })
            );
        }
        _ => println!("{}: {:.3} Gwei ({} wei)", chain.name, gwei, wei),
    }
    Ok(())
}

async fn estimate(
    ctx: &Context,
    chain_query: &str,
    to: Option<String>,
    from: Option<String>,
    value: Option<String>,
    data: Option<String>,
) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;
    let url = ctx.primary_rpc(chain)?;
    let (_, timeout) = super::config::get_health_settings(ctx)?;

    let value_hex = value.as_deref().map(to_hex_quantity).transpose()?;
    let req = EstimateRequest { from, to, value_hex, data_hex: data };

    let gas = chainctl_provider::gas::estimate_gas(url, &req, timeout)
        .await
        .map_err(ChainctlError::Config)?;

    match ctx.output {
        chainctl_output::OutputFormat::Json => {
            println!("{}", serde_json::json!({ "chain": chain.id, "gas": gas }));
        }
        _ => println!("Estimated gas: {gas} units"),
    }
    Ok(())
}
