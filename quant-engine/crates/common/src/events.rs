//! Canonical event types shared across all quant engines.
//! Maps to PTS-002 event contracts + new quant-specific events (PTS-006).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Market Data Events (from Go ingestion layer) ───

/// Normalized tick event — consumed by all quant engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub exchange: String,
    pub ltp: f64,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub bid_qty: Option<u64>,
    pub ask_qty: Option<u64>,
    pub volume: Option<u64>,
    pub oi: Option<u64>,
}

/// OHLCV candle — materialized by Go ingestion layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub exchange: String,
    pub timeframe: Timeframe,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
    pub oi: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Timeframe {
    #[serde(rename = "1m")]
    Min1,
    #[serde(rename = "5m")]
    Min5,
    #[serde(rename = "15m")]
    Min15,
    #[serde(rename = "1h")]
    Hour1,
    #[serde(rename = "4h")]
    Hour4,
    #[serde(rename = "1d")]
    Day1,
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Timeframe::Min1 => "1m",
            Timeframe::Min5 => "5m",
            Timeframe::Min15 => "15m",
            Timeframe::Hour1 => "1h",
            Timeframe::Hour4 => "4h",
            Timeframe::Day1 => "1d",
        };
        write!(f, "{s}")
    }
}

// ─── Quant Engine Output Events (PTS-006) ───

/// Market regime classification output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub timeframe: Timeframe,
    /// Probability the market is in a trending regime.
    pub p_trending: f64,
    /// Probability the market is in a ranging/mean-reverting regime.
    pub p_ranging: f64,
    /// Probability the market is in a high-volatility regime.
    pub p_high_vol: f64,
    /// Hurst exponent: H > 0.5 = trending, H < 0.5 = mean-reverting, H ≈ 0.5 = random walk.
    pub hurst_exponent: f64,
    /// GARCH(1,1) conditional variance forecast.
    pub garch_variance: f64,
    /// Most likely regime (argmax of probabilities).
    pub regime: MarketRegime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MarketRegime {
    Trending,
    Ranging,
    HighVolatility,
}

/// Trend analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub timeframe: Timeframe,
    /// Kalman-filtered slope estimate.
    pub kalman_slope: f64,
    /// Kalman slope uncertainty (standard deviation).
    pub kalman_uncertainty: f64,
    /// Linear regression R² (goodness of fit).
    pub regression_r_squared: f64,
    /// Mann-Kendall test p-value (< 0.05 = statistically significant trend).
    pub mann_kendall_p_value: f64,
    /// Overall trend direction.
    pub direction: TrendDirection,
    /// Trend strength [0.0, 1.0].
    pub strength: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendDirection {
    StrongUp,
    WeakUp,
    Neutral,
    WeakDown,
    StrongDown,
}

/// Reversal probability output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReversalEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub timeframe: Timeframe,
    /// Probability of reversal within next N bars.
    pub p_reversal: f64,
    /// Horizon (number of bars) for the reversal probability.
    pub horizon_bars: u32,
    /// CUSUM detection flag.
    pub cusum_triggered: bool,
    /// Price vs OI divergence score [-1.0, 1.0].
    pub oi_divergence: f64,
    /// Price vs volume divergence score [-1.0, 1.0].
    pub volume_divergence: f64,
}

/// Movement (magnitude) forecast output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementEvent {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub timeframe: Timeframe,
    /// GARCH-forecasted volatility (annualized).
    pub garch_vol: f64,
    /// Realized volatility (annualized).
    pub realized_vol: f64,
    /// Average True Range.
    pub atr: f64,
    /// Option-implied expected move (from ATM straddle).
    pub implied_move: Option<f64>,
    /// Expected range (high - low) for next bar.
    pub expected_range: f64,
}

/// Scored trading signal — the final output of the Bayesian fusion engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredSignal {
    pub signal_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub exchange: String,
    /// BUY or SELL.
    pub action: Side,
    /// Overall confidence [0.0, 1.0] — Bayesian posterior.
    pub confidence: f64,
    /// Confidence decomposition from each engine.
    pub regime_contribution: f64,
    pub trend_contribution: f64,
    pub reversal_contribution: f64,
    pub movement_contribution: f64,
    /// Suggested position size (lots) from Kelly criterion.
    pub kelly_size: u32,
    /// Strategy ID that generated the base signal.
    pub strategy_id: String,
    /// Human-readable reason.
    pub reason: String,
    /// Expected payoff ratio (reward / risk).
    pub payoff_ratio: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Side {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
}

// ─── NATS Subjects (PTS-006) ───

pub mod subjects {
    // Existing PTS-002 subjects
    pub const TICK: &str = "pts.market.tick";
    pub const CANDLE: &str = "pts.market.candle";
    pub const OPTION_CHAIN: &str = "pts.market.option_chain";

    // New quant engine subjects (PTS-006)
    pub const REGIME: &str = "pts.quant.regime";
    pub const TREND: &str = "pts.quant.trend";
    pub const REVERSAL: &str = "pts.quant.reversal";
    pub const MOVEMENT: &str = "pts.quant.movement";
    pub const SIGNAL: &str = "pts.quant.signal";
    pub const EXECUTION: &str = "pts.quant.execution";
}
