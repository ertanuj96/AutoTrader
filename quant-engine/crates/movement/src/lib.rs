//! Movement Analyzer — magnitude forecasting (not direction).
//! GARCH vol forecasts, ATR, and option-implied expected move.
pub mod vol_forecast;
pub mod atr;
pub mod implied_move;
pub mod engine;
pub use engine::MovementEngine;

