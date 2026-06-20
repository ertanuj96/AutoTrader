//! Bayesian Online Changepoint Detection (Adams & MacKay, 2007).
//!
//! Uses Normal-Inverse-Gamma conjugate prior with Student-t predictive distribution.
//! Run-length beliefs are maintained in log-space to avoid underflow.
//! O(N) memory capped at MAX_RUN_LENGTH steps for safety.

use statrs::distribution::{Continuous, StudentsT};

/// Maximum run length tracked; hypotheses beyond this are folded into the cap bucket.
const MAX_RUN_LENGTH: usize = 500;

// ─── Normal-Inverse-Gamma sufficient statistics ───────────────────────────────

/// Sufficient statistics for the NIG conjugate prior.
///
/// Prior: μ₀ = 0, κ₀ = 1, α₀ = 2, β₀ = 0.001
/// Predictive distribution: Student-t with df = 2α, location = μ, scale² = β(κ+1)/(ακ).
#[derive(Clone)]
struct NigParams {
    mu: f64,
    kappa: f64,
    alpha: f64,
    beta: f64,
}

impl NigParams {
    /// Standard NIG prior (zero mean, low precision, vague).
    fn prior() -> Self {
        Self { mu: 0.0, kappa: 1.0, alpha: 2.0, beta: 0.001 }
    }

    /// Conjugate Bayesian update with one new observation.
    fn update(&self, x: f64) -> Self {
        let kappa_new = self.kappa + 1.0;
        let mu_new = (self.kappa * self.mu + x) / kappa_new;
        let alpha_new = self.alpha + 0.5;
        let beta_new =
            self.beta + (self.kappa * (x - self.mu).powi(2)) / (2.0 * kappa_new);
        Self { mu: mu_new, kappa: kappa_new, alpha: alpha_new, beta: beta_new }
    }

    /// Log predictive probability p(x | data so far) under Student-t marginal.
    ///
    /// df = 2α,  location = μ,  scale² = β(κ+1)/(α·κ)
    fn log_pred(&self, x: f64) -> f64 {
        let df = 2.0 * self.alpha;
        let scale_sq =
            (self.beta * (self.kappa + 1.0)) / (self.alpha * self.kappa);
        let scale = scale_sq.sqrt().max(1e-12);
        match StudentsT::new(self.mu, scale, df) {
            Ok(dist) => dist.ln_pdf(x),
            Err(_) => f64::NEG_INFINITY,
        }
    }
}

// ─── Numerically stable log-sum-exp ──────────────────────────────────────────

fn log_sum_exp(vals: &[f64]) -> f64 {
    if vals.is_empty() { return f64::NEG_INFINITY; }
    let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if !max.is_finite() { return f64::NEG_INFINITY; }
    max + vals.iter().map(|&v| (v - max).exp()).sum::<f64>().ln()
}

// ─── Main detector ────────────────────────────────────────────────────────────

/// Bayesian Online Changepoint Detector (Adams & MacKay 2007).
///
/// Maintains a posterior distribution P(run_length = r | data) in log-space.
/// After each observation:
/// - `p_changepoint` is updated with P(changepoint at this step).
/// - `update()` returns P(run_length ≤ horizon) as the P(reversal within horizon).
pub struct ChangePointDetector {
    hazard_rate: f64,
    horizon: usize,

    /// Log posterior over run lengths: `log_probs[r]` = log P(r_t = r | x_{1:t}).
    log_probs: Vec<f64>,

    /// NIG sufficient statistics for each run-length hypothesis.
    nig: Vec<NigParams>,

    /// P(changepoint occurred at this observation) — exposed for engine use.
    pub p_changepoint: f64,

    /// Number of observations processed so far.
    pub n_obs: usize,
}

impl ChangePointDetector {
    /// Create a new detector.
    ///
    /// * `hazard_rate` – constant prior probability of a changepoint per step ∈ (0, 1).
    /// * `horizon`     – number of future bars for P(reversal) aggregation.
    pub fn new(hazard_rate: f64, horizon: usize) -> Self {
        assert!(
            hazard_rate > 0.0 && hazard_rate < 1.0,
            "hazard_rate must be in (0, 1), got {hazard_rate}"
        );
        Self {
            hazard_rate,
            horizon,
            log_probs: vec![0.0], // log P(r₀ = 0) = log 1 = 0
            nig: vec![NigParams::prior()],
            p_changepoint: 0.0,
            n_obs: 0,
        }
    }

