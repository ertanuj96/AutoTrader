//! Reversal Engine — orchestrates BOCD, CUSUM, and divergence detection
//! into a unified ReversalEvent output.

use autotrader_common::{ReversalEvent, Timeframe};
use chrono::Utc;
use uuid::Uuid;

use crate::changepoint::ChangePointDetector;
use crate::cusum::Cusum;
use crate::divergence::divergence_score;

const BUFFER_MAX: usize = 200;

/// Per-symbol, per-timeframe reversal detector.
pub struct ReversalEngine {
    symbol: String,
    timeframe: Timeframe,

    bocd: ChangePointDetector,
    cusum: Cusum,

    price_buffer: Vec<f64>,
    oi_buffer: Vec<f64>,
    volume_buffer: Vec<f64>,
    returns_buffer: Vec<f64>,

    horizon_bars: u32,
    min_observations: usize,
}

impl ReversalEngine {
    /// Construct a new engine.
    ///
    /// * `hazard_rate` – BOCD prior hazard (e.g. 0.01).
    /// * `horizon_bars` – how many bars ahead to forecast reversal probability.
    pub fn new(
        symbol: String,
        timeframe: Timeframe,
        hazard_rate: f64,
        horizon_bars: u32,
    ) -> Self {
        Self {
            symbol,
            timeframe,
            bocd: ChangePointDetector::new(hazard_rate, horizon_bars as usize),
            cusum: Cusum::new(5.0, 0.0),
            price_buffer: Vec::with_capacity(BUFFER_MAX),
            oi_buffer: Vec::with_capacity(BUFFER_MAX),
            volume_buffer: Vec::with_capacity(BUFFER_MAX),
            returns_buffer: Vec::with_capacity(BUFFER_MAX),
            horizon_bars,
            min_observations: 20,
        }
    }

    /// Feed one bar (price, open interest, volume).
    ///
    /// Returns `Some(ReversalEvent)` once `min_observations` log-returns have
    /// been accumulated; `None` during the warm-up period.
    pub fn update(&mut self, price: f64, oi: f64, volume: f64) -> Option<ReversalEvent> {
        // ── 1. Compute log-return ─────────────────────────────────────────────
        if let Some(&prev) = self.price_buffer.last() {
            if prev > 0.0 && price > 0.0 {
                let ret = (price / prev).ln();
                self.returns_buffer.push(ret);
                if self.returns_buffer.len() > BUFFER_MAX {
                    self.returns_buffer.drain(0..self.returns_buffer.len() - BUFFER_MAX);
                }
            }
        }

        // ── 2. Update buffers ─────────────────────────────────────────────────
        self.price_buffer.push(price);
        self.oi_buffer.push(oi);
        self.volume_buffer.push(volume);

        for buf in [&mut self.price_buffer, &mut self.oi_buffer, &mut self.volume_buffer] {
            if buf.len() > BUFFER_MAX {
                buf.drain(0..buf.len() - BUFFER_MAX);
            }
        }

        // ── 3. Wait for min observations ──────────────────────────────────────
        if self.returns_buffer.len() < self.min_observations {
            return None;
        }

        // ── 4. Scale return by 20-bar rolling std before feeding CUSUM ───────
        let n_std = 20.min(self.returns_buffer.len());
        let recent = &self.returns_buffer[self.returns_buffer.len() - n_std..];
        let mean: f64 = recent.iter().sum::<f64>() / n_std as f64;
        let var: f64 =
            recent.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n_std as f64;
        let std = var.sqrt().max(1e-10);
        let last_ret = *self.returns_buffer.last().unwrap();
        let normalized = last_ret / std;

        // ── 5. Update CUSUM ───────────────────────────────────────────────────
        let cusum_triggered = self.cusum.update(normalized);

        // ── 6. Update BOCD ────────────────────────────────────────────────────
        let p_reversal = self.bocd.update(normalized);

        // ── 7. Compute divergence scores ─────────────────────────────────────
        let oi_div = divergence_score(&self.price_buffer, &self.oi_buffer)
            .clamp(-1.0, 1.0);
        let vol_div = divergence_score(&self.price_buffer, &self.volume_buffer)
            .clamp(-1.0, 1.0);

        Some(ReversalEvent {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: self.symbol.clone(),
            timeframe: self.timeframe,
            p_reversal,
            horizon_bars: self.horizon_bars,
            cusum_triggered,
            oi_divergence: oi_div,
            volume_divergence: vol_div,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use autotrader_common::Timeframe;

    fn make_engine() -> ReversalEngine {
        ReversalEngine::new("NIFTY".into(), Timeframe::Min5, 0.01, 5)
    }

    #[test]
    fn no_event_before_min_observations() {
        let mut e = make_engine();
        // The first bar produces no return; returns start at bar index 1.
        // min_observations = 20, so first event can only come at bar index 21+.
        // Feed 20 bars and assert all return None.
        for i in 0..20_usize {
            let ev = e.update(18000.0 + i as f64, 1_000_000.0, 50_000.0);
            assert!(ev.is_none(), "expected None at bar {i}, got Some");
        }
    }

    #[test]
    fn p_reversal_in_unit_interval() {
        let mut e = make_engine();
        // Feed enough bars
        for i in 0..50_usize {
            let ev = e.update(18000.0 + i as f64 * 0.5, 1_000_000.0 + i as f64, 50_000.0);
            if let Some(ev) = ev {
                assert!(
                    ev.p_reversal >= 0.0 && ev.p_reversal <= 1.0,
                    "p_reversal={} out of [0,1]", ev.p_reversal
                );
            }
        }
    }

    #[test]
    fn cusum_triggers_on_sharp_move() {
        let mut e = make_engine();
        // Warm up with oscillating prices so rolling_std is non-trivial
        for i in 0..40_usize {
            let noise = if i % 2 == 0 { 10.0 } else { -10.0 };
            e.update(18000.0 + noise, 1_000_000.0, 50_000.0);
        }
        // Feed a large sustained upward drift — CUSUM should trigger
        let mut triggered = false;
        let mut price = 18100.0_f64;
        for _ in 0..30 {
            price += 50.0; // +50 per bar, way above normal ±10 noise
            let ev = e.update(price, 1_000_000.0, 50_000.0);
            if let Some(ev) = ev {
                if ev.cusum_triggered { triggered = true; break; }
            }
        }
        assert!(triggered, "CUSUM should trigger on sustained large upward drift");
    }

    #[test]
    fn oi_divergence_in_bounds() {
        let mut e = make_engine();
        for i in 0..50_usize {
            let oi = if i % 2 == 0 { 1_000_000.0 } else { 900_000.0 }; // oscillating OI
            if let Some(ev) = e.update(18000.0 + i as f64, oi, 50_000.0) {
                assert!(
                    ev.oi_divergence >= -1.0 && ev.oi_divergence <= 1.0,
                    "oi_divergence={} out of bounds", ev.oi_divergence
                );
            }
        }
    }

    #[test]
    fn event_fields_populated() {
        let mut e = make_engine();
        let mut last_ev = None;
        for i in 0..40_usize {
            last_ev = e.update(18000.0 + i as f64, 1_000_000.0, 50_000.0);
        }
        let ev = last_ev.expect("should have emitted event after 40 bars");
        assert_eq!(ev.symbol, "NIFTY");
        assert_eq!(ev.horizon_bars, 5);
        assert_eq!(ev.timeframe, Timeframe::Min5);
        assert!(ev.p_reversal >= 0.0 && ev.p_reversal <= 1.0);
    }
}
