use chainctl_core::{Chain, ChainctlError};
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct NetworkCmd {
    #[command(subcommand)]
    action: NetworkAction,
}

#[derive(Subcommand)]
enum NetworkAction {
    /// Add a custom chain to `~/.chainctl/registry.override.json`.
    Add {
        /// Short id used everywhere else (`chainctl chains info <id>`, etc).
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long = "chain-id")]
        chain_id: u64,
        #[arg(long)]
        symbol: String,
        /// Repeatable: --rpc-url <a> --rpc-url <b> ...
        #[arg(long = "rpc-url", required = true)]
        rpc_url: Vec<String>,
        #[arg(long = "explorer-url", default_value = "")]
        explorer_url: String,
        /// testnet | mainnet | devnet
        #[arg(long, default_value = "testnet")]
        network: String,
        #[arg(long = "parent-chain", default_value = "")]
        parent_chain: String,
        /// Repeatable: --alias <a> --alias <b> ...
        #[arg(long)]
        alias: Vec<String>,
    },
    /// List custom networks added via `network add`.
    List,
    /// Remove a custom network by id.
    Remove { id: String },
}

pub fn run(ctx: &Context, cmd: NetworkCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        NetworkAction::Add {
            id,
            name,
            chain_id,
            symbol,
            rpc_url,
            explorer_url,
            network,
            parent_chain,
            alias,
        } => add(ctx, id, name, chain_id, symbol, rpc_url, explorer_url, network, parent_chain, alias),
        NetworkAction::List => list(ctx),
        NetworkAction::Remove { id } => remove(ctx, &id),
    }
}

fn parse_network_kind(s: &str) -> Result<chainctl_core::NetworkKind, ChainctlError> {
    match s.to_lowercase().as_str() {
        "testnet" => Ok(chainctl_core::NetworkKind::Testnet),
        "mainnet" => Ok(chainctl_core::NetworkKind::Mainnet),
        "devnet" => Ok(chainctl_core::NetworkKind::Devnet),
        other => Err(ChainctlError::Config(format!(
            "invalid --network '{other}' (expected testnet, mainnet, or devnet)"
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
fn add(
    ctx: &Context,
    id: String,
    name: String,
    chain_id: u64,
    symbol: String,
    rpc_urls: Vec<String>,
    explorer_url: String,
    network: String,
    parent_chain: String,
    aliases: Vec<String>,
) -> Result<(), ChainctlError> {
    let network = parse_network_kind(&network)?;

    let chain = Chain {
        id: id.clone(),
        name,
        chain_id,
        symbol,
        network,
        parent_chain,
        explorer_url,
        rpc_urls,
        aliases,
        faucets: Vec::new(),
    };

    let mut overrides = chainctl_registry::load_overrides(&ctx.paths.registry_override_file)?;
    let replaced = overrides.iter().any(|c| c.id == chain.id);
    overrides.retain(|c| c.id != chain.id);
    overrides.push(chain);
    save_overrides(ctx, &overrides)?;

    if !ctx.quiet {
        let verb = if replaced { "Updated" } else { "Added" };
        println!("{verb} custom network '{id}'. Run `chainctl chains info {id}` to see it.");
    }
    Ok(())
}

fn list(ctx: &Context) -> Result<(), ChainctlError> {
    let overrides = chainctl_registry::load_overrides(&ctx.paths.registry_override_file)?;
    if overrides.is_empty() {
        println!("No custom networks yet. Add one with `chainctl network add <id> --name ... --chain-id ... --symbol ... --rpc-url ...`.");
        return Ok(());
    }
    println!(
        "{}",
        chainctl_output::render_chains(&overrides, ctx.output, &ctx.theme)
    );
    Ok(())
}

fn remove(ctx: &Context, id: &str) -> Result<(), ChainctlError> {
    let mut overrides = chainctl_registry::load_overrides(&ctx.paths.registry_override_file)?;
    let before = overrides.len();
    overrides.retain(|c| c.id != id);

    if overrides.len() == before {
        return Err(ChainctlError::Config(format!(
            "'{id}' is not a custom network — only chains added via `network add` can be removed"
        )));
    }

    save_overrides(ctx, &overrides)?;
    if !ctx.quiet {
        println!("Removed custom network '{id}'.");
    }
    Ok(())
}

fn save_overrides(ctx: &Context, overrides: &[Chain]) -> Result<(), ChainctlError> {
    let raw = serde_json::to_string_pretty(overrides).map_err(|e| ChainctlError::Config(e.to_string()))?;
    chainctl_provider::storage::write_atomic(&ctx.paths.registry_override_file, &raw)
}
