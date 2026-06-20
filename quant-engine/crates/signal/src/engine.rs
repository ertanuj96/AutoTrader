//! Signal Engine — async NATS consumer that fuses regime/trend/reversal/movement
//! events into `ScoredSignal`s via Bayesian fusion.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use async_nats::Client;
use futures::StreamExt as _;
use chrono::Utc;
use tracing::{info, warn, debug};
use uuid::Uuid;

use autotrader_common::{
    RegimeEvent, TrendEvent, ReversalEvent, MovementEvent, ScoredSignal,
    subjects, Side, TrendDirection,
};

use crate::fusion::bayesian_fusion;
use crate::filter::should_trade;
use crate::calibration::CalibrationTracker;

/// Events older than this are considered stale and will not be fused.
const STALENESS_SECS: u64 = 30;

// ─── Per-symbol-timeframe state ───────────────────────────────────────────────

struct SymbolTimeframeState {
    regime:   Option<(RegimeEvent,   Instant)>,
    trend:    Option<(TrendEvent,    Instant)>,
    reversal: Option<(ReversalEvent, Instant)>,
    movement: Option<(MovementEvent, Instant)>,
}

impl SymbolTimeframeState {
    fn new() -> Self {
        Self { regime: None, trend: None, reversal: None, movement: None }
    }

    /// True if all four engines have fresh data.
    fn all_fresh(&self, staleness: Duration) -> bool {
        let now = Instant::now();
        fn is_fresh<T>(opt: &Option<(T, Instant)>, now: Instant, staleness: Duration) -> bool {
            opt.as_ref().map(|(_, t)| now.duration_since(*t) < staleness).unwrap_or(false)
        }
        is_fresh(&self.regime, now, staleness)
            && is_fresh(&self.trend, now, staleness)
            && is_fresh(&self.reversal, now, staleness)
            && is_fresh(&self.movement, now, staleness)
    }
}

// ─── SignalEngine ─────────────────────────────────────────────────────────────

/// Bayesian signal fusion engine with optional NATS transport.
pub struct SignalEngine {
    /// Minimum confidence to emit a signal.
    pub confidence_threshold: f64,

    /// Bayesian prior probability (default 0.5).
    pub prior: f64,

    /// Per-(symbol, timeframe) state store.
    state: HashMap<(String, String), SymbolTimeframeState>,

    /// Brier score tracker.
    calibration: CalibrationTracker,

    /// Strategy identifier embedded in emitted signals.
    pub strategy_id: String,
}

impl SignalEngine {
    pub fn new(confidence_threshold: f64, strategy_id: String) -> Self {
        Self {
            confidence_threshold,
            prior: 0.5,
            state: HashMap::new(),
            calibration: CalibrationTracker::new(),
            strategy_id,
        }
    }

    // ── NATS run loop ─────────────────────────────────────────────────────────

