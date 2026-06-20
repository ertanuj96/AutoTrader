//! Volatility forecasting using realized vol and GARCH output.

/// Compute annualized realized volatility from returns.
pub fn realized_vol(returns: &[f64], annualization_factor: f64) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let var = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (returns.len() - 1) as f64;
    var.sqrt() * annualization_factor.sqrt()
}
