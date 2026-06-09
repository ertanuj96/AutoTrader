//! Bayesian Online Changepoint Detection (Adams & MacKay, 2007).
//! Computes P(reversal within N bars) in O(1) per observation (amortized).
//! TODO: Full implementation — currently a structural placeholder.

pub struct ChangePointDetector {
    hazard_rate: f64,
    run_length_probs: Vec<f64>,
}

impl ChangePointDetector {
    pub fn new(hazard_rate: f64) -> Self {
        Self { hazard_rate, run_length_probs: vec![1.0] }
    }

    /// Update with new observation, return P(changepoint just occurred).
    pub fn update(&mut self, _observation: f64) -> f64 {
        // TODO: Implement full Adams & MacKay algorithm with:
        // 1. Predictive distribution (Student-t with sufficient statistics)
        // 2. Run-length belief propagation
        // 3. Growth probability computation
        let _h = 1.0 / self.hazard_rate;
        0.0 // Placeholder
    }
}

