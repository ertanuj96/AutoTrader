//! GARCH(1,1) — Generalized Autoregressive Conditional Heteroskedasticity.
//!
//! Models volatility clustering: σ²_t = ω + α·r²_{t-1} + β·σ²_{t-1}
//! where ω > 0, α ≥ 0, β ≥ 0, α + β < 1 (stationarity constraint).

/// GARCH(1,1) model parameters and state.
#[derive(Debug, Clone)]
pub struct Garch {
    pub omega: f64,
    pub alpha: f64,
    pub beta: f64,
    pub current_variance: f64,
    pub long_run_variance: f64,
}

impl Garch {
    pub fn new(omega: f64, alpha: f64, beta: f64) -> Self {
        let lr = if (alpha + beta) < 1.0 { omega / (1.0 - alpha - beta) } else { omega };
        Self { omega, alpha, beta, current_variance: lr, long_run_variance: lr }
    }

    /// Update variance with a new return observation.
    pub fn update(&mut self, return_val: f64) -> f64 {
        self.current_variance = self.omega
            + self.alpha * return_val.powi(2)
            + self.beta * self.current_variance;
        self.current_variance
    }

    /// Fit GARCH(1,1) to a return series using variance targeting + grid optimization.
    pub fn fit(returns: &[f64]) -> Self {
        if returns.len() < 10 {
            return Self::new(0.00001, 0.1, 0.85);
        }

        let sample_var = returns.iter().map(|r| r * r).sum::<f64>() / returns.len() as f64;
        let mut best = Self::new(0.00001, 0.1, 0.85);
        let mut best_ll = f64::NEG_INFINITY;

        // Grid search over (alpha, beta) with variance targeting for omega
        for alpha_i in 1..20 {
            let a = alpha_i as f64 * 0.01;
            for beta_i in 1..20 {
                let b = beta_i as f64 * 0.05;
                if a + b >= 0.999 { continue; }
                let o = sample_var * (1.0 - a - b);
                if o <= 0.0 { continue; }

                let ll = Self::log_likelihood(returns, o, a, b);
                if ll > best_ll {
                    best_ll = ll;
                    best = Self::new(o, a, b);
                }
            }
        }
        // Run model through data to set current_variance
        for &r in returns { best.update(r); }
        best
    }

    fn log_likelihood(returns: &[f64], omega: f64, alpha: f64, beta: f64) -> f64 {
        let lr = omega / (1.0 - alpha - beta);
        let mut var = lr;
        let mut ll = 0.0;
        for &r in returns {
            if var <= 0.0 { return f64::NEG_INFINITY; }
            ll += -0.5 * (var.ln() + r * r / var);
            var = omega + alpha * r * r + beta * var;
        }
        ll
    }

    /// Forecast variance h steps ahead.
    pub fn forecast(&self, h: usize) -> f64 {
        let persistence = self.alpha + self.beta;
        self.long_run_variance
            + persistence.powi(h as i32) * (self.current_variance - self.long_run_variance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sinusoidal_returns(n: usize, amplitude: f64) -> Vec<f64> {
        (0..n).map(|i| (i as f64 * 0.7).sin() * amplitude).collect()
    }

    #[test]
    fn test_update_variance_positive() {
        let mut g = Garch::new(0.00001, 0.1, 0.85);
        for r in sinusoidal_returns(50, 0.01) {
            let v = g.update(r);
            assert!(v > 0.0, "variance must be positive, got {v}");
        }
    }

    #[test]
    fn test_shock_increases_variance() {
        let mut g = Garch::new(0.00001, 0.1, 0.85);
        let lr = g.long_run_variance;
        let v = g.update(0.05); // large 5% return
        assert!(v > lr, "large shock should raise variance above long-run: {v} vs {lr}");
    }

    #[test]
    fn test_stationarity_constraint_after_fit() {
        let returns = sinusoidal_returns(200, 0.01);
        let g = Garch::fit(&returns);
        assert!(g.alpha + g.beta < 1.0,
            "stationarity violated: α+β = {}", g.alpha + g.beta);
    }

    #[test]
    fn test_fit_omega_positive() {
        let g = Garch::fit(&sinusoidal_returns(200, 0.01));
        assert!(g.omega > 0.0, "ω must be positive, got {}", g.omega);
    }

    #[test]
    fn test_forecast_reverts_to_long_run() {
        let g = Garch::fit(&sinusoidal_returns(200, 0.01));
        // Long-horizon forecast should approach long_run_variance
        let f_far = g.forecast(100);
        let f_near = g.forecast(1);
        let lr = g.long_run_variance;
        assert!((f_far - lr).abs() < (f_near - lr).abs(),
            "far forecast {f_far} should be closer to LR {lr} than near {f_near}");
    }

    #[test]
    fn test_forecast_positive() {
        let g = Garch::fit(&sinusoidal_returns(200, 0.005));
        for h in [1, 5, 10, 50] {
            assert!(g.forecast(h) > 0.0, "forecast({h}) must be positive");
        }
    }

    #[test]
    fn test_fit_too_short_uses_defaults() {
        let g = Garch::fit(&[0.01, 0.02]);
        assert!(g.alpha + g.beta < 1.0);
        assert!(g.omega > 0.0);
    }
}

