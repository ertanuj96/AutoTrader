//! Reversal Engine orchestrator.
//! TODO: Wire up changepoint + cusum + divergence into ReversalEvent output.

pub struct ReversalEngine {
    pub symbol: String,
}

impl ReversalEngine {
    pub fn new(symbol: String) -> Self {
        Self { symbol }
    }
}

