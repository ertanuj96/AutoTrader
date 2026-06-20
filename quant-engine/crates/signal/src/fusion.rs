//! Bayesian fusion — combine regime, trend, reversal, and movement into a single confidence score.
//!
//! Uses Bayesian updating: P(signal_correct | evidence) ∝ P(evidence | correct) * P(correct)
//! Each engine contributes a likelihood ratio that updates the prior.

use autotrader_common::{
    MarketRegime, MovementEvent, RegimeEvent, ReversalEvent, TrendDirection, TrendEvent,
};

/// Fuse all engine outputs into a single confidence score [0.0, 1.0].
pub fn bayesian_fusion(
    regime: &RegimeEvent,
    trend: &TrendEvent,
    reversal: &ReversalEvent,
    movement: &MovementEvent,
    prior: f64,
) -> (f64, f64, f64, f64, f64) {
    let mut posterior = prior;

    // Regime contribution: trending regime boosts confidence, high-vol reduces it
    let regime_lr = match regime.regime {
        MarketRegime::Trending => 1.0 + regime.p_trending * 0.5,
        MarketRegime::Ranging => 0.7,
        MarketRegime::HighVolatility => 0.5,
    };
    posterior = update_bayesian(posterior, regime_lr);
    let regime_contrib = regime_lr - 1.0;

    // Trend contribution: strong trend with significance boosts confidence
    let trend_lr = match trend.direction {
        TrendDirection::StrongUp | TrendDirection::StrongDown => 1.0 + trend.strength * 0.4,
        TrendDirection::WeakUp | TrendDirection::WeakDown => 1.0 + trend.strength * 0.2,
        TrendDirection::Neutral => 0.8,
    };
    posterior = update_bayesian(posterior, trend_lr);
    let trend_contrib = trend_lr - 1.0;

    // Reversal contribution: high reversal probability reduces confidence
    let reversal_lr = 1.0 - reversal.p_reversal * 0.5;
    posterior = update_bayesian(posterior, reversal_lr.max(0.3));
    let reversal_contrib = reversal_lr - 1.0;

    // Movement contribution: sufficient expected range (vs cost) boosts confidence
    let range_ratio = if movement.atr > 0.0 {
        movement.expected_range / movement.atr
    } else {
        1.0
    };
    let movement_lr = if range_ratio > 1.2 {
        1.2
    } else if range_ratio < 0.5 {
        0.6
    } else {
        1.0
    };
    posterior = update_bayesian(posterior, movement_lr);
    let movement_contrib = movement_lr - 1.0;

    (
        posterior.clamp(0.0, 1.0),
        regime_contrib,
        trend_contrib,
        reversal_contrib,
        movement_contrib,
    )
}

