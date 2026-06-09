//! Position-level risk checks.
pub struct RiskCheck {
    pub max_daily_loss: f64,
    pub max_position_size: u32,
    pub max_drawdown_pct: f64,
    pub current_daily_pnl: f64,
    pub peak_equity: f64,
    pub current_equity: f64,
}

impl RiskCheck {
    pub fn is_safe(&self, proposed_lots: u32) -> Result<(), String> {
        if self.current_daily_pnl < -self.max_daily_loss {
            return Err(format!("Daily loss limit hit: {:.0}", self.current_daily_pnl));
        }
        if proposed_lots > self.max_position_size {
            return Err(format!("Position size {proposed_lots} > max {}", self.max_position_size));
        }
        if self.peak_equity > 0.0 {
            let dd = (self.peak_equity - self.current_equity) / self.peak_equity * 100.0;
            if dd > self.max_drawdown_pct {
                return Err(format!("Drawdown {dd:.1}% > max {:.1}%", self.max_drawdown_pct));
            }
        }
        Ok(())
    }
}

