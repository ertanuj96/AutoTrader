//! Executor Engine — position sizing, risk checks, SEBI OPS throttle, order FSM.
pub mod engine;
pub mod risk;
pub mod sizing;
pub mod state_machine;
pub mod throttle;
pub use engine::{ExecutorConfig, ExecutorEngine, OrderIntent};
