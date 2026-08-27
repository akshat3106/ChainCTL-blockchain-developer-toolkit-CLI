mod abi;
mod cache;
mod chains;
mod config;
mod contract;
mod doctor;
mod ens;
mod explorer;
mod faucet;
mod gas;
mod network;
mod rpc;
mod tx;
mod update;
mod version;
mod wallet;

use chainctl_core::{ChainctlError, Registry};
use chainctl_output::{OutputFormat, Theme};
use chainctl_provider::storage::Paths;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "chainctl",
    version,
    about = "Blockchain Developer Toolkit — testnet faucet discovery & management"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format.
    #[arg(long, short = 'o', global = true, value_enum, default_value_t = OutputFormatArg::Table)]
    pub output: OutputFormatArg,

    /// Disable colored output.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Override the `~/.chainctl` config/cache directory.
    #[arg(long, global = true, value_name = "PATH")]
    pub config_dir: Option<String>,

    /// Suppress non-essential output.
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,

    /// Bypass any local cache and force a fresh lookup.
    #[arg(long, global = true)]
    pub fresh: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OutputFormatArg {
    Table,
    Json,
    Plain,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(value: OutputFormatArg) -> Self {
        match value {
            OutputFormatArg::Table => OutputFormat::Table,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Plain => OutputFormat::Plain,
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// List supported chains, or show details for one.
    Chains(chains::ChainsCmd),
    /// Search, inspect, open, or rank testnet faucets.
    Faucet(faucet::FaucetCmd),
    /// List, test, or benchmark chain RPC endpoints.
    Rpc(rpc::RpcCmd),
    /// Add, list, or remove custom chains.
    Network(network::NetworkCmd),
    /// Open a chain's block explorer (homepage, tx, or address).
    Explorer(explorer::ExplorerCmd),
    /// Gas price and estimation.
    Gas(gas::GasCmd),
    /// Read-only wallet lookups (balance).
    Wallet(wallet::WalletCmd),
    /// Transaction status lookup.
    Tx(tx::TxCmd),
    /// Encode/decode function calldata (cast-style signatures).
    Abi(abi::AbiCmd),
    /// Call read-only contract functions.
    Contract(contract::ContractCmd),
    /// ENS name resolution (mainnet).
    Ens(ens::EnsCmd),
    /// Refresh the local faucet registry.
    Update,
    /// Manage the local cache.
    Cache(cache::CacheCmd),
    /// Read or edit `~/.chainctl/config.yaml`.
    Config(config::ConfigCmd),
    /// Check that ChainCTL's environment is healthy.
    Doctor,
    /// Print the ChainCTL version.
    Version,
}

/// Shared, request-scoped state built once in `main.rs` and threaded into
/// every command handler — the constructor-injection point described in
/// ARCHITECTURE.md §3/§4.
pub struct Context {
    pub paths: Paths,
    pub output: OutputFormat,
    pub theme: Theme,
    pub quiet: bool,
    pub fresh: bool,
}

impl Context {
    pub fn from_cli(cli: &Cli) -> Result<Self, ChainctlError> {
        let paths = Paths::resolve(cli.config_dir.as_deref())?;
        paths.ensure_dirs()?;
        Ok(Self {
            paths,
            output: cli.output.into(),
            theme: Theme::detect(cli.no_color),
            quiet: cli.quiet,
            fresh: cli.fresh,
        })
    }

    /// Loads the registry: the user's locally-updated copy if present
    /// (otherwise the snapshot embedded in the binary), with any custom
    /// networks from `registry.override.json` layered on top
    /// (ARCHITECTURE.md §7).
    pub fn load_registry(&self) -> Result<Registry, ChainctlError> {
        let user_copy = (!self.fresh).then_some(self.paths.registry_file.as_path());
        let overrides = Some(self.paths.registry_override_file.as_path());
        chainctl_registry::load_with_overrides(user_copy, overrides)
    }

    pub fn resolve_chain<'a>(
        &self,
        registry: &'a Registry,
        query: &str,
    ) -> Result<&'a chainctl_core::Chain, ChainctlError> {
        registry
            .find_chain(query)
            .ok_or_else(|| ChainctlError::ChainNotFound(query.to_string()))
    }

    /// The RPC URL used by `gas`/`wallet`/`tx`/`contract`/`ens` — always the
    /// first configured endpoint for the chain. There's no multi-endpoint
    /// fallback yet; `chainctl rpc test` is how you'd notice a chain's
    /// primary RPC has gone bad and needs a `network add` override.
    pub fn primary_rpc<'a>(&self, chain: &'a chainctl_core::Chain) -> Result<&'a str, ChainctlError> {
        chain
            .rpc_urls
            .first()
            .map(String::as_str)
            .ok_or_else(|| ChainctlError::NoRpcEndpoints(chain.id.clone()))
    }
}

pub async fn dispatch(cli: Cli) -> Result<(), ChainctlError> {
    let ctx = Context::from_cli(&cli)?;
    match cli.command {
        Commands::Chains(cmd) => chains::run(&ctx, cmd),
        Commands::Faucet(cmd) => faucet::run(&ctx, cmd).await,
        Commands::Rpc(cmd) => rpc::run(&ctx, cmd).await,
        Commands::Network(cmd) => network::run(&ctx, cmd),
        Commands::Explorer(cmd) => explorer::run(&ctx, cmd),
        Commands::Gas(cmd) => gas::run(&ctx, cmd).await,
        Commands::Wallet(cmd) => wallet::run(&ctx, cmd).await,
        Commands::Tx(cmd) => tx::run(&ctx, cmd).await,
        Commands::Abi(cmd) => abi::run(&ctx, cmd),
        Commands::Contract(cmd) => contract::run(&ctx, cmd).await,
        Commands::Ens(cmd) => ens::run(&ctx, cmd).await,
        Commands::Update => update::run(&ctx).await,
        Commands::Cache(cmd) => cache::run(&ctx, cmd).await,
        Commands::Config(cmd) => config::run(&ctx, cmd),
        Commands::Doctor => doctor::run(&ctx),
        Commands::Version => version::run(&ctx),
    }
}
