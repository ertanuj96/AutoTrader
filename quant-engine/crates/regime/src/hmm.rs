//! Hidden Markov Model with Baum-Welch parameter estimation and Viterbi decoding.
//!
//! The HMM models the market as transitioning between K hidden states (default 3):
//!   State 0: Trending — directional moves, high autocorrelation
//!   State 1: Ranging — mean-reverting, low net displacement
//!   State 2: High Volatility — large moves in both directions
//!
//! Observations are log-returns, modeled as Gaussian emissions per state.
//!
//! Reference: Rabiner, L.R. "A Tutorial on Hidden Markov Models" (1989)
//! Reference: Hamilton, J.D. "A New Approach to the Economic Analysis of
//!            Nonstationary Time Series" (1989)

use nalgebra::{DMatrix, DVector};
use std::f64::consts::PI;

/// A Gaussian Hidden Markov Model with K states.
#[derive(Debug, Clone)]
pub struct GaussianHMM {
    /// Number of hidden states.
    pub k: usize,
    /// Transition probability matrix A[i][j] = P(state_j | state_i). Shape: K×K.
    pub transition: DMatrix<f64>,
    /// Initial state distribution π. Length: K.
    pub initial: DVector<f64>,
    /// Emission means μ_k for each state. Length: K.
    pub means: DVector<f64>,
    /// Emission variances σ²_k for each state. Length: K.
    pub variances: DVector<f64>,
    /// Log-likelihood of the last fit.
    pub log_likelihood: f64,
}

impl GaussianHMM {
    /// Create a new HMM with K states and reasonable initial parameters.
    pub fn new(k: usize) -> Self {
        // Uniform initial distribution
        let initial = DVector::from_element(k, 1.0 / k as f64);

        // Slightly sticky transition matrix (high self-transition probability)
        // Regimes tend to persist — a trending market stays trending for a while.
        let mut transition = DMatrix::from_element(k, k, 0.05 / (k - 1) as f64);
        for i in 0..k {
            transition[(i, i)] = 0.95;
            // Normalize row
            let row_sum: f64 = (0..k).map(|j| transition[(i, j)]).sum();
            for j in 0..k {
                transition[(i, j)] /= row_sum;
            }
        }

        // Initial emission parameters (will be refined by Baum-Welch)
        // State 0 (trending): positive mean, moderate variance
        // State 1 (ranging): near-zero mean, low variance
        // State 2 (high-vol): near-zero mean, high variance
        let means = DVector::from_vec(vec![0.001, 0.0, 0.0]);
        let variances = DVector::from_vec(vec![0.0001, 0.00005, 0.0005]);

        Self {
            k,
            transition,
            initial,
            means,
            variances,
            log_likelihood: f64::NEG_INFINITY,
        }
    }

    /// Gaussian PDF: P(x | μ, σ²)
    fn gaussian_pdf(x: f64, mean: f64, variance: f64) -> f64 {
        let denom = (2.0 * PI * variance).sqrt();
        let exponent = -0.5 * (x - mean).powi(2) / variance;
        exponent.exp() / denom
    }

    /// Compute emission probabilities B[t][k] = P(obs[t] | state=k) for all t, k.
    fn emission_matrix(&self, observations: &[f64]) -> DMatrix<f64> {
        let t = observations.len();
        let mut b = DMatrix::zeros(t, self.k);
        for (i, &obs) in observations.iter().enumerate() {
            for j in 0..self.k {
                b[(i, j)] = Self::gaussian_pdf(obs, self.means[j], self.variances[j])
                    .max(1e-300); // Floor to avoid log(0)
            }
        }
        b
    }

    /// Forward algorithm — compute α[t][k] = P(obs[0..=t], state_t=k | λ).
    /// Uses log-space scaling for numerical stability.
    fn forward(&self, observations: &[f64]) -> (DMatrix<f64>, Vec<f64>) {
        let t_len = observations.len();
        let b = self.emission_matrix(observations);
        let mut alpha = DMatrix::zeros(t_len, self.k);
        let mut scales = vec![0.0_f64; t_len];

        // Initialization: α[0][k] = π[k] * B[0][k]
        for k in 0..self.k {
            alpha[(0, k)] = self.initial[k] * b[(0, k)];
        }
        scales[0] = alpha.row(0).sum();
        if scales[0] > 0.0 {
            for k in 0..self.k {
                alpha[(0, k)] /= scales[0];
            }
        }

        // Induction: α[t][j] = Σ_i(α[t-1][i] * A[i][j]) * B[t][j]
        for t in 1..t_len {
            for j in 0..self.k {
                let mut sum = 0.0;
                for i in 0..self.k {
                    sum += alpha[(t - 1, i)] * self.transition[(i, j)];
                }
                alpha[(t, j)] = sum * b[(t, j)];
            }
            scales[t] = alpha.row(t).sum();
            if scales[t] > 0.0 {
                for k in 0..self.k {
                    alpha[(t, k)] /= scales[t];
                }
            }
        }

        (alpha, scales)
    }