/// Bayesian update: P(H|E) = P(E|H)*P(H) / [P(E|H)*P(H) + P(E|¬H)*P(¬H)]
/// where likelihood_ratio = P(E|H) / P(E|¬H)
fn update_bayesian(prior: f64, likelihood_ratio: f64) -> f64 {
    let odds = prior / (1.0 - prior).max(1e-10);
    let posterior_odds = odds * likelihood_ratio;
    posterior_odds / (1.0 + posterior_odds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use autotrader_common::{MarketRegime, Timeframe, TrendDirection};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_regime(regime: MarketRegime, p_trending: f64) -> RegimeEvent {
        RegimeEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: "NIFTY".into(),
            timeframe: Timeframe::Min5,
            p_trending,
            p_ranging: 1.0 - p_trending,
            p_high_vol: 0.0,
            hurst_exponent: 0.6,
            garch_variance: 0.0001,
            regime,
        }
    }

    fn make_trend(direction: TrendDirection, strength: f64) -> TrendEvent {
        TrendEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: "NIFTY".into(),
            timeframe: Timeframe::Min5,
            direction,
            strength,
            kalman_slope: 0.001,
            kalman_uncertainty: 0.0001,
            regression_r_squared: 0.8,
            mann_kendall_p_value: 0.02,
        }
    }

    fn make_reversal(p_reversal: f64) -> ReversalEvent {
        ReversalEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: "NIFTY".into(),
            timeframe: Timeframe::Min5,
            p_reversal,
            horizon_bars: 5,
            cusum_triggered: false,
            oi_divergence: 0.0,
            volume_divergence: 0.0,
        }
    }

    fn make_movement(expected_range: f64, atr: f64) -> MovementEvent {
        MovementEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: "NIFTY".into(),
            timeframe: Timeframe::Min5,
            expected_range,
            atr,
            garch_vol: 0.01,
            realized_vol: 0.012,
            implied_move: None,
        }
    }

    #[test]
    fn test_output_always_in_unit_interval() {
        let r = make_regime(MarketRegime::Trending, 0.8);
        let t = make_trend(TrendDirection::StrongUp, 0.9);
        let rev = make_reversal(0.1);
        let m = make_movement(100.0, 50.0);
        let (conf, rc, tc, revc, mc) = bayesian_fusion(&r, &t, &rev, &m, 0.5);
        assert!((0.0..=1.0).contains(&conf), "conf = {conf}");
        // Contributions are unbounded but fusion output must be clamped
        let _ = (rc, tc, revc, mc);
    }

    #[test]
    fn test_trending_regime_strong_trend_boosts_confidence() {
        let r_good = make_regime(MarketRegime::Trending, 0.9);
        let r_bad = make_regime(MarketRegime::Ranging, 0.1);
        let t = make_trend(TrendDirection::StrongUp, 0.9);
        let rev = make_reversal(0.1);
        let m = make_movement(120.0, 80.0);
        let (conf_good, _, _, _, _) = bayesian_fusion(&r_good, &t, &rev, &m, 0.5);
        let (conf_bad, _, _, _, _) = bayesian_fusion(&r_bad, &t, &rev, &m, 0.5);
        assert!(
            conf_good > conf_bad,
            "trending {conf_good} should beat ranging {conf_bad}"
        );
    }

    #[test]
    fn test_high_vol_regime_reduces_confidence() {
        let r_normal = make_regime(MarketRegime::Trending, 0.8);
        let r_hv = make_regime(MarketRegime::HighVolatility, 0.0);
        let t = make_trend(TrendDirection::StrongUp, 0.9);
        let rev = make_reversal(0.1);
        let m = make_movement(100.0, 50.0);
        let (conf_normal, _, _, _, _) = bayesian_fusion(&r_normal, &t, &rev, &m, 0.5);
        let (conf_hv, _, _, _, _) = bayesian_fusion(&r_hv, &t, &rev, &m, 0.5);
        assert!(
            conf_normal > conf_hv,
            "HV should reduce confidence: {conf_hv} vs {conf_normal}"
        );
    }

    #[test]
    fn test_high_reversal_prob_lowers_confidence() {
        let r = make_regime(MarketRegime::Trending, 0.8);
        let t = make_trend(TrendDirection::StrongUp, 0.9);
        let m = make_movement(100.0, 50.0);
        let (conf_low_rev, _, _, _, _) = bayesian_fusion(&r, &t, &make_reversal(0.05), &m, 0.5);
        let (conf_high_rev, _, _, _, _) = bayesian_fusion(&r, &t, &make_reversal(0.9), &m, 0.5);
        assert!(
            conf_low_rev > conf_high_rev,
            "low rev {conf_low_rev} should beat high rev {conf_high_rev}"
        );
    }

    #[test]
    fn test_prior_zero_stays_zero() {
        let r = make_regime(MarketRegime::Trending, 0.9);
        let t = make_trend(TrendDirection::StrongUp, 1.0);
        let rev = make_reversal(0.0);
        let m = make_movement(200.0, 50.0);
        let (conf, _, _, _, _) = bayesian_fusion(&r, &t, &rev, &m, 0.0);
        assert!(
            conf < 0.01,
            "zero prior should yield near-zero posterior: {conf}"
        );
    }
}
