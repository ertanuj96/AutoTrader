//! Executor Engine — consumes ScoredSignals from NATS, applies Kelly + risk +
//! SEBI throttle, emits OrderIntents.

use async_nats::Client;
use chrono::{DateTime, Utc};
use futures::StreamExt as _;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

use autotrader_common::{subjects, ScoredSignal, Side};

use crate::risk::RiskCheck;
use crate::sizing::{kelly_fraction, kelly_lots};
use crate::state_machine::{OrderRecord, OrderStateMachine, OrderStatus};
use crate::throttle::OpsThrottle;

// ─── Order intent published to broker supervisor ──────────────────────────────

/// Outbound order intent published on `pts.order.submitted`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIntent {
    pub order_id: Uuid,
    pub signal_id: Uuid,
    pub strategy_id: String,
    pub symbol: String,
    pub exchange: String,
    pub side: String,
    pub quantity: u32,
    pub order_type: String, // "MARKET"
    pub product: String,    // "MIS"
    pub timestamp: DateTime<Utc>,
}

// ─── Config ───────────────────────────────────────────────────────────────────

pub struct ExecutorConfig {
    pub capital: f64,
    pub lot_value: f64, // e.g. 50_000 for NIFTY
    pub max_lots: u32,
    pub kelly_max_fraction: f64, // cap, e.g. 0.10
    pub max_daily_loss: f64,
    pub max_position_size: u32,
    pub max_drawdown_pct: f64,
    pub max_ops: u32, // SEBI: 9
    pub strategy_id: String,
}

// ─── Engine ───────────────────────────────────────────────────────────────────

pub struct ExecutorEngine {
    config: ExecutorConfig,
    throttle: OpsThrottle,
    orders: OrderStateMachine,
    daily_pnl: f64,
    peak_equity: f64,
    current_equity: f64,
}

impl ExecutorEngine {
    pub fn new(config: ExecutorConfig) -> Self {
        let throttle = OpsThrottle::new(config.max_ops);
        let peak = config.capital;
        Self {
            throttle,
            orders: OrderStateMachine::new(),
            daily_pnl: 0.0,
            peak_equity: peak,
            current_equity: config.capital,
            config,
        }
    }

    // ── NATS run loop ─────────────────────────────────────────────────────────

