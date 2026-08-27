use chainctl_core::ChainctlError;
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct TxCmd {
    #[command(subcommand)]
    action: TxAction,
}

#[derive(Subcommand)]
enum TxAction {
    /// Look up a transaction's status by hash.
    Status { chain: String, hash: String },
}

pub async fn run(ctx: &Context, cmd: TxCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        TxAction::Status { chain, hash } => status(ctx, &chain, &hash).await,
    }
}

async fn status(ctx: &Context, chain_query: &str, hash: &str) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;
    let url = ctx.primary_rpc(chain)?;
    let (_, timeout) = super::config::get_health_settings(ctx)?;

    let tx_status = chainctl_provider::tx::status(url, hash, timeout)
        .await
        .map_err(ChainctlError::Config)?;

    if ctx.output == chainctl_output::OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&tx_status).unwrap());
        return Ok(());
    }

    if !tx_status.found {
        println!("{hash}: not found (never broadcast, or not yet propagated)");
        return Ok(());
    }
    if tx_status.pending {
        println!("{hash}: pending (broadcast, not yet mined)");
        return Ok(());
    }

    let outcome = match tx_status.success {
        Some(true) => "success",
        Some(false) => "failed",
        None => "unknown",
    };
    println!("{hash}: {outcome}");
    if let Some(block) = tx_status.block_number {
        println!("  Block:    {block}");
    }
    if let Some(gas) = tx_status.gas_used {
        println!("  Gas used: {gas}");
    }
    if let Some(from) = &tx_status.from {
        println!("  From:     {from}");
    }
    if let Some(to) = &tx_status.to {
        println!("  To:       {to}");
    }
    Ok(())
}
