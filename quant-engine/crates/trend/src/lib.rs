//! Trend Analyzer — Kalman filter, regression, and Mann-Kendall significance test.
pub mod kalman;
pub mod regression;
pub mod mann_kendall;
pub mod engine;
pub use engine::TrendEngine;