    /// Process one observation and return P(reversal within `horizon` bars).
    pub fn update(&mut self, x: f64) -> f64 {
        let h = self.hazard_rate;
        let log_h = h.ln();
        let log_1mh = (1.0 - h).ln();
        let n = self.log_probs.len();

        // ── 1. Compute log-predictive under each hypothesis ──────────────────
        let log_preds: Vec<f64> = self.nig.iter().map(|nig| nig.log_pred(x)).collect();

        // ── 2. Changepoint mass: Σ_r P(r) · p(x|r) · H ─────────────────────
        let cp_terms: Vec<f64> = (0..n)
            .map(|r| self.log_probs[r] + log_preds[r])
            .collect();
        let log_cp_total = log_sum_exp(&cp_terms) + log_h;

        // ── 3. Build new hypothesis vector (capped at MAX_RUN_LENGTH + 1) ───
        let new_n = (n + 1).min(MAX_RUN_LENGTH + 1);
        let mut new_log_probs = vec![f64::NEG_INFINITY; new_n];
        let mut new_nig: Vec<NigParams> = Vec::with_capacity(new_n);

        // r = 0: changepoint hypothesis — reset to prior
        new_log_probs[0] = log_cp_total;
        new_nig.push(NigParams::prior());

        // r = 1 .. min(n, MAX_RUN_LENGTH): growth hypotheses
        let copy_up_to = n.min(MAX_RUN_LENGTH);
        for r in 0..copy_up_to {
            new_log_probs[r + 1] = self.log_probs[r] + log_preds[r] + log_1mh;
            new_nig.push(self.nig[r].update(x));
        }

        // If at capacity, fold overflow into the last bucket
        if n > MAX_RUN_LENGTH {
            let overflow: Vec<f64> = (MAX_RUN_LENGTH..n)
                .map(|r| self.log_probs[r] + log_preds[r] + log_1mh)
                .collect();
            let lse = log_sum_exp(&overflow);
            new_log_probs[MAX_RUN_LENGTH] =
                log_sum_exp(&[new_log_probs[MAX_RUN_LENGTH], lse]);
            // NIG for this bucket stays the one we already pushed (approximate)
        }

        // ── 4. Normalise ─────────────────────────────────────────────────────
        let log_evidence = log_sum_exp(&new_log_probs);
        for lp in new_log_probs.iter_mut() {
            *lp -= log_evidence;
        }

        // ── 5. Commit & expose public fields ─────────────────────────────────
        self.p_changepoint = new_log_probs[0].exp().clamp(0.0, 1.0);
        self.log_probs = new_log_probs;
        self.nig = new_nig;
        self.n_obs += 1;

        // ── 6. P(reversal within horizon) = Σ P(r ≤ horizon) ────────────────
        let p_rev: f64 = self.log_probs[..self.horizon.min(self.log_probs.len())]
            .iter()
            .map(|&lp| lp.exp())
            .sum();
        p_rev.clamp(0.0, 1.0)
    }

    /// Index (run length) of the hypothesis with highest posterior probability.
    pub fn most_probable_run_length(&self) -> usize {
        self.log_probs
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Expected run length E[r] = Σ r · P(r).
    pub fn expected_run_length(&self) -> f64 {
        self.log_probs
            .iter()
            .enumerate()
            .map(|(r, &lp)| r as f64 * lp.exp())
            .sum()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outputs_in_unit_interval() {
        let mut d = ChangePointDetector::new(0.05, 5);
        for i in 0..50_usize {
            // Inject occasional shocks to keep it interesting
            let x = if i % 17 == 0 { 10.0 } else { 0.0 };
            let p = d.update(x);
            assert!(
                p >= 0.0 && p <= 1.0,
                "p={p} out of [0,1] at step {i}"
            );
        }
    }

    #[test]
    fn run_length_probs_sum_to_one() {
        let mut d = ChangePointDetector::new(0.05, 10);
        for i in 0..30_usize {
            d.update(i as f64 * 0.1);
        }
        let total: f64 = d.log_probs.iter().map(|&lp| lp.exp()).sum();
        assert!(
            (total - 1.0).abs() < 1e-6,
            "probs should sum to 1, got {total}"
        );
    }

    #[test]
    fn shock_shifts_run_length_distribution() {
        let mut d = ChangePointDetector::new(0.01, 5);
        // Warm up with stable near-zero returns
        for _ in 0..50 {
            d.update(0.0);
        }
        let mrl_before = d.most_probable_run_length();
        // Large shock — the run-length distribution should collapse toward short runs
        for _ in 0..5 {
            d.update(100.0);
        }
        let mrl_after = d.most_probable_run_length();
        // After shock the expected most-probable run length should be shorter or
        // the expected run length should drop — the regime has reset.
        let erl_after = d.expected_run_length();
        // Either most probable run length drops OR expected run length is short (≤30)
        assert!(
            mrl_after < mrl_before || erl_after < 30.0,
            "shock should shift toward shorter runs: mrl {mrl_before}→{mrl_after}, erl={erl_after:.2}"
        );
    }

    #[test]
    fn stable_regime_grows_run_length() {
        let mut d = ChangePointDetector::new(0.005, 5);
        // Feed identical signal — run length should grow
        for _ in 0..40 {
            d.update(0.001);
        }
        let mrl = d.most_probable_run_length();
        assert!(
            mrl > 5,
            "expected long run after 40 stable observations, got mrl={mrl}"
        );
    }

    #[test]
    fn no_panic_on_600_observations() {
        let mut d = ChangePointDetector::new(0.01, 10);
        for i in 0..600_usize {
            // Inject one shock at midpoint to test MAX_RUN_LENGTH logic
            let x = if i == 300 { 50.0 } else { 0.0 };
            let p = d.update(x);
            assert!(
                p >= 0.0 && p <= 1.0,
                "p={p} out of [0,1] at step {i}"
            );
        }
        // Memory is bounded by MAX_RUN_LENGTH + 1
        assert!(d.log_probs.len() <= MAX_RUN_LENGTH + 1);
    }
}
