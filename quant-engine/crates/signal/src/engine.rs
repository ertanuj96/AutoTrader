//! Signal Engine orchestrator.
//! TODO: Wire NATS subscriptions to consume regime/trend/reversal/movement events
//! and publish ScoredSignal via Bayesian fusion.

pub struct SignalEngine {
    pub confidence_threshold: f64,
}
impl SignalEngine {
    pub fn new(threshold: f64) -> Self { Self { confidence_threshold: threshold } }
}

