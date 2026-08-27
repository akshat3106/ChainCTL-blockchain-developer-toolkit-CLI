use chainctl_core::{Chain, ChainctlError, Faucet, HealthState, RpcLatencyStats, Score};
use comfy_table::{presets::UTF8_FULL_CONDENSED, ContentArrangement, Table};
use owo_colors::OwoColorize;

use crate::theme::Theme;
use crate::{HealthRow, RpcListRow, RpcTestRow};

fn status_label(status: HealthState, theme: &Theme) -> String {
    let (text, apply): (&str, fn(&str) -> String) = match status {
        HealthState::Online => ("Online", |s| s.green().to_string()),
        HealthState::Offline => ("Offline", |s| s.red().to_string()),
        HealthState::Degraded => ("Slow", |s| s.yellow().to_string()),
        HealthState::Maintenance => ("Maintenance", |s| s.blue().to_string()),
        HealthState::Unknown => ("Unknown", |s| s.dimmed().to_string()),
    };
    if theme.color {
        apply(text)
    } else {
        text.to_string()
    }
}

fn requirements_summary(f: &Faucet) -> String {
    let mut reqs = vec![];
    if f.requirements.github_auth {
        reqs.push("GitHub");
    }
    if f.requirements.discord_auth {
        reqs.push("Discord");
    }
    if f.requirements.captcha {
        reqs.push("CAPTCHA");
    }
    if f.requirements.wallet_connect {
        reqs.push("WalletConnect");
    }
    if reqs.is_empty() {
        "none".to_string()
    } else {
        reqs.join(", ")
    }
}

pub fn chains(chains: &[Chain], _theme: &Theme) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["ID", "Name", "Chain ID", "Symbol", "Network", "Faucets"]);

    for chain in chains {
        table.add_row(vec![
            chain.id.clone(),
            chain.name.clone(),
            chain.chain_id.to_string(),
            chain.symbol.clone(),
            format!("{:?}", chain.network).to_lowercase(),
            chain.faucets.len().to_string(),
        ]);
    }

    table.to_string()
}

pub fn faucets(chain: &Chain, faucets: &[&Faucet], theme: &Theme) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Name", "Source", "Provider", "Cooldown", "Requires", "URL"]);

    for f in faucets {
        let source = format!("{:?}", f.source).to_lowercase();
        let source = if theme.color && f.source.base_score() >= 100.0 {
            source.green().to_string()
        } else {
            source
        };
        table.add_row(vec![
            f.name.clone(),
            source,
            f.provider.clone(),
            format!("{}{}", f.cooldown.amount, &f.cooldown.unit[..1]),
            requirements_summary(f),
            f.url.clone(),
        ]);
    }

    format!("Faucets for {} ({})\n{}", chain.name, chain.id, table)
}

pub fn faucet_info(chain: &Chain, faucet: &Faucet, theme: &Theme) -> String {
    let heading = if theme.color {
        format!("{}", faucet.name.bold())
    } else {
        faucet.name.clone()
    };

    let mut out = String::new();
    out.push_str(&format!("{heading}  ({})\n\n", chain.name));
    out.push_str(&format!("  URL:              {}\n", faucet.url));
    out.push_str(&format!("  Source:           {:?}\n", faucet.source));
    out.push_str(&format!("  Provider:         {}\n", faucet.provider));
    out.push_str(&format!("  Requirements:     {}\n", requirements_summary(faucet)));
    out.push_str(&format!(
        "  Cooldown:         {} {}\n",
        faucet.cooldown.amount, faucet.cooldown.unit
    ));
    out.push_str(&format!(
        "  Amount per claim: {} {}\n",
        faucet.amount_per_claim.amount, faucet.amount_per_claim.symbol
    ));
    out.push_str(&format!(
        "  Last verified:    {}\n",
        faucet.metadata.last_verified_at.format("%Y-%m-%d")
    ));
    out.push_str(&format!(
        "  Community rating: {}\n",
        faucet
            .community_rating
            .map(|r| format!("{r:.0}/100"))
            .unwrap_or_else(|| "no ratings yet".to_string())
    ));
    out
}

