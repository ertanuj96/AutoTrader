//! Option-implied expected move from ATM straddle pricing.

/// Compute expected move from ATM straddle (call + put premium).
/// expected_move ≈ straddle_price * 0.85 (rule of thumb for 1 SD move).
pub fn implied_expected_move(atm_call_premium: f64, atm_put_premium: f64) -> f64 {
    (atm_call_premium + atm_put_premium) * 0.85
}

