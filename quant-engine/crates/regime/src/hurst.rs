//! Hurst Exponent — Rescaled Range (R/S) Analysis.
//!
//! H > 0.5 → trending, H = 0.5 → random walk, H < 0.5 → mean-reverting.

/// Compute Hurst exponent via R/S analysis.
pub fn hurst_exponent(series: &[f64]) -> Option<f64> {
    let n = series.len();
    if n < 20 { return None; }

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

    if log_ns.len() < 2 { return None; }

    // OLS: log(R/S) = H * log(n) + c → H = slope
    let np = log_ns.len() as f64;
    let sx: f64 = log_ns.iter().sum();
    let sy: f64 = log_rs.iter().sum();
    let sxy: f64 = log_ns.iter().zip(&log_rs).map(|(x, y)| x * y).sum();
    let sxx: f64 = log_ns.iter().map(|x| x * x).sum();
    let d = np * sxx - sx * sx;
    if d.abs() < 1e-15 { return None; }

    Some(((np * sxy - sx * sy) / d).clamp(0.0, 1.0))
}

fn rescaled_range(s: &[f64]) -> Option<f64> {
    let n = s.len();
    if n < 2 { return None; }
    let mean = s.iter().sum::<f64>() / n as f64;
    let std = (s.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    if std < 1e-15 { return None; }

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
    fn test_trending_hurst() {
        let series: Vec<f64> = (0..256).map(|i| i as f64 * 0.1).collect();
        let h = hurst_exponent(&series).unwrap();
        assert!(h > 0.5, "Hurst for trend: {h}");
    }
}