    /// Backward algorithm — compute β[t][k] = P(obs[t+1..T] | state_t=k, λ).
    fn backward(&self, observations: &[f64], scales: &[f64]) -> DMatrix<f64> {
        let t_len = observations.len();
        let b = self.emission_matrix(observations);
        let mut beta = DMatrix::zeros(t_len, self.k);

        // Initialization: β[T-1][k] = 1 (scaled)
        for k in 0..self.k {
            beta[(t_len - 1, k)] = 1.0;
        }

        // Induction (backward): β[t][i] = Σ_j(A[i][j] * B[t+1][j] * β[t+1][j])
        for t in (0..t_len - 1).rev() {
            for i in 0..self.k {
                let mut sum = 0.0;
                for j in 0..self.k {
                    sum += self.transition[(i, j)] * b[(t + 1, j)] * beta[(t + 1, j)];
                }
                beta[(t, i)] = sum;
            }
            if scales[t + 1] > 0.0 {
                for k in 0..self.k {
                    beta[(t, k)] /= scales[t + 1];
                }
            }
        }

        beta
    }

    /// Baum-Welch algorithm — estimate HMM parameters from observations.
    /// This is the Expectation-Maximization (EM) algorithm for HMMs.
    ///
    /// Returns the fitted model (mutates self).
    pub fn fit(&mut self, observations: &[f64], max_iter: usize, tol: f64) {
        let t_len = observations.len();
        if t_len < 2 {
            return;
        }

        for _iter in 0..max_iter {
            // E-step: compute forward-backward probabilities
            let (alpha, scales) = self.forward(observations);
            let beta = self.backward(observations, &scales);
            let b = self.emission_matrix(observations);

            // Compute γ[t][k] = P(state_t=k | observations, λ)
            let mut gamma = DMatrix::zeros(t_len, self.k);
            for t in 0..t_len {
                let mut denom = 0.0;
                for k in 0..self.k {
                    gamma[(t, k)] = alpha[(t, k)] * beta[(t, k)];
                    denom += gamma[(t, k)];
                }
                if denom > 0.0 {
                    for k in 0..self.k {
                        gamma[(t, k)] /= denom;
                    }
                }
            }

            // Compute ξ[t][i][j] = P(state_t=i, state_{t+1}=j | observations, λ)
            let mut xi = vec![DMatrix::zeros(self.k, self.k); t_len - 1];
            for t in 0..t_len - 1 {
                let mut denom = 0.0;
                for i in 0..self.k {
                    for j in 0..self.k {
                        xi[t][(i, j)] = alpha[(t, i)]
                            * self.transition[(i, j)]
                            * b[(t + 1, j)]
                            * beta[(t + 1, j)];
                        denom += xi[t][(i, j)];
                    }
                }
                if denom > 0.0 {
                    for i in 0..self.k {
                        for j in 0..self.k {
                            xi[t][(i, j)] /= denom;
                        }
                    }
                }
            }

            // M-step: re-estimate parameters
            // Initial distribution
            for k in 0..self.k {
                self.initial[k] = gamma[(0, k)];
            }

            // Transition matrix
            for i in 0..self.k {
                let gamma_sum: f64 = (0..t_len - 1).map(|t| gamma[(t, i)]).sum();
                if gamma_sum > 0.0 {
                    for j in 0..self.k {
                        let xi_sum: f64 = (0..t_len - 1).map(|t| xi[t][(i, j)]).sum();
                        self.transition[(i, j)] = xi_sum / gamma_sum;
                    }
                }
            }

            // Emission means and variances
            for k in 0..self.k {
                let gamma_sum: f64 = (0..t_len).map(|t| gamma[(t, k)]).sum();
                if gamma_sum > 0.0 {
                    // Mean
                    let weighted_sum: f64 = (0..t_len)
                        .map(|t| gamma[(t, k)] * observations[t])
                        .sum();
                    self.means[k] = weighted_sum / gamma_sum;

                    // Variance
                    let weighted_var: f64 = (0..t_len)
                        .map(|t| {
                            gamma[(t, k)] * (observations[t] - self.means[k]).powi(2)
                        })
                        .sum();
                    self.variances[k] = (weighted_var / gamma_sum).max(1e-10);
                }
            }

            // Compute log-likelihood
            let new_ll: f64 = scales.iter().filter(|&&s| s > 0.0).map(|s| s.ln()).sum();

            // Check convergence
            if (new_ll - self.log_likelihood).abs() < tol {
                self.log_likelihood = new_ll;
                break;
            }
            self.log_likelihood = new_ll;
        }
    }

