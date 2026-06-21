//! SEBI OPS throttle — ensures we stay under 10 orders/second.

use std::time::Instant;

pub struct OpsThrottle {
    max_ops: u32,
    timestamps: Vec<Instant>,
}

impl OpsThrottle {
    pub fn new(max_ops: u32) -> Self {
        Self {
            max_ops,
            timestamps: Vec::with_capacity(max_ops as usize + 1),
        }
    }

    /// Returns true if we can place an order, false if throttled.
    pub fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        self.timestamps
            .retain(|&t| now.duration_since(t).as_secs_f64() < 1.0);
        if self.timestamps.len() as u32 >= self.max_ops {
            return false;
        }
        self.timestamps.push(now);
        true
    }
}
