//! Movement Engine — magnitude forecasting via EWMA vol, ATR, and implied move.

use autotrader_common::{MovementEvent, Timeframe};
use chrono::Utc;
use uuid::Uuid;

use crate::atr::atr;
use crate::implied_move::implied_expected_move;
use crate::vol_forecast::realized_vol;

const BUFFER_MAX: usize = 200;

/// Per-symbol, per-timeframe movement forecaster.
pub struct MovementEngine {
    symbol: String,
    timeframe: Timeframe,

    /// OHLC buffer: (high, low, close).
    ohlc_buffer: Vec<(f64, f64, f64)>,

    /// Log-return buffer.
    returns_buffer: Vec<f64>,

    /// EWMA variance (RiskMetrics λ = 0.94).
    ewma_var: f64,

    /// EWMA decay factor.
    ewma_lambda: f64,

    /// ATR period.
    atr_period: usize,

    /// Annualization factor (e.g. 252 for daily, 252*390 for 1-min bars).
    annualization: f64,

    /// Minimum returns before emitting events.
    min_observations: usize,

    /// True after the first close is recorded.
    initialized: bool,
}

impl MovementEngine {
    /// Create a new engine.
    ///
    /// * `atr_period` – lookback for ATR (e.g. 14).
    /// * `annualization_factor` – trading periods per year (e.g. 252 for daily bars).
    pub fn new(
        symbol: String,
        timeframe: Timeframe,
        atr_period: usize,
        annualization_factor: f64,
    ) -> Self {
        Self {
            symbol,
            timeframe,
            ohlc_buffer: Vec::with_capacity(BUFFER_MAX),
            returns_buffer: Vec::with_capacity(BUFFER_MAX),
            ewma_var: 0.0,
            ewma_lambda: 0.94,
            atr_period,
            annualization: annualization_factor,
            min_observations: atr_period + 1,
            initialized: false,
        }
    }

