//! Movement Analyzer — magnitude forecasting (not direction).
//! GARCH vol forecasts, ATR, and option-implied expected move.
pub mod atr;
pub mod engine;
pub mod implied_move;
pub mod vol_forecast;
pub use engine::MovementEngine;
