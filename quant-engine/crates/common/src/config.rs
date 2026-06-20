//! Engine configuration loaded from environment or config file.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct EngineConfig {
    /// NATS server URL.
    pub nats_url: String,
    /// Redis URL for state.
    pub redis_url: String,
    /// Instruments to track.
    pub instruments: Vec<String>,
    /// Risk limits.
    pub risk: RiskConfig,
    /// Regime engine parameters.
    pub regime: RegimeConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiskConfig {
    /// Max daily loss in INR.
    pub max_daily_loss: f64,
    /// Max lots per position.
    pub max_position_size: u32,
    /// Max capital per strategy.
    pub max_exposure_per_strategy: f64,
    /// Max drawdown percentage.
    pub max_drawdown_pct: f64,
    /// Max orders per second (SEBI compliance).
    pub max_ops: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegimeConfig {
    /// Number of HMM states (default: 3 — trending, ranging, high-vol).
    pub hmm_states: usize,
    /// Lookback window for Hurst exponent calculation.
    pub hurst_window: usize,
    /// GARCH(1,1) omega parameter initial guess.
    pub garch_omega: f64,
    /// GARCH(1,1) alpha parameter initial guess.
    pub garch_alpha: f64,
    /// GARCH(1,1) beta parameter initial guess.
    pub garch_beta: f64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            nats_url: "nats://localhost:4222".into(),
            redis_url: "redis://localhost:6379".into(),
            instruments: vec!["NSE:NIFTY".into(), "NSE:BANKNIFTY".into()],
            risk: RiskConfig {
                max_daily_loss: 10_000.0,
                max_position_size: 50,
                max_exposure_per_strategy: 100_000.0,
                max_drawdown_pct: 5.0,
                max_ops: 9, // Under SEBI HFT threshold
            },
            regime: RegimeConfig {
                hmm_states: 3,
                hurst_window: 100,
                garch_omega: 0.00001,
                garch_alpha: 0.1,
                garch_beta: 0.85,
            },
        }
    }
}