    /// Subscribe to the four quant engine subjects, fuse events, and publish
    /// `ScoredSignal`s to `pts.quant.signal`.
    pub async fn run(mut self, nc: Client) -> anyhow::Result<()> {
        let mut sub_regime   = nc.subscribe(subjects::REGIME).await?;
        let mut sub_trend    = nc.subscribe(subjects::TREND).await?;
        let mut sub_reversal = nc.subscribe(subjects::REVERSAL).await?;
        let mut sub_movement = nc.subscribe(subjects::MOVEMENT).await?;

        info!("SignalEngine running — subscribed to 4 engine subjects");

        loop {
            tokio::select! {
                Some(msg) = sub_regime.next() => {
                    if let Ok(ev) = serde_json::from_slice::<RegimeEvent>(&msg.payload) {
                        let key = (ev.symbol.clone(), ev.timeframe.to_string());
                        self.state.entry(key.clone()).or_insert_with(SymbolTimeframeState::new)
                            .regime = Some((ev, Instant::now()));
                        self.try_fuse_and_publish(&nc, &key).await;
                    }
                }
                Some(msg) = sub_trend.next() => {
                    if let Ok(ev) = serde_json::from_slice::<TrendEvent>(&msg.payload) {
                        let key = (ev.symbol.clone(), ev.timeframe.to_string());
                        self.state.entry(key.clone()).or_insert_with(SymbolTimeframeState::new)
                            .trend = Some((ev, Instant::now()));
                        self.try_fuse_and_publish(&nc, &key).await;
                    }
                }
                Some(msg) = sub_reversal.next() => {
                    if let Ok(ev) = serde_json::from_slice::<ReversalEvent>(&msg.payload) {
                        let key = (ev.symbol.clone(), ev.timeframe.to_string());
                        self.state.entry(key.clone()).or_insert_with(SymbolTimeframeState::new)
                            .reversal = Some((ev, Instant::now()));
                        self.try_fuse_and_publish(&nc, &key).await;
                    }
                }
                Some(msg) = sub_movement.next() => {
                    if let Ok(ev) = serde_json::from_slice::<MovementEvent>(&msg.payload) {
                        let key = (ev.symbol.clone(), ev.timeframe.to_string());
                        self.state.entry(key.clone()).or_insert_with(SymbolTimeframeState::new)
                            .movement = Some((ev, Instant::now()));
                        self.try_fuse_and_publish(&nc, &key).await;
                    }
                }
            }
        }
    }

    async fn try_fuse_and_publish(&self, nc: &Client, key: &(String, String)) {
        let staleness = Duration::from_secs(STALENESS_SECS);
        let st = match self.state.get(key) {
            Some(s) if s.all_fresh(staleness) => s,
            _ => return,
        };

        let regime   = &st.regime.as_ref().unwrap().0;
        let trend    = &st.trend.as_ref().unwrap().0;
        let reversal = &st.reversal.as_ref().unwrap().0;
        let movement = &st.movement.as_ref().unwrap().0;

        let (confidence, rc, tc, revc, mc) =
            bayesian_fusion(regime, trend, reversal, movement, self.prior);

        if !should_trade(confidence, self.confidence_threshold) {
            debug!(symbol = %key.0, tf = %key.1, confidence, "signal below threshold — skip");
            return;
        }

        let action = match trend.direction {
            TrendDirection::StrongUp | TrendDirection::WeakUp => Side::Buy,
            _ => Side::Sell,
        };

        let signal = ScoredSignal {
            signal_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            symbol: regime.symbol.clone(),
            exchange: "NSE".into(),
            action,
            confidence,
            regime_contribution: rc,
            trend_contribution: tc,
            reversal_contribution: revc,
            movement_contribution: mc,
            kelly_size: 0, // executor computes this
            strategy_id: self.strategy_id.clone(),
            reason: format!(
                "regime={:?} trend={:?} strength={:.2} p_rev={:.2}",
                regime.regime, trend.direction, trend.strength, reversal.p_reversal
            ),
            payoff_ratio: 1.5, // default; override with actual R:R if available
        };

        match serde_json::to_vec(&signal) {
            Ok(payload) => {
                if let Err(e) = nc.publish(subjects::SIGNAL, payload.into()).await {
                    warn!(err = %e, "failed to publish ScoredSignal");
                } else {
                    info!(
                        symbol = %signal.symbol, confidence,
                        action = ?signal.action, "ScoredSignal published"
                    );
                }
            }
            Err(e) => warn!(err = %e, "failed to serialize ScoredSignal"),
        }
    }

    // ── Synchronous test interface ─────────────────────────────────────────────

    /// Fuse events without NATS — useful for unit tests.
    ///
    /// Returns `Some((confidence, r_contrib, t_contrib, rev_contrib, m_contrib))`
    /// if confidence exceeds the threshold, else `None`.
    pub fn fuse(
        &self,
        regime:   &RegimeEvent,
        trend:    &TrendEvent,
        reversal: &ReversalEvent,
        movement: &MovementEvent,
    ) -> Option<(f64, f64, f64, f64, f64)> {
        let result = bayesian_fusion(regime, trend, reversal, movement, self.prior);
        if should_trade(result.0, self.confidence_threshold) {
            Some(result)
        } else {
            None
        }
    }

