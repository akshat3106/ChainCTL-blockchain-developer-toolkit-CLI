use std::collections::HashMap;
use std::time::Duration;

use chainctl_core::{Chain, ChainctlError, Faucet, HealthStatus, Score};
use chainctl_output::HealthRow;
use clap::{Args, Subcommand};

use super::Context;

/// Floor for `--watch`'s `--interval`, so a mistyped `--interval 1` can't
/// turn `faucet status --watch` into an accidental hammer against faucet
/// servers (ARCHITECTURE.md §9).
const MIN_WATCH_INTERVAL_SECS: u64 = 15;

#[derive(Args)]
pub struct FaucetCmd {
    #[command(subcommand)]
    action: FaucetAction,
}

#[derive(Subcommand)]
enum FaucetAction {
    /// List every known faucet for a chain.
    Search {
        chain: String,
        /// Filter by source: official, partner, or community.
        #[arg(long)]
        source: Option<String>,
    },
    /// Show full detail for one faucet.
    Info {
        chain: String,
        /// Faucet id (defaults to the top-ranked one).
        #[arg(long)]
        faucet: Option<String>,
    },
    /// Open the recommended (or a specific) faucet in the default browser.
    Open {
        chain: String,
        #[arg(long)]
        faucet: Option<String>,
    },
    /// Health-check every faucet for a chain (or all chains), concurrently.
    Status {
        chain: Option<String>,
        /// Repeat the check on an interval until interrupted (Ctrl+C).
        #[arg(long)]
        watch: bool,
        /// Seconds between checks in --watch mode (floor: 15).
        #[arg(long, default_value_t = 60)]
        interval: u64,
    },
    /// Rank faucets for a chain and print the best one.
    Recommend {
        chain: String,
        /// Print the per-factor score breakdown.
        #[arg(long)]
        explain: bool,
    },
}

pub async fn run(ctx: &Context, cmd: FaucetCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        FaucetAction::Search { chain, source } => search(ctx, &chain, source.as_deref()),
        FaucetAction::Info { chain, faucet } => info(ctx, &chain, faucet.as_deref()).await,
        FaucetAction::Open { chain, faucet } => open(ctx, &chain, faucet.as_deref()).await,
        FaucetAction::Status { chain, watch, interval } => {
            status(ctx, chain.as_deref(), watch, interval).await
        }
        FaucetAction::Recommend { chain, explain } => recommend(ctx, &chain, explain).await,
    }
}

fn search(ctx: &Context, chain_query: &str, source: Option<&str>) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;

    let faucets: Vec<&Faucet> = chain
        .faucets
        .iter()
        .filter(|f| match source {
            Some(s) => format!("{:?}", f.source).eq_ignore_ascii_case(s),
            None => true,
        })
        .collect();

    if faucets.is_empty() {
        return Err(ChainctlError::NoFaucetsFound(chain.id.clone()));
    }

    println!(
        "{}",
        chainctl_output::render_faucets(chain, &faucets, ctx.output, &ctx.theme)
    );
    Ok(())
}

/// Ranks every faucet on a chain using live-but-cached health data, any
/// curated `communityRating`s, and the weights from `config.yaml` — the
/// Phase 2 recommendation engine, now backed by Phase 3's concurrent checker.
async fn rank_faucets(ctx: &Context, chain: &Chain) -> Result<Vec<Score>, ChainctlError> {
    let health = cached_health(ctx, &chain.faucets).await?;
    let ratings: HashMap<String, f64> = chain
        .faucets
        .iter()
        .filter_map(|f| f.community_rating.map(|r| (f.id.clone(), r)))
        .collect();
    let weights = super::config::get_weights(ctx)?;
    Ok(chainctl_scoring::score(&chain.faucets, &health, &ratings, &weights))
}

/// Read-through health cache: fresh entries are reused as-is; everything
/// stale or missing is probed in one concurrent, bounded, per-host-throttled
/// batch via `chainctl_provider::health::check_all`, and the cache is
/// updated so the next call is instant (ARCHITECTURE.md §9).
async fn cached_health(ctx: &Context, faucets: &[Faucet]) -> Result<Vec<HealthStatus>, ChainctlError> {
    let ttl = Duration::from_secs(super::config::get_cache_ttl_minutes(ctx)? * 60);
    let mut cache = chainctl_provider::health_cache::load(&ctx.paths.health_cache_file);

    let mut resolved: HashMap<String, HealthStatus> = HashMap::new();
    let mut stale = Vec::new();
    for faucet in faucets {
        let cached = (!ctx.fresh).then(|| cache.get(&faucet.id)).flatten();
        match cached {
            Some(s) if chainctl_provider::health_cache::is_fresh(s, ttl) => {
                resolved.insert(faucet.id.clone(), s.clone());
            }
            _ => stale.push(faucet.clone()),
        }
    }

    if !stale.is_empty() {
        let (concurrency, timeout) = super::config::get_health_settings(ctx)?;
        let checked = chainctl_provider::health::check_all(&stale, timeout, concurrency).await;
        for status in checked {
            cache.insert(status.faucet_id.clone(), status.clone());
            resolved.insert(status.faucet_id.clone(), status);
        }
        chainctl_provider::health_cache::save(&ctx.paths.health_cache_file, &cache)?;
    }

    Ok(faucets
        .iter()
        .filter_map(|f| resolved.get(&f.id).cloned())
        .collect())
}

