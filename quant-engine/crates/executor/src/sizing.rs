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

    #[test]
    fn test_kelly_basic() {
        // 60% win rate, 1.5:1 payoff → f* = (0.6*1.5 - 0.4)/1.5 = 0.333
        let f = kelly_fraction(0.6, 1.5, 0.25);
        assert!((f - 0.25).abs() < 0.01); // Capped at 0.25
    }

    #[test]
    fn test_kelly_no_edge() {
        // 50% win rate, 1:1 payoff → f* = 0 (no edge)
        let f = kelly_fraction(0.5, 1.0, 0.25);
        assert!(f.abs() < 0.01);
    }
}

