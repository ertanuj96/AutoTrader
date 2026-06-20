//! Trend Engine orchestrator — combines Kalman, regression, and Mann-Kendall.

use crate::kalman::KalmanFilter;
use crate::mann_kendall::mann_kendall;
use crate::regression::linear_regression;
use autotrader_common::{Timeframe, TrendDirection, TrendEvent};
use chrono::Utc;
use uuid::Uuid;

pub struct TrendEngine {
    symbol: String,
    timeframe: Timeframe,
    kalman: KalmanFilter,
    price_buffer: Vec<f64>,
    window: usize,
    initialized: bool,
}

impl TrendEngine {
    pub fn new(symbol: String, timeframe: Timeframe, window: usize) -> Self {
        Self {
            symbol,
            timeframe,
            kalman: KalmanFilter::new(0.0, 0.01, 1.0),
            price_buffer: Vec::with_capacity(window + 10),
            window,
            initialized: false,
        }
    }

    pub fn update(&mut self, price: f64) -> Option<TrendEvent> {
        if !self.initialized {
            self.kalman = KalmanFilter::new(price, 0.01, 1.0);
            self.initialized = true;
        }

        let (_, slope, uncertainty) = self.kalman.update(price);
        self.price_buffer.push(price);
        if self.price_buffer.len() > self.window * 2 {
            self.price_buffer
                .drain(0..self.price_buffer.len() - self.window);
        }

        if self.price_buffer.len() < 20 {
            return None;
        }

        let window = &self.price_buffer[self.price_buffer.len().saturating_sub(self.window)..];
        let reg = linear_regression(window)?;
        let mk = mann_kendall(window, 0.05)?;

        // Combine signals into direction + strength
        let norm_slope = slope / (price.abs().max(1.0) * 0.001);
        let strength = (reg.r_squared * 0.4
            + (1.0 - mk.p_value).clamp(0.0, 1.0) * 0.3
            + norm_slope.abs().clamp(0.0, 1.0) * 0.3)
            .clamp(0.0, 1.0);

        let direction = match (norm_slope, strength) {
            (s, st) if s > 1.0 && st > 0.6 => TrendDirection::StrongUp,
            (s, _) if s > 0.2 => TrendDirection::WeakUp,
            (s, st) if s < -1.0 && st > 0.6 => TrendDirection::StrongDown,
            (s, _) if s < -0.2 => TrendDirection::WeakDown,
            _ => TrendDirection::Neutral,
        };

        Some(TrendEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: self.symbol.clone(),
            timeframe: self.timeframe,
            kalman_slope: slope,
            kalman_uncertainty: uncertainty,
            regression_r_squared: reg.r_squared,
            mann_kendall_p_value: mk.p_value,
            direction,
            strength,
        })
    }
}
