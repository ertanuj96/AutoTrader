//! Reversal Analyzer — Bayesian changepoint detection, CUSUM, and divergence.
pub mod changepoint;
pub mod cusum;
pub mod divergence;
pub mod engine;
pub use engine::ReversalEngine;

