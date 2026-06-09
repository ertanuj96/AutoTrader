//! Signal Generation Engine — Bayesian fusion of all analysis engines.
pub mod fusion;
pub mod calibration;
pub mod filter;
pub mod engine;
pub use engine::SignalEngine;

