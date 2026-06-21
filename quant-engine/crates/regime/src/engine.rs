//! Regime Engine orchestrator — combines HMM, Hurst, and GARCH outputs
//! into a unified regime classification per symbol and timeframe.

use autotrader_common::{MarketRegime, RegimeEvent, Timeframe};
use chrono::Utc;
use uuid::Uuid;

use crate::garch::Garch;
use crate::hmm::GaussianHMM;
use crate::hurst::hurst_exponent;

/// Per-instrument, per-timeframe regime state.
pub struct RegimeEngine {
    symbol: String,
    timeframe: Timeframe,
    hmm: GaussianHMM,
    garch: Garch,
    returns_buffer: Vec<f64>,
    prices_buffer: Vec<f64>,
    max_buffer: usize,
    min_observations: usize,
}

impl RegimeEngine {
    pub fn new(symbol: String, timeframe: Timeframe) -> Self {
        Self {
            symbol,
            timeframe,
            hmm: GaussianHMM::new(3),
            garch: Garch::new(0.00001, 0.1, 0.85),
            returns_buffer: Vec::with_capacity(512),
            prices_buffer: Vec::with_capacity(512),
            max_buffer: 500,
            min_observations: 50,
        }
    }

    /// Feed a new price and get updated regime classification.
    pub fn update(&mut self, price: f64) -> Option<RegimeEvent> {
        // Compute log-return
        if let Some(&last) = self.prices_buffer.last() {
            if last > 0.0 && price > 0.0 {
                let ret = (price / last).ln();
                self.returns_buffer.push(ret);
                self.garch.update(ret);
            }
        }
        self.prices_buffer.push(price);

        // Trim buffers
        if self.returns_buffer.len() > self.max_buffer {
            self.returns_buffer
                .drain(0..self.returns_buffer.len() - self.max_buffer);
        }
        if self.prices_buffer.len() > self.max_buffer {
            self.prices_buffer
                .drain(0..self.prices_buffer.len() - self.max_buffer);
        }

        if self.returns_buffer.len() < self.min_observations {
            return None;
        }

        // 1. HMM: fit and get state probabilities
        self.hmm.fit(&self.returns_buffer, 20, 1e-4);
        let probs = self.hmm.current_state_probabilities(&self.returns_buffer);
        let (p_trending, p_ranging, p_high_vol) = if probs.len() >= 3 {
            (probs[0], probs[1], probs[2])
        } else {
            (0.33, 0.33, 0.34)
        };

        // 2. Hurst exponent
        let h = hurst_exponent(&self.prices_buffer).unwrap_or(0.5);

        // 3. GARCH variance
        let gv = self.garch.current_variance;

        // Determine regime (argmax with Hurst as tiebreaker)
        let regime = if p_high_vol > p_trending && p_high_vol > p_ranging {
            MarketRegime::HighVolatility
        } else if h > 0.55 && p_trending >= p_ranging {
            MarketRegime::Trending
        } else if h < 0.45 && p_ranging >= p_trending {
            MarketRegime::Ranging
        } else if p_trending > p_ranging {
            MarketRegime::Trending
        } else {
            MarketRegime::Ranging
        };

        Some(RegimeEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: self.symbol.clone(),
            timeframe: self.timeframe,
            p_trending,
            p_ranging,
            p_high_vol,
            hurst_exponent: h,
            garch_variance: gv,
            regime,
        })
    }
}
