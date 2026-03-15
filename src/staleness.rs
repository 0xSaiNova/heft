//! Staleness scoring for ranking items by reclaim priority.

use crate::config::StalenessConfig;

pub fn compute_staleness(
    size_bytes: u64,
    last_modified: Option<i64>,
    now: i64,
    config: &StalenessConfig,
) -> f64 {
    let age_days = match last_modified {
        Some(ts) => ((now - ts).max(0) as f64) / 86400.0,
        None => return size_bytes as f64,
    };
    let factor = config
        .brackets
        .iter()
        .rev()
        .find(|b| age_days >= b.days as f64)
        .map(|b| b.factor)
        .unwrap_or(config.default_factor);
    size_bytes as f64 * factor
}

pub fn staleness_label(active: Option<bool>, age_days: Option<f64>) -> &'static str {
    if active == Some(true) {
        return "active";
    }
    match age_days {
        None => "unknown",
        Some(d) if d < 7.0 => "fresh",
        Some(d) if d < 30.0 => "cooling",
        Some(d) if d < 90.0 => "stale",
        Some(d) if d < 180.0 => "very stale",
        _ => "dormant",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{StalenessBracket, StalenessConfig};

    fn default_cfg() -> StalenessConfig {
        StalenessConfig::default()
    }

    const DAY: i64 = 86400;

    #[test]
    fn fresh_scores_zero() {
        let cfg = default_cfg();
        let now = 100_000_000;
        // 10 days old, lands in the 7 day bracket (factor 0.0)
        let score = compute_staleness(500, Some(now - 10 * DAY), now, &cfg);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn cooling_scores_half() {
        let cfg = default_cfg();
        let now = 100_000_000;
        // 45 days old, lands in the 30 day bracket (factor 0.5)
        let score = compute_staleness(1000, Some(now - 45 * DAY), now, &cfg);
        assert_eq!(score, 500.0);
    }

    #[test]
    fn stale_scores_full() {
        let cfg = default_cfg();
        let now = 100_000_000;
        // 120 days old, lands in the 90 day bracket (factor 1.0)
        let score = compute_staleness(2000, Some(now - 120 * DAY), now, &cfg);
        assert_eq!(score, 2000.0);
    }

    #[test]
    fn dormant_uses_default_factor() {
        let cfg = StalenessConfig {
            brackets: vec![StalenessBracket {
                days: 7,
                factor: 0.0,
            }],
            default_factor: 5.0,
        };
        let now = 100_000_000;
        // 1 day old, below the only bracket at 7 days, so default_factor applies
        let score = compute_staleness(100, Some(now - DAY), now, &cfg);
        assert_eq!(score, 500.0);
    }

    #[test]
    fn unknown_age_returns_raw_size() {
        let cfg = default_cfg();
        let score = compute_staleness(4096, None, 1_000_000, &cfg);
        assert_eq!(score, 4096.0);
    }

    #[test]
    fn labels_match_brackets() {
        assert_eq!(staleness_label(Some(true), Some(500.0)), "active");
        assert_eq!(staleness_label(None, None), "unknown");
        assert_eq!(staleness_label(Some(false), Some(1.0)), "fresh");
        assert_eq!(staleness_label(Some(false), Some(10.0)), "cooling");
        assert_eq!(staleness_label(Some(false), Some(45.0)), "stale");
        assert_eq!(staleness_label(Some(false), Some(120.0)), "very stale");
        assert_eq!(staleness_label(Some(false), Some(200.0)), "dormant");
    }
}
