//! Market Regime Engine — determines whether the market is trending, ranging, or volatile.
//!
//! This is the foundational engine: every other engine's behavior is gated by the regime.
//! Uses three complementary approaches:
//!   1. Hidden Markov Model (Baum-Welch estimation + Viterbi decoding)
//!   2. Hurst exponent via Rescaled Range (R/S) analysis
//!   3. GARCH(1,1) for volatility clustering detection

pub mod hmm;
pub mod hurst;
pub mod garch;
pub mod engine;

pub use engine::RegimeEngine;

