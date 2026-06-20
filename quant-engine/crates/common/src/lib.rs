//! AutoTrader Common — Shared types, events, and NATS helpers for the quant engine.

pub mod config;
pub mod events;
pub mod nats_client;

pub use config::EngineConfig;
pub use events::*;
