//! AutoTrader Common — Shared types, events, and NATS helpers for the quant engine.

pub mod events;
pub mod config;
pub mod nats_client;

pub use events::*;
pub use config::EngineConfig;