    /// Viterbi algorithm — find the most likely state sequence.
    pub fn decode(&self, observations: &[f64]) -> Vec<usize> {
        let t_len = observations.len();
        if t_len == 0 {
            return vec![];
        }

        let b = self.emission_matrix(observations);
        let mut delta = DMatrix::zeros(t_len, self.k); // Log probabilities
        let mut psi = vec![vec![0_usize; self.k]; t_len]; // Backpointers

        // Initialization
        for k in 0..self.k {
            delta[(0, k)] = self.initial[k].ln() + b[(0, k)].ln();
        }

        // Recursion
        for t in 1..t_len {
            for j in 0..self.k {
                let mut best_val = f64::NEG_INFINITY;
                let mut best_i = 0;
                for i in 0..self.k {
                    let val = delta[(t - 1, i)] + self.transition[(i, j)].ln();
                    if val > best_val {
                        best_val = val;
                        best_i = i;
                    }
                }
                delta[(t, j)] = best_val + b[(t, j)].ln();
                psi[t][j] = best_i;
            }
        }

        // Backtrack
        let mut states = vec![0_usize; t_len];
        states[t_len - 1] = (0..self.k)
            .max_by(|&a, &b| delta[(t_len - 1, a)].partial_cmp(&delta[(t_len - 1, b)]).unwrap())
            .unwrap_or(0);

        for t in (0..t_len - 1).rev() {
            states[t] = psi[t + 1][states[t + 1]];
        }

        states
    }

    /// Get the current state probabilities (last row of gamma from forward-backward).
    pub fn current_state_probabilities(&self, observations: &[f64]) -> Vec<f64> {
        let (alpha, _scales) = self.forward(observations);
        let t = observations.len();
        if t == 0 {
            return vec![1.0 / self.k as f64; self.k];
        }
        let last_row: Vec<f64> = (0..self.k).map(|k| alpha[(t - 1, k)]).collect();
        let sum: f64 = last_row.iter().sum();
        if sum > 0.0 {
            last_row.iter().map(|&v| v / sum).collect()
        } else {
            vec![1.0 / self.k as f64; self.k]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmm_construction() {
        let hmm = GaussianHMM::new(3);
        assert_eq!(hmm.k, 3);
        assert_eq!(hmm.transition.nrows(), 3);
        assert_eq!(hmm.transition.ncols(), 3);
        // Rows should sum to 1
        for i in 0..3 {
            let row_sum: f64 = (0..3).map(|j| hmm.transition[(i, j)]).sum();
            assert!((row_sum - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_hmm_fit_synthetic() {
        // Generate synthetic data: alternating trending and ranging periods
        let mut data = Vec::new();
        let mut val = 100.0;

        // Trending up
        for _ in 0..50 {
            val += 0.5 + 0.1 * (rand_simple() - 0.5);
            data.push(val);
        }
        // Ranging
        let center = val;
        for _ in 0..50 {
            val = center + 2.0 * (rand_simple() - 0.5);
            data.push(val);
        }

        // Convert to returns
        let returns: Vec<f64> = data.windows(2).map(|w| (w[1] / w[0]).ln()).collect();

        let mut hmm = GaussianHMM::new(3);
        hmm.fit(&returns, 100, 1e-6);

        // Should converge (log-likelihood should be finite)
        assert!(hmm.log_likelihood.is_finite());
    }

    /// Simple deterministic pseudo-random for tests (no external dep needed).
    fn rand_simple() -> f64 {
        use std::time::SystemTime;
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        ((seed as f64 * 0.0000001) % 1.0).abs()
    }
}

