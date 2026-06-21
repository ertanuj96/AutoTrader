//! Divergence detection — Price vs OI/Volume/Breadth.

/// Compute divergence score between price and a secondary series (e.g., OI).
/// Returns [-1.0, 1.0]: positive = bullish divergence, negative = bearish divergence.
pub fn divergence_score(prices: &[f64], secondary: &[f64]) -> f64 {
    if prices.len() < 5 || secondary.len() < 5 {
        return 0.0;
    }
    let n = prices.len().min(secondary.len());
    let p = &prices[prices.len() - n..];
    let s = &secondary[secondary.len() - n..];

    let p_slope = simple_slope(p);
    let s_slope = simple_slope(s);

    // Divergence = opposite slopes
    if p_slope.abs() < 1e-10 || s_slope.abs() < 1e-10 {
        return 0.0;
    }
    let agreement = p_slope.signum() * s_slope.signum();
    -agreement * (p_slope.abs().min(1.0) + s_slope.abs().min(1.0)) / 2.0
}

fn simple_slope(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }
    let first_half: f64 = data[..data.len() / 2].iter().sum::<f64>() / (data.len() / 2) as f64;
    let second_half: f64 =
        data[data.len() / 2..].iter().sum::<f64>() / (data.len() - data.len() / 2) as f64;
    second_half - first_half
}