    /// Subscribe to `pts.quant.signal`, apply sizing + risk + throttle, and
    /// publish `OrderIntent`s to `pts.order.submitted`.
    pub async fn run(mut self, nc: Client) -> anyhow::Result<()> {
        let mut sub = nc.subscribe(subjects::SIGNAL).await?;
        info!(strategy = %self.config.strategy_id, "ExecutorEngine running");

        while let Some(msg) = sub.next().await {
            let signal: ScoredSignal = match serde_json::from_slice(&msg.payload) {
                Ok(s) => s,
                Err(e) => {
                    warn!(err = %e, "failed to deserialize ScoredSignal — skipping");
                    continue;
                }
            };

            // ── 1. Kelly sizing ──────────────────────────────────────────────
            let fraction = kelly_fraction(
                signal.confidence,
                signal.payoff_ratio,
                self.config.kelly_max_fraction,
            );
            let lots = kelly_lots(
                fraction,
                self.current_equity,
                self.config.lot_value,
                self.config.max_lots,
            );

            if lots == 0 {
                info!(signal_id = %signal.signal_id, "Kelly sizing yielded 0 lots — skip");
                continue;
            }

            // ── 2. Risk check ────────────────────────────────────────────────
            let risk = RiskCheck {
                max_daily_loss: self.config.max_daily_loss,
                max_position_size: self.config.max_position_size,
                max_drawdown_pct: self.config.max_drawdown_pct,
                current_daily_pnl: self.daily_pnl,
                peak_equity: self.peak_equity,
                current_equity: self.current_equity,
            };
            if let Err(e) = risk.is_safe(lots) {
                warn!(err = %e, signal_id = %signal.signal_id, "risk check failed — skip");
                continue;
            }

            // ── 3. SEBI throttle ─────────────────────────────────────────────
            if !self.throttle.try_acquire() {
                warn!(signal_id = %signal.signal_id, "OPS throttle hit — skip");
                continue;
            }

            // ── 4. Record order in FSM ───────────────────────────────────────
            let order_id = Uuid::new_v4();
            let record = OrderRecord {
                order_id,
                signal_id: signal.signal_id,
                symbol: signal.symbol.clone(),
                exchange: signal.exchange.clone(),
                side: match signal.action {
                    Side::Buy => "BUY",
                    Side::Sell => "SELL",
                }
                .into(),
                requested_qty: lots,
                status: OrderStatus::Submitted,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            self.orders.submit(record);

            // ── 5. Publish order intent ──────────────────────────────────────
            let intent = OrderIntent {
                order_id,
                signal_id: signal.signal_id,
                strategy_id: self.config.strategy_id.clone(),
                symbol: signal.symbol.clone(),
                exchange: signal.exchange.clone(),
                side: match signal.action {
                    Side::Buy => "BUY",
                    Side::Sell => "SELL",
                }
                .into(),
                quantity: lots,
                order_type: "MARKET".into(),
                product: "MIS".into(),
                timestamp: Utc::now(),
            };

            match serde_json::to_vec(&intent) {
                Ok(payload) => {
                    if let Err(e) = nc.publish("pts.order.submitted", payload.into()).await {
                        warn!(err = %e, order_id = %order_id, "failed to publish OrderIntent");
                    } else {
                        info!(
                            order_id = %order_id, signal_id = %signal.signal_id,
                            symbol = %signal.symbol, lots,
                            side = ?signal.action, "OrderIntent published"
                        );
                    }
                }
                Err(e) => warn!(err = %e, "failed to serialize OrderIntent"),
            }
        }

        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::RiskCheck;
    use crate::sizing::{kelly_fraction, kelly_lots};

    fn default_config() -> ExecutorConfig {
        ExecutorConfig {
            capital: 1_000_000.0,
            lot_value: 50_000.0,
            max_lots: 15,
            kelly_max_fraction: 0.10,
            max_daily_loss: 50_000.0,
            max_position_size: 15,
            max_drawdown_pct: 5.0,
            max_ops: 9,
            strategy_id: "test".into(),
        }
    }

    /// Helper that runs the sizing + risk pipeline (mirrors what run() does).
    fn try_size_and_check(
        confidence: f64,
        payoff_ratio: f64,
        daily_pnl: f64,
        equity: f64,
        peak: f64,
        config: &ExecutorConfig,
    ) -> Result<u32, String> {
        let fraction = kelly_fraction(confidence, payoff_ratio, config.kelly_max_fraction);
        let lots = kelly_lots(fraction, equity, config.lot_value, config.max_lots);
        let risk = RiskCheck {
            max_daily_loss: config.max_daily_loss,
            max_position_size: config.max_position_size,
            max_drawdown_pct: config.max_drawdown_pct,
            current_daily_pnl: daily_pnl,
            peak_equity: peak,
            current_equity: equity,
        };
        risk.is_safe(lots)?;
        Ok(lots)
    }

    #[test]
    fn safe_conditions_high_confidence_yields_lots() {
        let cfg = default_config();
        let result = try_size_and_check(0.70, 1.5, 0.0, 1_000_000.0, 1_000_000.0, &cfg);
        assert!(result.is_ok(), "should be safe: {:?}", result);
        assert!(result.unwrap() > 0, "high confidence should yield > 0 lots");
    }

    #[test]
    fn zero_confidence_yields_zero_lots() {
        let cfg = default_config();
        let fraction = kelly_fraction(0.0, 1.5, cfg.kelly_max_fraction);
        let lots = kelly_lots(fraction, cfg.capital, cfg.lot_value, cfg.max_lots);
        assert_eq!(lots, 0, "zero confidence → 0 lots");
    }

    #[test]
    fn daily_loss_breached_returns_err() {
        let cfg = default_config();
        // Daily PnL well below limit
        let result = try_size_and_check(0.75, 2.0, -60_000.0, 940_000.0, 1_000_000.0, &cfg);
        assert!(result.is_err(), "should be blocked by daily loss limit");
        assert!(result.unwrap_err().contains("Daily loss"));
    }

    #[test]
    fn drawdown_breached_returns_err() {
        let cfg = default_config();
        // 7% drawdown — over 5% limit
        let result = try_size_and_check(0.75, 2.0, 0.0, 930_000.0, 1_000_000.0, &cfg);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Drawdown"));
    }

    #[test]
    fn max_lots_never_exceeded() {
        let cfg = default_config();
        // Even with very high confidence, lots are capped
        let fraction = kelly_fraction(0.99, 10.0, cfg.kelly_max_fraction);
        let lots = kelly_lots(fraction, cfg.capital, cfg.lot_value, cfg.max_lots);
        assert!(
            lots <= cfg.max_lots,
            "lots={lots} should not exceed max={}",
            cfg.max_lots
        );
    }
}