async fn pick_faucet<'a>(
    ctx: &Context,
    chain: &'a Chain,
    explicit_id: Option<&str>,
) -> Result<&'a Faucet, ChainctlError> {
    if let Some(id) = explicit_id {
        return chain
            .faucets
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| ChainctlError::NoFaucetsFound(format!("{}/{}", chain.id, id)));
    }

    let scores = rank_faucets(ctx, chain).await?;
    let top = scores
        .first()
        .ok_or_else(|| ChainctlError::NoFaucetsFound(chain.id.clone()))?;
    chain
        .faucets
        .iter()
        .find(|f| f.id == top.faucet_id)
        .ok_or_else(|| ChainctlError::NoFaucetsFound(chain.id.clone()))
}

async fn info(ctx: &Context, chain_query: &str, faucet_id: Option<&str>) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;
    let faucet = pick_faucet(ctx, chain, faucet_id).await?;

    println!(
        "{}",
        chainctl_output::render_faucet_info(chain, faucet, ctx.output, &ctx.theme)
    );
    Ok(())
}

async fn open(ctx: &Context, chain_query: &str, faucet_id: Option<&str>) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;
    let faucet = pick_faucet(ctx, chain, faucet_id).await?;

    chainctl_provider::browser::open_url(&faucet.url)?;
    if !ctx.quiet {
        println!("Opening {} — {}", faucet.name, faucet.url);
    }
    Ok(())
}

async fn status(
    ctx: &Context,
    chain_query: Option<&str>,
    watch: bool,
    interval: u64,
) -> Result<(), ChainctlError> {
    let interval = interval.max(MIN_WATCH_INTERVAL_SECS);

    loop {
        run_status_once(ctx, chain_query, watch).await?;

        if !watch {
            return Ok(());
        }
        if !ctx.quiet {
            eprintln!("\nWatching — next check in {interval}s (Ctrl+C to stop)");
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

async fn run_status_once(ctx: &Context, chain_query: Option<&str>, watch: bool) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chains: Vec<&Chain> = match chain_query {
        Some(q) => vec![ctx.resolve_chain(&registry, q)?],
        None => registry.chains.iter().collect(),
    };

    let faucets: Vec<Faucet> = chains.iter().flat_map(|c| c.faucets.iter().cloned()).collect();
    let chain_by_faucet: HashMap<&str, &str> = chains
        .iter()
        .flat_map(|c| c.faucets.iter().map(move |f| (f.id.as_str(), c.id.as_str())))
        .collect();

    if !ctx.quiet && ctx.output == chainctl_output::OutputFormat::Table {
        eprintln!("Checking faucet health (this hits real faucet servers, please be patient)...");
    }

    // `status` always probes live — it's the command whose whole purpose is
    // a fresh report — but still writes through the shared cache, so a
    // subsequent `recommend`/`open` doesn't have to re-probe within the TTL.
    let (concurrency, timeout) = super::config::get_health_settings(ctx)?;
    let statuses = chainctl_provider::health::check_all(&faucets, timeout, concurrency).await;

    let mut cache = chainctl_provider::health_cache::load(&ctx.paths.health_cache_file);
    for status in &statuses {
        cache.insert(status.faucet_id.clone(), status.clone());
    }
    chainctl_provider::health_cache::save(&ctx.paths.health_cache_file, &cache)?;

    let faucet_by_id: HashMap<&str, &Faucet> = faucets.iter().map(|f| (f.id.as_str(), f)).collect();
    let mut rows: Vec<HealthRow> = statuses
        .into_iter()
        .filter_map(|status| {
            let faucet = *faucet_by_id.get(status.faucet_id.as_str())?;
            let chain = chain_by_faucet.get(status.faucet_id.as_str()).copied().unwrap_or("");
            Some(HealthRow {
                chain: chain.to_string(),
                faucet_name: faucet.name.clone(),
                url: faucet.url.clone(),
                status,
            })
        })
        .collect();
    // Concurrent checks complete in arbitrary order; sort for a stable display.
    rows.sort_by(|a, b| a.chain.cmp(&b.chain).then(a.faucet_name.cmp(&b.faucet_name)));

    if watch {
        print!("\x1B[2J\x1B[H"); // clear screen between rounds
        println!(
            "chainctl faucet status — {}\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
    }
    println!(
        "{}",
        chainctl_output::render_health(&rows, ctx.output, &ctx.theme)
    );
    Ok(())
}

async fn recommend(ctx: &Context, chain_query: &str, explain: bool) -> Result<(), ChainctlError> {
    let registry = ctx.load_registry()?;
    let chain = ctx.resolve_chain(&registry, chain_query)?;

    if chain.faucets.is_empty() {
        return Err(ChainctlError::NoFaucetsFound(chain.id.clone()));
    }

    let scores = rank_faucets(ctx, chain).await?;
    let top = &scores[0];
    let faucet = chain
        .faucets
        .iter()
        .find(|f| f.id == top.faucet_id)
        .expect("scored faucet must exist in chain.faucets");

    println!(
        "{}",
        chainctl_output::render_recommendation(chain, faucet, top, explain, ctx.output, &ctx.theme)
    );
    Ok(())
}
