//! Brier score calibration tracking.
//! Brier score = (1/N) * Σ (confidence - outcome)² where outcome ∈ {0, 1}.
//! Perfect calibration: when system says 70%, it's right ~70% of the time.

pub struct CalibrationTracker {
    predictions: Vec<(f64, bool)>, // (confidence, was_correct)
}

impl CalibrationTracker {
    pub fn new() -> Self { Self { predictions: Vec::new() } }

    pub fn record(&mut self, confidence: f64, was_correct: bool) {
        self.predictions.push((confidence, was_correct));
        if self.predictions.len() > 10_000 {
            self.predictions.drain(0..5_000);
        }
    }

    /// Brier score: lower is better. 0 = perfect, 0.25 = random.
    pub fn brier_score(&self) -> f64 {
        if self.predictions.is_empty() { return 0.25; }
        let sum: f64 = self.predictions.iter()
            .map(|(c, correct)| {
                let outcome = if *correct { 1.0 } else { 0.0 };
                (c - outcome).powi(2)
            })
            .sum();
        sum / self.predictions.len() as f64
    }
}

