//! Movement Engine orchestrator.
//! TODO: Wire up vol_forecast + ATR + implied_move into MovementEvent.

pub struct MovementEngine {
    pub symbol: String,
}
impl MovementEngine {
    pub fn new(symbol: String) -> Self { Self { symbol } }
}

