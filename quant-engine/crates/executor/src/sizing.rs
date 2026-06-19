//! Kelly criterion for optimal position sizing.
//!
//! f* = (p * b - q) / b  where:
//!   p = probability of win, q = 1 - p, b = payoff ratio (win/loss)
//! Capped at max_fraction to prevent over-leveraging.

/// Compute optimal Kelly fraction, capped at max_fraction.
pub fn kelly_fraction(win_probability: f64, payoff_ratio: f64, max_fraction: f64) -> f64 {
    if payoff_ratio <= 0.0 || win_probability <= 0.0 || win_probability >= 1.0 {
        return 0.0;
    }
    let q = 1.0 - win_probability;
    let f = (win_probability * payoff_ratio - q) / payoff_ratio;
    f.max(0.0).min(max_fraction)
}

/// Convert Kelly fraction to lot size given capital and lot value.
pub fn kelly_lots(fraction: f64, capital: f64, lot_value: f64, max_lots: u32) -> u32 {
    if lot_value <= 0.0 { return 0; }
    let lots = (fraction * capital / lot_value).floor() as u32;
    lots.min(max_lots).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ── Unit tests ──

    #[test]
    fn test_positive_edge_with_cap() {
        // 60% win, 1.5:1 payoff → f* ≈ 0.333 → capped at 0.25
        let f = kelly_fraction(0.6, 1.5, 0.25);
        assert!((f - 0.25).abs() < 0.001, "f = {f}");
    }

    #[test]
    fn test_no_edge_returns_zero() {
        // 50% win, 1:1 payoff → f* = 0
        let f = kelly_fraction(0.5, 1.0, 0.25);
        assert!(f.abs() < 1e-10, "f = {f}");
    }

    #[test]
    fn test_negative_edge_returns_zero() {
        // 40% win, 1:1 payoff → negative Kelly → 0
        let f = kelly_fraction(0.4, 1.0, 0.25);
        assert_eq!(f, 0.0);
    }

    #[test]
    fn test_zero_win_prob_returns_zero() {
        assert_eq!(kelly_fraction(0.0, 2.0, 0.5), 0.0);
    }

    #[test]
    fn test_one_win_prob_returns_max() {
        let f = kelly_fraction(1.0, 2.0, 0.25);
        // f* = (1.0*2 - 0)/2 = 1.0 → capped at 0.25
        // But win_prob >= 1.0 is guarded → returns 0
        assert_eq!(f, 0.0);
    }

    #[test]
    fn test_zero_payoff_returns_zero() {
        assert_eq!(kelly_fraction(0.6, 0.0, 0.25), 0.0);
    }

    #[test]
    fn test_lots_zero_when_no_capital() {
        assert_eq!(kelly_lots(0.25, 0.0, 100.0, 10), 0);
    }

    #[test]
    fn test_lots_capped_at_max() {
        // 25% of 1,000,000 / 10,000 per lot = 25 lots → capped at 10
        let lots = kelly_lots(0.25, 1_000_000.0, 10_000.0, 10);
        assert_eq!(lots, 10);
    }

    #[test]
    fn test_lots_scales_with_capital() {
        let lots_small = kelly_lots(0.2, 100_000.0, 10_000.0, 100);
        let lots_large = kelly_lots(0.2, 500_000.0, 10_000.0, 100);
        assert!(lots_large >= lots_small);
    }

    // ── Property-based tests ──

    proptest! {
        #[test]
        fn prop_kelly_fraction_in_range(
            win_prob in 0.01f64..0.99,
            payoff in 0.01f64..10.0,
            max_frac in 0.01f64..1.0,
        ) {
            let f = kelly_fraction(win_prob, payoff, max_frac);
            prop_assert!(f >= 0.0, "fraction must be non-negative: {f}");
            prop_assert!(f <= max_frac + 1e-10, "fraction {f} exceeds max {max_frac}");
        }

        #[test]
        fn prop_kelly_lots_never_exceeds_max(
            fraction in 0.0f64..1.0,
            capital in 0.0f64..10_000_000.0,
            lot_value in 1.0f64..100_000.0,
            max_lots in 0u32..1000,
        ) {
            let lots = kelly_lots(fraction, capital, lot_value, max_lots);
            prop_assert!(lots <= max_lots, "lots {lots} > max {max_lots}");
        }
    }
}

