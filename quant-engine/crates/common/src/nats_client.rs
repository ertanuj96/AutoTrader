//! NATS client helpers for connecting and subscribing to event streams.

use anyhow::Result;
use async_nats::Client;
use tracing::info;

/// Connect to NATS server.
pub async fn connect(url: &str) -> Result<Client> {
    let client = async_nats::connect(url).await?;
    info!(url = url, "Connected to NATS");
    Ok(client)
}
