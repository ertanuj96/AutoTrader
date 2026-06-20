//! Signal Generation Engine — Bayesian fusion of all analysis engines.
pub mod calibration;
pub mod engine;
pub mod filter;
pub mod fusion;
pub use engine::SignalEngine;
