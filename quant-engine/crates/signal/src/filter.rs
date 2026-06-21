//! Confidence threshold gating — only trade above a configurable threshold.

/// Filter signals below confidence threshold.
pub fn should_trade(confidence: f64, threshold: f64) -> bool {
    confidence >= threshold
}
