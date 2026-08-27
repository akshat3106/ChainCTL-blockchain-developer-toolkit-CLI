use chainctl_core::ChainctlError;
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct ExplorerCmd {
    #[command(subcommand)]
    action: ExplorerAction,
}

#[derive(Subcommand)]
enum ExplorerAction {
    /// Open a chain's block explorer homepage.
    Open { chain: String },
    /// Open a transaction on the chain's block explorer.
    Tx { chain: String, hash: String },
    /// Open an address on the chain's block explorer.
    Address { chain: String, address: String },
}

pub fn run(ctx: &Context, cmd: ExplorerCmd) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let (chain_query, url) = match &cmd.action {
        ExplorerAction::Open { chain } => (chain, None),
        ExplorerAction::Tx { chain, hash } => (chain, Some(format!("/tx/{hash}"))),
        ExplorerAction::Address { chain, address } => (chain, Some(format!("/address/{address}"))),
    };

    let chain = ctx.resolve_chain(&registry, chain_query)?;
    if chain.explorer_url.is_empty() {
        return Err(ChainctlError::Config(format!(
            "'{}' has no explorerUrl configured in the registry",
            chain.id
        )));
    }

    let target = match url {
        Some(path) => format!("{}{}", chain.explorer_url.trim_end_matches('/'), path),
        None => chain.explorer_url.clone(),
    };

    chainctl_provider::browser::open_url(&target)?;
    if !ctx.quiet {
        println!("Opening {target}");
    }
    Ok(())
}
