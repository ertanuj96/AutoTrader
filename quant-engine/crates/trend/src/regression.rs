//! Linear regression with R² for trend strength measurement.

pub struct RegressionResult {
    pub slope: f64,
    pub intercept: f64,
    pub r_squared: f64,
}

/// OLS linear regression on windowed price series.
pub fn linear_regression(prices: &[f64]) -> Option<RegressionResult> {
    let n = prices.len();
    if n < 3 {
        return None;
    }

    let nf = n as f64;
    let sx: f64 = (0..n).map(|i| i as f64).sum();
    let sy: f64 = prices.iter().sum();
    let sxy: f64 = prices.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
    let sxx: f64 = (0..n).map(|i| (i as f64).powi(2)).sum();

    let d = nf * sxx - sx * sx;
    if d.abs() < 1e-15 {
        return None;
    }

    let slope = (nf * sxy - sx * sy) / d;
    let intercept = (sy - slope * sx) / nf;

    // R²
    let mean_y = sy / nf;
    let ss_tot: f64 = prices.iter().map(|y| (y - mean_y).powi(2)).sum();
    let ss_res: f64 = prices
        .iter()
        .enumerate()
        .map(|(i, &y)| (y - (intercept + slope * i as f64)).powi(2))
        .sum();

    let r_squared = if ss_tot > 1e-15 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };

    Some(RegressionResult {
        slope,
        intercept,
        r_squared,
    })
}
