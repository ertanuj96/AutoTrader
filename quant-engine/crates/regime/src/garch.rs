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

    #[test]
    fn test_garch_update() {
        let mut g = Garch::new(0.00001, 0.1, 0.85);
        let v = g.update(0.02); // 2% return
        assert!(v > 0.0);
        assert!(v > g.long_run_variance); // Shock should increase variance
    }

    #[test]
    fn test_garch_fit() {
        let returns: Vec<f64> = (0..200).map(|i| ((i as f64 * 0.7).sin()) * 0.01).collect();
        let g = Garch::fit(&returns);
        assert!(g.omega > 0.0);
        assert!(g.alpha + g.beta < 1.0);
    }
}