    /// Update with one OHLC bar. Returns `Some(MovementEvent)` after warm-up.
    pub fn update(&mut self, high: f64, low: f64, close: f64) -> Option<MovementEvent> {
        // ── 1. Compute log-return from consecutive closes ─────────────────────
        if let Some(&(_, _, prev_close)) = self.ohlc_buffer.last() {
            if prev_close > 0.0 && close > 0.0 {
                let ret = (close / prev_close).ln();
                // ── 2. Update EWMA variance ───────────────────────────────────
                if !self.initialized {
                    self.ewma_var = ret * ret;
                    self.initialized = true;
                } else {
                    let lam = self.ewma_lambda;
                    self.ewma_var = lam * self.ewma_var + (1.0 - lam) * ret * ret;
                }
                self.returns_buffer.push(ret);
                if self.returns_buffer.len() > BUFFER_MAX {
                    self.returns_buffer
                        .drain(0..self.returns_buffer.len() - BUFFER_MAX);
                }
            }
        } else {
            // First bar — no return yet, but initialise EWMA on small guess
            self.ewma_var = 0.0001;
        }

        self.ohlc_buffer.push((high, low, close));
        if self.ohlc_buffer.len() > BUFFER_MAX {
            self.ohlc_buffer
                .drain(0..self.ohlc_buffer.len() - BUFFER_MAX);
        }

        // ── 3. Warm-up gate ───────────────────────────────────────────────────
        if self.returns_buffer.len() < self.min_observations {
            return None;
        }

        // ── 4. Realized vol (annualized) ─────────────────────────────────────
        let rv = realized_vol(&self.returns_buffer, self.annualization);

        // ── 5. GARCH-style vol from EWMA (annualized) ────────────────────────
        let gv = self.ewma_var.sqrt() * self.annualization.sqrt();

        // ── 6. ATR ───────────────────────────────────────────────────────────
        let atr_val = atr(&self.ohlc_buffer, self.atr_period)?;

        // ── 7. Expected range: ATR is the baseline; scale up if vol elevated ─
        let vol_ratio = if rv > 0.0 { gv / rv } else { 1.0 };
        let expected_range = atr_val * vol_ratio.max(1.0);

        Some(MovementEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: self.symbol.clone(),
            timeframe: self.timeframe,
            garch_vol: gv,
            realized_vol: rv,
            atr: atr_val,
            implied_move: None, // set by update_with_options
            expected_range,
        })
    }

    /// Update with OHLC + option premiums to populate `implied_move`.
    pub fn update_with_options(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
        call_prem: f64,
        put_prem: f64,
    ) -> Option<MovementEvent> {
        let mut ev = self.update(high, low, close)?;
        ev.implied_move = Some(implied_expected_move(call_prem, put_prem));
        Some(ev)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use autotrader_common::Timeframe;

    fn make_engine() -> MovementEngine {
        MovementEngine::new("NIFTY".into(), Timeframe::Min5, 14, 252.0)
    }

    #[test]
    fn no_event_before_min_observations() {
        let mut e = make_engine();
        for i in 0..14_usize {
            let price = 18000.0 + i as f64;
            let ev = e.update(price + 10.0, price - 10.0, price);
            assert!(ev.is_none(), "expected None at bar {i}");
        }
    }

    #[test]
    fn atr_positive_after_warmup() {
        let mut e = make_engine();
        let mut last_ev = None;
        for i in 0..30_usize {
            let price = 18000.0 + i as f64;
            last_ev = e.update(price + 15.0, price - 15.0, price);
        }
        let ev = last_ev.expect("should emit after warmup");
        assert!(ev.atr > 0.0, "ATR must be positive, got {}", ev.atr);
    }

    #[test]
    fn garch_vol_positive_after_warmup() {
        let mut e = make_engine();
        let mut last_ev = None;
        for i in 0..30_usize {
            let price = 18000.0 + i as f64 * 2.0;
            last_ev = e.update(price + 10.0, price - 10.0, price);
        }
        let ev = last_ev.expect("should emit");
        assert!(
            ev.garch_vol > 0.0,
            "garch_vol must be positive, got {}",
            ev.garch_vol
        );
    }

    #[test]
    fn high_vol_returns_higher_garch_vol_than_low_vol() {
        let mut low_vol_engine = make_engine();
        let mut high_vol_engine = make_engine();

        let mut low_ev = None;
        let mut high_ev = None;

        for i in 0..40_usize {
            let base = 18000.0 + i as f64 * 0.1; // tiny close drift → low vol
            let base_hv = 18000.0 + (i as f64 * 50.0) * (if i % 2 == 0 { 1.0 } else { -1.0 });
            low_ev = low_vol_engine.update(base + 5.0, base - 5.0, base);
            high_ev = high_vol_engine.update(base_hv + 200.0, base_hv - 200.0, base_hv);
        }

        let lv = low_ev.expect("low vol should emit");
        let hv = high_ev.expect("high vol should emit");
        assert!(
            hv.garch_vol > lv.garch_vol,
            "high vol ({}) should exceed low vol ({})",
            hv.garch_vol,
            lv.garch_vol
        );
    }

    #[test]
    fn expected_range_positive() {
        let mut e = make_engine();
        let mut last_ev = None;
        for i in 0..30_usize {
            let price = 18000.0 + i as f64;
            last_ev = e.update(price + 20.0, price - 20.0, price);
        }
        let ev = last_ev.expect("should emit");
        assert!(ev.expected_range > 0.0, "expected_range must be positive");
    }

    #[test]
    fn update_with_options_populates_implied_move() {
        let mut e = make_engine();
        let mut last_ev = None;
        for i in 0..30_usize {
            let price = 18000.0 + i as f64;
            last_ev = e.update_with_options(price + 10.0, price - 10.0, price, 50.0, 48.0);
        }
        let ev = last_ev.expect("should emit");
        let im = ev.implied_move.expect("implied_move should be Some");
        // (50 + 48) * 0.85 = 83.3
        assert!((im - 83.3).abs() < 0.5, "implied_move = {im}");
    }
}
