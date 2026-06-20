//! Brier score calibration tracking.
//! Brier score = (1/N) * Σ (confidence - outcome)² where outcome ∈ {0, 1}.
//! Perfect calibration: when system says 70%, it's right ~70% of the time.

pub struct CalibrationTracker {
    predictions: Vec<(f64, bool)>, // (confidence, was_correct)
}

impl Default for CalibrationTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationTracker {
    pub fn new() -> Self {
        Self {
            predictions: Vec::new(),
        }
    }

    pub fn record(&mut self, confidence: f64, was_correct: bool) {
        self.predictions.push((confidence, was_correct));
        if self.predictions.len() > 10_000 {
            self.predictions.drain(0..5_000);
        }
    }

    /// Brier score: lower is better. 0 = perfect, 0.25 = random.
    pub fn brier_score(&self) -> f64 {
        if self.predictions.is_empty() {
            return 0.25;
        }
        let sum: f64 = self
            .predictions
            .iter()
            .map(|(c, correct)| {
                let outcome = if *correct { 1.0 } else { 0.0 };
                (c - outcome).powi(2)
            })
            .sum();
        sum / self.predictions.len() as f64
    }

    pub fn len(&self) -> usize {
        self.predictions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.predictions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_returns_random_score() {
        let ct = CalibrationTracker::new();
        assert!((ct.brier_score() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_perfect_predictor_score_near_zero() {
        let mut ct = CalibrationTracker::new();
        // Always predict 1.0 and always be correct
        for _ in 0..100 {
            ct.record(1.0, true);
        }
        assert!(ct.brier_score() < 0.01, "score = {}", ct.brier_score());
    }

    #[test]
    fn test_always_wrong_score_near_one() {
        let mut ct = CalibrationTracker::new();
        // Predict 1.0 but always wrong
        for _ in 0..100 {
            ct.record(1.0, false);
        }
        assert!(ct.brier_score() > 0.9, "score = {}", ct.brier_score());
    }

    #[test]
    fn test_random_50pct_predictor_near_quarter() {
        let mut ct = CalibrationTracker::new();
        // 50% confidence, alternating correct/incorrect
        for i in 0..1000 {
            ct.record(0.5, i % 2 == 0);
        }
        let bs = ct.brier_score();
        assert!((bs - 0.25).abs() < 0.01, "Brier = {bs}");
    }

    #[test]
    fn test_window_caps_at_10k() {
        let mut ct = CalibrationTracker::new();
        for i in 0..12_000 {
            ct.record(0.7, i % 3 != 0);
        }
        // After drain, window is at most 10_000 - 5_000 + new = ≤10_000
        assert!(ct.len() <= 10_000, "len = {}", ct.len());
    }

    #[test]
    fn test_score_in_unit_interval() {
        let mut ct = CalibrationTracker::new();
        for i in 0..500 {
            ct.record((i % 11) as f64 / 10.0, i % 2 == 0);
        }
        let s = ct.brier_score();
        assert!((0.0..=1.0).contains(&s), "score = {s}");
    }
}
