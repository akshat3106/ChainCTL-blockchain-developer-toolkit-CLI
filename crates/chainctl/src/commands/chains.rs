use chainctl_core::ChainctlError;
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct ChainsCmd {
    #[command(subcommand)]
    action: Option<ChainsAction>,
}

#[derive(Subcommand)]
enum ChainsAction {
    /// Show details for a single chain.
    Info { chain: String },
}

pub fn run(ctx: &Context, cmd: ChainsCmd) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;

    let chains = match &cmd.action {
        None => registry.chains.clone(),
        Some(ChainsAction::Info { chain }) => {
            vec![ctx.resolve_chain(&registry, chain)?.clone()]
        }
    };

    println!(
        "{}",
        chainctl_output::render_chains(&chains, ctx.output, &ctx.theme)
    );
    Ok(())
}
