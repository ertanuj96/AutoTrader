//! Bayesian fusion — combine regime, trend, reversal, and movement into a single confidence score.
//!
//! Uses Bayesian updating: P(signal_correct | evidence) ∝ P(evidence | correct) * P(correct)
//! Each engine contributes a likelihood ratio that updates the prior.

use autotrader_common::{RegimeEvent, TrendEvent, ReversalEvent, MovementEvent, MarketRegime, TrendDirection};

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
    let range_ratio = if movement.atr > 0.0 { movement.expected_range / movement.atr } else { 1.0 };
    let movement_lr = if range_ratio > 1.2 { 1.2 } else if range_ratio < 0.5 { 0.6 } else { 1.0 };
    posterior = update_bayesian(posterior, movement_lr);
    let movement_contrib = movement_lr - 1.0;

    (posterior.clamp(0.0, 1.0), regime_contrib, trend_contrib, reversal_contrib, movement_contrib)
}

/// Bayesian update: P(H|E) = P(E|H)*P(H) / [P(E|H)*P(H) + P(E|¬H)*P(¬H)]
/// where likelihood_ratio = P(E|H) / P(E|¬H)
fn update_bayesian(prior: f64, likelihood_ratio: f64) -> f64 {
    let odds = prior / (1.0 - prior).max(1e-10);
    let posterior_odds = odds * likelihood_ratio;
    posterior_odds / (1.0 + posterior_odds)
}

