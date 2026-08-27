use chainctl_core::ChainctlError;
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct ContractCmd {
    #[command(subcommand)]
    action: ContractAction,
}

#[derive(Subcommand)]
enum ContractAction {
    /// Call a read-only contract function, e.g.
    /// `chainctl contract read base 0x... "balanceOf(address)(uint256)" 0xabc...`.
    Read {
        chain: String,
        address: String,
        signature: String,
        args: Vec<String>,
    },
}

pub async fn run(ctx: &Context, cmd: ContractCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        ContractAction::Read { chain, address, signature, args } => {
            read(ctx, &chain, &address, &signature, &args).await
        }
    }
}

async fn read(
    ctx: &Context,
    chain_query: &str,
    address: &str,
    signature: &str,
    args: &[String],
) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;
    let url = ctx.primary_rpc(chain)?;
    let (_, timeout) = super::config::get_health_settings(ctx)?;

    let sig = chainctl_provider::abi::parse_signature(signature).map_err(ChainctlError::Config)?;
    if sig.inputs.len() != args.len() {
        return Err(ChainctlError::Config(format!(
            "'{signature}' takes {} argument(s), got {}",
            sig.inputs.len(),
            args.len()
        )));
    }
    if sig.outputs.is_empty() {
        return Err(ChainctlError::Config(
            "signature has no output types — use \"name(inputTypes)(outputTypes)\", e.g. \"balanceOf(address)(uint256)\"".to_string(),
        ));
    }

    let values = chainctl_provider::contract::call_read(url, address, &sig, args, timeout)
        .await
        .map_err(ChainctlError::Config)?;

    match ctx.output {
        chainctl_output::OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({ "chain": chain.id, "address": address, "result": values })
            );
        }
        _ => {
            for (t, v) in sig.outputs.iter().zip(values) {
                println!("{t}: {v}");
            }
        }
    }
    Ok(())
}
