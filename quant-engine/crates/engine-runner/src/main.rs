//! AutoTrader Quant Engine Runner.
//!
//! The orchestrator binary that boots the full quant pipeline on NATS:
//!
//! ```text
//!   pts.market.candle ─┬─► RegimeEngine   ─► pts.quant.regime   ─┐
//!                      ├─► TrendEngine    ─► pts.quant.trend     ├─► SignalEngine
//!                      ├─► ReversalEngine ─► pts.quant.reversal  │   (Bayesian fusion)
//!                      └─► MovementEngine ─► pts.quant.movement ─┘        │
//!                                                                         ▼
//!                                                              pts.quant.signal
//!                                                                         │
//!                                                                         ▼
//!                                                              ExecutorEngine
//!                                                              (Kelly → risk → OPS)
//!                                                                         │
//!                                                                         ▼
//!                                                              pts.order.submitted
//! ```
//!
//! Configuration is read from environment variables (see `Settings::from_env`).

use std::collections::HashMap;

use anyhow::Result;
use async_nats::Client;
use futures::StreamExt as _;
use serde::Deserialize;
use tracing::{error, info, warn};

use autotrader_common::{subjects, Timeframe};
use autotrader_executor::{ExecutorConfig, ExecutorEngine};
use autotrader_movement::MovementEngine;
use autotrader_regime::RegimeEngine;
use autotrader_reversal::ReversalEngine;
use autotrader_signal::SignalEngine;
use autotrader_trend::TrendEngine;

// ─── Candle event (matches the Go ingestion `CandleBar` JSON) ──────────────────

#[derive(Debug, Clone, Deserialize)]
struct CandleBar {
    symbol: String,
    #[allow(dead_code)]
    exchange: String,
    timeframe: Timeframe,
    #[allow(dead_code)]
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    #[serde(default)]
    volume: u64,
    #[serde(default)]
    oi: u64,
}

// ─── Runtime settings ──────────────────────────────────────────────────────────

struct Settings {
    nats_url: String,
    confidence_threshold: f64,
    strategy_id: String,
    // Engine params
    trend_window: usize,
    reversal_hazard: f64,
    reversal_horizon: u32,
    atr_period: usize,
    annualization: f64,
    // Executor params
    capital: f64,
    lot_value: f64,
    max_lots: u32,
    kelly_max_fraction: f64,
    max_daily_loss: f64,
    max_position_size: u32,
    max_drawdown_pct: f64,
    max_ops: u32,
}

impl Settings {
    fn from_env() -> Self {
        Self {
            nats_url: env_str("NATS_URL", "nats://localhost:4222"),
            confidence_threshold: env_f64("CONFIDENCE_THRESHOLD", 0.6),
            strategy_id: env_str("STRATEGY_ID", "quant-v2"),
            trend_window: env_usize("TREND_WINDOW", 30),
            reversal_hazard: env_f64("REVERSAL_HAZARD", 0.01),
            reversal_horizon: env_u32("REVERSAL_HORIZON", 5),
            atr_period: env_usize("ATR_PERIOD", 14),
            // 252 trading days × 75 five-min bars/day ≈ default for intraday
            annualization: env_f64("ANNUALIZATION", (252.0_f64 * 75.0).sqrt()),
            capital: env_f64("CAPITAL", 1_000_000.0),
            lot_value: env_f64("LOT_VALUE", 50_000.0),
            max_lots: env_u32("MAX_LOTS", 10),
            kelly_max_fraction: env_f64("KELLY_MAX_FRACTION", 0.10),
            max_daily_loss: env_f64("MAX_DAILY_LOSS", 50_000.0),
            max_position_size: env_u32("MAX_POSITION_SIZE", 50),
            max_drawdown_pct: env_f64("MAX_DRAWDOWN_PCT", 5.0),
            max_ops: env_u32("MAX_OPS", 9),
        }
    }
}

// ─── Per-instrument analysis engine bundle ─────────────────────────────────────

struct AnalysisBundle {
    regime: RegimeEngine,
    trend: TrendEngine,
    reversal: ReversalEngine,
    movement: MovementEngine,
}

