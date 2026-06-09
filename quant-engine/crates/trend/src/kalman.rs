//! Kalman Filter — constant velocity model for slope estimation.
//!
//! State: [price, slope]
//! Transition: price_{t+1} = price_t + slope_t, slope_{t+1} = slope_t
//! Observation: z_t = price_t + noise

use nalgebra::{Matrix2, Vector2, Matrix1x2, Matrix1};

/// 1D Kalman filter with constant velocity model.
pub struct KalmanFilter {
    /// State vector [position, velocity/slope].
    pub state: Vector2<f64>,
    /// State covariance.
    pub covariance: Matrix2<f64>,
    /// Process noise.
    q: Matrix2<f64>,
    /// Measurement noise variance.
    r: f64,
}

impl KalmanFilter {
    pub fn new(initial_price: f64, process_noise: f64, measurement_noise: f64) -> Self {
        Self {
            state: Vector2::new(initial_price, 0.0),
            covariance: Matrix2::identity() * 1.0,
            q: Matrix2::new(process_noise, 0.0, 0.0, process_noise * 0.1),
            r: measurement_noise,
        }
    }

    /// Update the filter with a new observation. Returns (filtered_price, slope, slope_uncertainty).
    pub fn update(&mut self, observation: f64) -> (f64, f64, f64) {
        // State transition: F = [[1, 1], [0, 1]]
        let f = Matrix2::new(1.0, 1.0, 0.0, 1.0);
        let h = Matrix1x2::new(1.0, 0.0); // Observation model

        // Predict
        let pred_state = f * self.state;
        let pred_cov = f * self.covariance * f.transpose() + self.q;

        // Innovation
        let innovation = observation - (h * pred_state)[0];
        let s = (h * pred_cov * h.transpose())[0] + self.r;

        // Kalman gain
        let k = pred_cov * h.transpose() * Matrix1::new(1.0 / s);
        let kalman_gain = Vector2::new(k[(0, 0)], k[(1, 0)]);

        // Update
        self.state = pred_state + kalman_gain * innovation;
        let i_kh = Matrix2::identity() - kalman_gain * h;
        self.covariance = i_kh * pred_cov;

        let slope_uncertainty = self.covariance[(1, 1)].sqrt();
        (self.state[0], self.state[1], slope_uncertainty)
    }

    pub fn slope(&self) -> f64 { self.state[1] }
    pub fn price(&self) -> f64 { self.state[0] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_tracks_trend() {
        let mut kf = KalmanFilter::new(100.0, 0.01, 1.0);
        for i in 1..100 {
            let price = 100.0 + i as f64 * 0.5; // Linear trend
            kf.update(price);
        }
        assert!(kf.slope() > 0.4, "Slope: {}", kf.slope());
    }
}