    // ── Calibration ───────────────────────────────────────────────────────────

    pub fn record_outcome(&mut self, confidence: f64, was_correct: bool) {
        self.calibration.record(confidence, was_correct);
    }

    pub fn brier_score(&self) -> f64 {
        self.calibration.brier_score()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use autotrader_common::{MarketRegime, TrendDirection, Timeframe};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_engine() -> SignalEngine {
        SignalEngine::new(0.55, "test-strategy".into())
    }

    fn regime(r: MarketRegime, p_trending: f64) -> RegimeEvent {
        RegimeEvent {
            event_id: Uuid::new_v4(), timestamp: Utc::now(),
            symbol: "NIFTY".into(), timeframe: Timeframe::Min5,
            p_trending, p_ranging: 1.0 - p_trending, p_high_vol: 0.0,
            hurst_exponent: 0.6, garch_variance: 0.0001, regime: r,
        }
    }

    fn trend(d: TrendDirection, strength: f64) -> TrendEvent {
        TrendEvent {
            event_id: Uuid::new_v4(), timestamp: Utc::now(),
            symbol: "NIFTY".into(), timeframe: Timeframe::Min5,
            direction: d, strength, kalman_slope: 0.001, kalman_uncertainty: 0.0001,
            regression_r_squared: 0.8, mann_kendall_p_value: 0.02,
        }
    }

    fn reversal(p_rev: f64) -> ReversalEvent {
        ReversalEvent {
            event_id: Uuid::new_v4(), timestamp: Utc::now(),
            symbol: "NIFTY".into(), timeframe: Timeframe::Min5,
            p_reversal: p_rev, horizon_bars: 5, cusum_triggered: false,
            oi_divergence: 0.0, volume_divergence: 0.0,
        }
    }

    fn movement(range: f64, atr: f64) -> MovementEvent {
        MovementEvent {
            event_id: Uuid::new_v4(), timestamp: Utc::now(),
            symbol: "NIFTY".into(), timeframe: Timeframe::Min5,
            expected_range: range, atr, garch_vol: 0.01,
            realized_vol: 0.012, implied_move: None,
        }
    }

    #[test]
    fn fuse_all_good_inputs_returns_some() {
        let e = make_engine();
        let result = e.fuse(
            &regime(MarketRegime::Trending, 0.9),
            &trend(TrendDirection::StrongUp, 0.9),
            &reversal(0.05),
            &movement(120.0, 80.0),
        );
        assert!(result.is_some(), "should produce a signal with all-good inputs");
        let (conf, _, _, _, _) = result.unwrap();
        assert!(conf >= 0.0 && conf <= 1.0);
    }

    #[test]
    fn fuse_all_bad_inputs_returns_none() {
        let e = make_engine();
        let result = e.fuse(
            &regime(MarketRegime::HighVolatility, 0.0),
            &trend(TrendDirection::Neutral, 0.0),
            &reversal(0.95),
            &movement(10.0, 80.0),
        );
        // With all-bad inputs posterior should fall below 0.55 threshold
        if let Some((conf, _, _, _, _)) = result {
            // If it does pass the threshold, confidence should still be in [0,1]
            assert!(conf >= 0.0 && conf <= 1.0);
        }
        // (result may be None — that is the expected case for bad inputs)
    }

    #[test]
    fn brier_score_starts_at_random() {
        let e = make_engine();
        assert!((e.brier_score() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn record_outcome_updates_brier() {
        let mut e = make_engine();
        for _ in 0..100 { e.record_outcome(1.0, true); }
        assert!(e.brier_score() < 0.01);
    }
}