impl AnalysisBundle {
    fn new(symbol: &str, tf: Timeframe, s: &Settings) -> Self {
        Self {
            regime: RegimeEngine::new(symbol.to_string(), tf),
            trend: TrendEngine::new(symbol.to_string(), tf, s.trend_window),
            reversal: ReversalEngine::new(
                symbol.to_string(),
                tf,
                s.reversal_hazard,
                s.reversal_horizon,
            ),
            movement: MovementEngine::new(symbol.to_string(), tf, s.atr_period, s.annualization),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let settings = Settings::from_env();
    info!(nats = %settings.nats_url, strategy = %settings.strategy_id, "Quant Engine Runner starting");

    let nc = autotrader_common::nats_client::connect(&settings.nats_url).await?;

    // ── Spawn SignalEngine ──────────────────────────────────────────────────────
    {
        let nc = nc.clone();
        let signal =
            SignalEngine::new(settings.confidence_threshold, settings.strategy_id.clone());
        tokio::spawn(async move {
            if let Err(e) = signal.run(nc).await {
                error!(err = %e, "SignalEngine terminated");
            }
        });
    }

    // ── Spawn ExecutorEngine ────────────────────────────────────────────────────
    {
        let nc = nc.clone();
        let exec_cfg = ExecutorConfig {
            capital: settings.capital,
            lot_value: settings.lot_value,
            max_lots: settings.max_lots,
            kelly_max_fraction: settings.kelly_max_fraction,
            max_daily_loss: settings.max_daily_loss,
            max_position_size: settings.max_position_size,
            max_drawdown_pct: settings.max_drawdown_pct,
            max_ops: settings.max_ops,
            strategy_id: settings.strategy_id.clone(),
        };
        let executor = ExecutorEngine::new(exec_cfg);
        tokio::spawn(async move {
            if let Err(e) = executor.run(nc).await {
                error!(err = %e, "ExecutorEngine terminated");
            }
        });
    }

    // ── Analysis loop: drive the 4 engines off the candle stream ───────────────
    run_analysis(nc, settings).await
}

/// Subscribe to `pts.market.candle`, maintain a per-(symbol, timeframe) bundle of
/// analysis engines, feed each candle, and publish the resulting engine events.
async fn run_analysis(nc: Client, settings: Settings) -> Result<()> {
    let mut sub = nc.subscribe(subjects::CANDLE).await?;
    info!("Analysis loop running — subscribed to {}", subjects::CANDLE);

    let mut bundles: HashMap<(String, Timeframe), AnalysisBundle> = HashMap::new();

    while let Some(msg) = sub.next().await {
        let bar: CandleBar = match serde_json::from_slice(&msg.payload) {
            Ok(b) => b,
            Err(e) => {
                warn!(err = %e, "skipping malformed candle");
                continue;
            }
        };

        let key = (bar.symbol.clone(), bar.timeframe);
        let bundle = bundles
            .entry(key)
            .or_insert_with(|| AnalysisBundle::new(&bar.symbol, bar.timeframe, &settings));

        // RegimeEngine ─────────────────────────────────────────────────────────
        if let Some(ev) = bundle.regime.update(bar.close) {
            publish(&nc, subjects::REGIME, &ev).await;
        }
        // TrendEngine ──────────────────────────────────────────────────────────
        if let Some(ev) = bundle.trend.update(bar.close) {
            publish(&nc, subjects::TREND, &ev).await;
        }
        // ReversalEngine ───────────────────────────────────────────────────────
        if let Some(ev) =
            bundle
                .reversal
                .update(bar.close, bar.oi as f64, bar.volume as f64)
        {
            publish(&nc, subjects::REVERSAL, &ev).await;
        }
        // MovementEngine ───────────────────────────────────────────────────────
        if let Some(ev) = bundle.movement.update(bar.high, bar.low, bar.close) {
            publish(&nc, subjects::MOVEMENT, &ev).await;
        }
    }

    Ok(())
}

async fn publish<T: serde::Serialize>(nc: &Client, subject: &'static str, ev: &T) {
    match serde_json::to_vec(ev) {
        Ok(payload) => {
            if let Err(e) = nc.publish(subject, payload.into()).await {
                warn!(err = %e, subject, "failed to publish engine event");
            }
        }
        Err(e) => warn!(err = %e, subject, "failed to serialize engine event"),
    }
}

// ─── Tiny env helpers ──────────────────────────────────────────────────────────

fn env_str(key: &str, def: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| def.to_string())
}
fn env_f64(key: &str, def: f64) -> f64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(def)
}
fn env_usize(key: &str, def: usize) -> usize {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(def)
}
fn env_u32(key: &str, def: u32) -> u32 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(def)
}
