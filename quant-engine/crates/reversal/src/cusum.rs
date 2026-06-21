//! CUSUM (Cumulative Sum) change detection — Page's test.

pub struct Cusum {
    threshold: f64,
    drift: f64,
    s_pos: f64,
    s_neg: f64,
}

impl Cusum {
    pub fn new(threshold: f64, drift: f64) -> Self {
        Self {
            threshold,
            drift,
            s_pos: 0.0,
            s_neg: 0.0,
        }
    }

    /// Update with new value. Returns true if change detected.
    pub fn update(&mut self, value: f64) -> bool {
        self.s_pos = (self.s_pos + value - self.drift).max(0.0);
        self.s_neg = (self.s_neg - value - self.drift).max(0.0);
        let triggered = self.s_pos > self.threshold || self.s_neg > self.threshold;
        if triggered {
            self.s_pos = 0.0;
            self.s_neg = 0.0;
        }
        triggered
    }
}
