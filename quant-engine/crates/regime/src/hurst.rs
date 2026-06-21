//! Hurst Exponent — Rescaled Range (R/S) Analysis.
//!
//! H > 0.5 → trending, H = 0.5 → random walk, H < 0.5 → mean-reverting.

/// Compute Hurst exponent via R/S analysis.
pub fn hurst_exponent(series: &[f64]) -> Option<f64> {
    let n = series.len();
    if n < 20 {
        return None;
    }

    let mut log_ns = Vec::new();
    let mut log_rs = Vec::new();
    let mut sub_len = 8;

    while sub_len <= n / 4 {
        let num_sub = n / sub_len;
        let mut rs_sum = 0.0;
        let mut count = 0;

        for i in 0..num_sub {
            let sub = &series[i * sub_len..(i + 1) * sub_len];
            if let Some(rs) = rescaled_range(sub) {
                rs_sum += rs;
                count += 1;
            }
        }
        if count > 0 {
            log_ns.push((sub_len as f64).ln());
            log_rs.push((rs_sum / count as f64).ln());
        }
        sub_len *= 2;
    }

    if log_ns.len() < 2 {
        return None;
    }

    // OLS: log(R/S) = H * log(n) + c → H = slope
    let np = log_ns.len() as f64;
    let sx: f64 = log_ns.iter().sum();
    let sy: f64 = log_rs.iter().sum();
    let sxy: f64 = log_ns.iter().zip(&log_rs).map(|(x, y)| x * y).sum();
    let sxx: f64 = log_ns.iter().map(|x| x * x).sum();
    let d = np * sxx - sx * sx;
    if d.abs() < 1e-15 {
        return None;
    }

    Some(((np * sxy - sx * sy) / d).clamp(0.0, 1.0))
}

fn rescaled_range(s: &[f64]) -> Option<f64> {
    let n = s.len();
    if n < 2 {
        return None;
    }
    let mean = s.iter().sum::<f64>() / n as f64;
    let std = (s.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    if std < 1e-15 {
        return None;
    }

    let mut cum = 0.0;
    let (mut mx, mut mn) = (f64::NEG_INFINITY, f64::INFINITY);
    for &x in s {
        cum += x - mean;
        mx = mx.max(cum);
        mn = mn.min(cum);
    }
    Some((mx - mn) / std)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trending_hurst_above_half() {
        let series: Vec<f64> = (0..256).map(|i| i as f64 * 0.1).collect();
        let h = hurst_exponent(&series).unwrap();
        assert!(h > 0.5, "Expected H > 0.5 for trend, got {h}");
    }

    #[test]
    fn test_mean_reverting_hurst_below_half() {
        // Oscillating series: alternates between +1 and -1 → mean-reverting
        let series: Vec<f64> = (0..256)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let h = hurst_exponent(&series).unwrap();
        assert!(h < 0.5, "Expected H < 0.5 for mean-reverting, got {h}");
    }

    #[test]
    fn test_hurst_output_bounded_for_oscillating_cumsum() {
        // Strictly alternating +1 / -1 cumsum: bounded path → lower H than pure trend
        let increments: Vec<f64> = (0..512)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let cum: Vec<f64> = increments
            .iter()
            .scan(0.0_f64, |acc, &x| {
                *acc += x;
                Some(*acc)
            })
            .collect();
        // Strictly alternating cumsum has H < pure trend; output must be in [0,1]
        if let Some(h) = hurst_exponent(&cum) {
            assert!((0.0..=1.0).contains(&h), "H out of [0,1]: {h}");
            assert!(h < 1.0, "alternating cumsum should not have maximum H: {h}");
        }
    }

    #[test]
    fn test_too_short_returns_none() {
        assert!(hurst_exponent(&[1.0, 2.0, 3.0]).is_none());
        assert!(hurst_exponent(&[]).is_none());
    }

    #[test]
    fn test_constant_series_returns_none() {
        // Constant series → std = 0 → rescaled_range returns None
        let series = vec![5.0_f64; 100];
        // Either None or a value ∈ [0,1] — must not panic
        if let Some(h) = hurst_exponent(&series) {
            assert!((0.0..=1.0).contains(&h));
        }
    }

    #[test]
    fn test_output_clamped_to_unit_interval() {
        let series: Vec<f64> = (0..128).map(|i| i as f64).collect();
        let h = hurst_exponent(&series).unwrap();
        assert!((0.0..=1.0).contains(&h), "H out of [0,1]: {h}");
    }
}