pub fn recommendation(
    chain: &Chain,
    faucet: &Faucet,
    score: &Score,
    explain: bool,
    theme: &Theme,
) -> String {
    let mut out = String::new();
    let label = if theme.color {
        "Recommended".green().bold().to_string()
    } else {
        "Recommended".to_string()
    };
    out.push_str(&format!(
        "{label}: {} for {} — score {:.1}/100\n  {}\n",
        faucet.name, chain.name, score.total, faucet.url
    ));

    if explain {
        out.push_str("\n  Breakdown:\n");
        let mut entries: Vec<_> = score.breakdown.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (factor, contribution) in entries {
            out.push_str(&format!("    {factor:<16} {contribution:+.1}\n"));
        }
    }

    out
}

/// Renders SSL certificate expiry as "<N>d" (days remaining), color-coded:
/// red inside 14 days, yellow inside 30, green otherwise. `ssl_valid ==
/// Some(true)` with no expiry means the handshake succeeded (via the real
/// HTTP request) but the separate expiry-reading probe didn't — shown as a
/// plain "valid" rather than treated as an error, since it's best-effort.
fn ssl_label(status: &chainctl_core::HealthStatus, theme: &Theme) -> String {
    match (status.ssl_valid, status.ssl_expires_at) {
        (Some(true), Some(expires_at)) => {
            let days_left = (expires_at - chrono::Utc::now()).num_days();
            let text = format!("{days_left}d");
            if !theme.color {
                text
            } else if days_left <= 14 {
                text.red().to_string()
            } else if days_left <= 30 {
                text.yellow().to_string()
            } else {
                text.green().to_string()
            }
        }
        (Some(true), None) => "valid".to_string(),
        (Some(false), _) => "n/a".to_string(),
        (None, _) => "-".to_string(),
    }
}

pub fn health(rows: &[HealthRow], theme: &Theme) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Chain", "Faucet", "Status", "HTTP", "Latency", "SSL"]);

    for row in rows {
        table.add_row(vec![
            row.chain.clone(),
            row.faucet_name.clone(),
            status_label(row.status.status, theme),
            row.status
                .http_status
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".to_string()),
            row.status
                .latency_ms
                .map(|ms| format!("{ms}ms"))
                .unwrap_or_else(|| "-".to_string()),
            ssl_label(&row.status, theme),
        ]);
    }

    table.to_string()
}

pub fn rpc_list(rows: &[RpcListRow]) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Chain", "Chain ID", "RPC URL"]);

    for row in rows {
        table.add_row(vec![row.chain.clone(), row.chain_id.to_string(), row.url.clone()]);
    }

    table.to_string()
}

fn bool_label(value: bool, theme: &Theme) -> String {
    let (text, color_ok) = if value { ("yes", true) } else { ("no", false) };
    if !theme.color {
        text.to_string()
    } else if color_ok {
        text.green().to_string()
    } else {
        text.red().to_string()
    }
}

pub fn rpc_test(rows: &[RpcTestRow], theme: &Theme) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Chain", "URL", "Reachable", "Chain ID OK", "Latency", "Error"]);

    for row in rows {
        let r = &row.result;
        table.add_row(vec![
            row.chain.clone(),
            r.url.clone(),
            bool_label(r.reachable, theme),
            if r.reachable { bool_label(r.chain_id_matches, theme) } else { "-".to_string() },
            r.latency_ms.map(|ms| format!("{ms}ms")).unwrap_or_else(|| "-".to_string()),
            r.error.clone().unwrap_or_default(),
        ]);
    }

    table.to_string()
}

pub fn rpc_latency(chain: &Chain, stats: &[RpcLatencyStats], _theme: &Theme) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["URL", "Samples", "Failures", "Min", "Avg", "Max"]);

    for s in stats {
        table.add_row(vec![
            s.url.clone(),
            s.samples.to_string(),
            s.failures.to_string(),
            s.min_ms.map(|v| format!("{v}ms")).unwrap_or_else(|| "-".to_string()),
            s.avg_ms.map(|v| format!("{v}ms")).unwrap_or_else(|| "-".to_string()),
            s.max_ms.map(|v| format!("{v}ms")).unwrap_or_else(|| "-".to_string()),
        ]);
    }

    format!("RPC latency for {} ({})\n{}", chain.name, chain.id, table)
}

pub fn error(err: &ChainctlError, theme: &Theme) -> String {
    let msg = err.to_string();
    let mut out = if theme.color {
        format!("{} {}", "✗".red().bold(), msg)
    } else {
        format!("x {msg}")
    };
    if let Some(hint) = err.hint() {
        let arrow = if theme.color {
            format!("{}", "→".dimmed())
        } else {
            "->".to_string()
        };
        out.push_str(&format!("\n  {arrow} {hint}"));
    }
    out
}
