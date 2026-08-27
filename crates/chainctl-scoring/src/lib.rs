//! Standalone weighted-scoring engine for ranking faucets.
//!
//! Zero I/O, zero async — everything here is a pure function over data the
//! caller already has, so it is independently unit-testable and reusable
//! outside the CLI (see ARCHITECTURE.md §10).

use std::collections::HashMap;

use chainctl_core::{Faucet, HealthState, HealthStatus, Score};

#[derive(Debug, Clone, Copy)]
pub struct Weights {
    pub official: f64,
    pub availability: f64,
    pub latency: f64,
    pub community: f64,
    pub recent_failures: f64,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            official: 0.40,
            availability: 0.30,
            latency: 0.15,
            community: 0.10,
            recent_failures: 0.05,
        }
    }
}

/// Scores every faucet in `faucets`, returned sorted highest-first.
///
/// `health` and `ratings` are optional inputs: entries missing from either
/// (e.g. before the Phase 3 health engine or Phase 2 community ratings land)
/// fall back to a neutral 50/100, so the algorithm degrades gracefully rather
/// than penalizing faucets for data ChainCTL simply hasn't collected yet.
pub fn score(
    faucets: &[Faucet],
    health: &[HealthStatus],
    ratings: &HashMap<String, f64>,
    weights: &Weights,
) -> Vec<Score> {
    let health_by_faucet: HashMap<&str, &HealthStatus> =
        health.iter().map(|h| (h.faucet_id.as_str(), h)).collect();

    let fastest_latency_ms = health
        .iter()
        .filter_map(|h| h.latency_ms)
        .filter(|ms| *ms > 0)
        .min();

    let mut scores: Vec<Score> = faucets
        .iter()
        .map(|faucet| {
            let health = health_by_faucet.get(faucet.id.as_str()).copied();

            let official = official_score(faucet);
            let availability = availability_score(health);
            let latency = latency_score(health, fastest_latency_ms);
            let community = ratings.get(&faucet.id).copied().unwrap_or(50.0);
            let recent_failure_penalty = recent_failure_penalty(health);

            // `-0.0 * x` is `-0.0` in IEEE 754, which would otherwise render
            // as an ugly "-0.0" in `--explain` output for a faucet with no
            // recent failures.
            let recent_failures_contribution = if recent_failure_penalty == 0.0 {
                0.0
            } else {
                -weights.recent_failures * recent_failure_penalty
            };

            let mut breakdown = HashMap::new();
            breakdown.insert("official".to_string(), weights.official * official);
            breakdown.insert("availability".to_string(), weights.availability * availability);
            breakdown.insert("latency".to_string(), weights.latency * latency);
            breakdown.insert("community".to_string(), weights.community * community);
            breakdown.insert("recentFailures".to_string(), recent_failures_contribution);

            // Only floor at 0 — never cap at 100. Custom weights (via
            // `config set recommend.weights.*`) can legitimately push the
            // positive factors' sum above 100; capping `total` while leaving
            // `breakdown` uncapped would make `--explain`'s numbers stop
            // adding up, which is worse than an occasional >100 score.
            let total = breakdown.values().sum::<f64>().max(0.0);

            Score {
                faucet_id: faucet.id.clone(),
                total,
                breakdown,
            }
        })
        .collect();

    scores.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap());
    scores
}

fn official_score(faucet: &Faucet) -> f64 {
    let base = faucet.source.base_score();
    let priority_penalty = (faucet.priority.saturating_sub(1) as f64) * 5.0;
    (base - priority_penalty).clamp(0.0, 100.0)
}

fn availability_score(health: Option<&HealthStatus>) -> f64 {
    match health.map(|h| h.status) {
        Some(HealthState::Online) => 100.0,
        Some(HealthState::Degraded) => 50.0,
        Some(HealthState::Maintenance) => 20.0,
        Some(HealthState::Offline) => 0.0,
        Some(HealthState::Unknown) | None => 50.0,
    }
}

fn latency_score(health: Option<&HealthStatus>, fastest_latency_ms: Option<u64>) -> f64 {
    match (health.and_then(|h| h.latency_ms), fastest_latency_ms) {
        (Some(latency), Some(fastest)) if latency > 0 => {
            (100.0 * fastest as f64 / latency as f64).clamp(0.0, 100.0)
        }
        _ => 50.0,
    }
}

fn recent_failure_penalty(health: Option<&HealthStatus>) -> f64 {
    match health.map(|h| h.status) {
        Some(HealthState::Offline) => 100.0,
        Some(HealthState::Degraded) => 40.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chainctl_core::{
        Amount, Cooldown, FaucetMetadata, FaucetSource, HealthCheckConfig, Requirements,
    };
    use chrono::Utc;

    fn faucet(id: &str, source: FaucetSource, priority: u32) -> Faucet {
        Faucet {
            id: id.to_string(),
            name: id.to_string(),
            url: format!("https://{id}.example"),
            source,
            provider: "Test".to_string(),
            requirements: Requirements {
                github_auth: false,
                discord_auth: false,
                captcha: false,
                wallet_connect: false,
                min_mainnet_balance: None,
            },
            cooldown: Cooldown { amount: 24, unit: "hours".into() },
            daily_limit: None,
            amount_per_claim: Amount { amount: "0.1".into(), symbol: "ETH".into() },
            priority,
            exclude_from_health_check: false,
            tags: vec![],
            health_check: HealthCheckConfig {
                method: "HEAD".into(),
                endpoint: format!("https://{id}.example"),
                expected_status: vec![200],
            },
            metadata: FaucetMetadata {
                added_at: Utc::now(),
                last_verified_at: Utc::now(),
                maintainer: "test".into(),
            },
            community_rating: None,
        }
    }

    #[test]
    fn official_outranks_community_with_no_other_signal() {
        let faucets = vec![
            faucet("official", FaucetSource::Official, 1),
            faucet("community", FaucetSource::Community, 1),
        ];
        let scores = score(&faucets, &[], &HashMap::new(), &Weights::default());
        assert_eq!(scores[0].faucet_id, "official");
        assert!(scores[0].total > scores[1].total);
    }

    #[test]
    fn offline_faucet_is_penalized_below_unknown_faucet() {
        let faucets = vec![
            faucet("offline", FaucetSource::Official, 1),
            faucet("unknown", FaucetSource::Official, 1),
        ];
        let health = vec![HealthStatus {
            faucet_id: "offline".into(),
            checked_at: Utc::now(),
            status: HealthState::Offline,
            http_status: None,
            latency_ms: None,
            ssl_valid: None,
            ssl_expires_at: None,
            error: Some("connection refused".into()),
        }];
        let scores = score(&faucets, &health, &HashMap::new(), &Weights::default());
        let offline = scores.iter().find(|s| s.faucet_id == "offline").unwrap();
        let unknown = scores.iter().find(|s| s.faucet_id == "unknown").unwrap();
        assert!(offline.total < unknown.total);
    }
}
