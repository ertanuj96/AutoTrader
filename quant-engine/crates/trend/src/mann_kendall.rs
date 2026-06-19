//! Mann-Kendall non-parametric trend significance test.
//!
//! Tests H0: no monotonic trend vs H1: monotonic trend exists.
//! Returns p-value; < 0.05 = statistically significant trend.

use statrs::distribution::{ContinuousCDF, Normal};

pub struct MannKendallResult {
    pub s_statistic: f64,
    pub z_score: f64,
    pub p_value: f64,
    pub trend_significant: bool,
}

/// Run Mann-Kendall test on a time series.
pub fn mann_kendall(data: &[f64], alpha: f64) -> Option<MannKendallResult> {
    let n = data.len();
    if n < 4 { return None; }

    // S = Σ sign(x_j - x_i) for all i < j
    let mut s: i64 = 0;
    for i in 0..n - 1 {
        for j in i + 1..n {
            let diff = data[j] - data[i];
            if diff > 0.0 { s += 1; }
            else if diff < 0.0 { s -= 1; }
        }
    }

    // Variance of S (no ties correction for simplicity)
    let nf = n as f64;
    let var_s = nf * (nf - 1.0) * (2.0 * nf + 5.0) / 18.0;

    // Z-score with continuity correction
    let z = if s > 0 { (s as f64 - 1.0) / var_s.sqrt() }
    else if s < 0 { (s as f64 + 1.0) / var_s.sqrt() }
    else { 0.0 };

    let normal = Normal::new(0.0, 1.0).ok()?;
    let p = 2.0 * (1.0 - normal.cdf(z.abs()));

    Some(MannKendallResult {
        s_statistic: s as f64,
        z_score: z,
        p_value: p,
        trend_significant: p < alpha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strong_uptrend_significant() {
        let data: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let r = mann_kendall(&data, 0.05).unwrap();
        assert!(r.trend_significant, "p = {}", r.p_value);
        assert!(r.s_statistic > 0.0);
        assert!(r.z_score > 0.0);
    }

    #[test]
    fn test_strong_downtrend_significant() {
        let data: Vec<f64> = (0..50).map(|i| 50.0 - i as f64).collect();
        let r = mann_kendall(&data, 0.05).unwrap();
        assert!(r.trend_significant, "p = {}", r.p_value);
        assert!(r.s_statistic < 0.0);
        assert!(r.z_score < 0.0);
    }

    #[test]
    fn test_constant_series_not_significant() {
        let data = vec![5.0_f64; 30];
        let r = mann_kendall(&data, 0.05).unwrap();
        assert!(!r.trend_significant, "p = {}", r.p_value);
        assert_eq!(r.s_statistic, 0.0);
    }

    #[test]
    fn test_too_short_returns_none() {
        assert!(mann_kendall(&[1.0, 2.0, 3.0], 0.05).is_none());
        assert!(mann_kendall(&[], 0.05).is_none());
    }

    #[test]
    fn test_p_value_in_unit_interval() {
        let data: Vec<f64> = (0..20).map(|i| (i as f64 * 0.5).sin()).collect();
        let r = mann_kendall(&data, 0.05).unwrap();
        assert!(r.p_value >= 0.0 && r.p_value <= 1.0, "p = {}", r.p_value);
    }
}

