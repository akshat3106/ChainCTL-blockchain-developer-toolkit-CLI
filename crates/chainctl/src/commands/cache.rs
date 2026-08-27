use chainctl_core::ChainctlError;
use clap::{Args, Subcommand};

use super::Context;

#[derive(Args)]
pub struct CacheCmd {
    #[command(subcommand)]
    action: CacheAction,
}

#[derive(Subcommand)]
enum CacheAction {
    /// Delete everything under `~/.chainctl/cache/`.
    Clear,
    /// Show cache location, size, and file count.
    Info,
    /// Re-fetch the registry and clear cached health results.
    Refresh,
}

pub async fn run(ctx: &Context, cmd: CacheCmd) -> Result<(), ChainctlError> {
    match cmd.action {
        CacheAction::Clear => clear(ctx),
        CacheAction::Info => info(ctx),
        CacheAction::Refresh => {
            super::update::run(ctx).await?;
            clear(ctx)
        }
    }
}

fn clear(ctx: &Context) -> Result<(), ChainctlError> {
    if ctx.paths.cache_dir.exists() {
        std::fs::remove_dir_all(&ctx.paths.cache_dir)?;
    }
    std::fs::create_dir_all(&ctx.paths.cache_dir)?;
    if !ctx.quiet {
        println!("Cache cleared: {}", ctx.paths.cache_dir.display());
    }
    Ok(())
}

fn info(ctx: &Context) -> Result<(), ChainctlError> {
    let (file_count, total_bytes) = dir_stats(&ctx.paths.cache_dir)?;
    println!("Cache directory: {}", ctx.paths.cache_dir.display());
    println!("Files:           {file_count}");
    println!("Size:            {} KB", (total_bytes as f64 / 1024.0).ceil());
    Ok(())
}

fn dir_stats(dir: &std::path::Path) -> Result<(u64, u64), ChainctlError> {
    if !dir.exists() {
        return Ok((0, 0));
    }
    let mut count = 0;
    let mut bytes = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            count += 1;
            bytes += entry.metadata()?.len();
        }
    }
    Ok((count, bytes))
}
