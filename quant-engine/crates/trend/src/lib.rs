//! Trend Analyzer — Kalman filter, regression, and Mann-Kendall significance test.
pub mod engine;
pub mod kalman;
pub mod mann_kendall;
pub mod regression;
pub use engine::TrendEngine;
