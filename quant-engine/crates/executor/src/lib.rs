//! Executor Engine — position sizing, risk checks, SEBI OPS throttle, order FSM.
pub mod sizing;
pub mod risk;
pub mod throttle;
pub mod state_machine;
pub mod engine;
pub use engine::{ExecutorConfig, ExecutorEngine, OrderIntent};

