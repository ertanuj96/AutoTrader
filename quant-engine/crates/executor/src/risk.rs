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
            return Err(format!(
                "Daily loss limit hit: {:.0}",
                self.current_daily_pnl
            ));
        }
        if proposed_lots > self.max_position_size {
            return Err(format!(
                "Position size {proposed_lots} > max {}",
                self.max_position_size
            ));
        }
        if self.peak_equity > 0.0 {
            let dd = (self.peak_equity - self.current_equity) / self.peak_equity * 100.0;
            if dd > self.max_drawdown_pct {
                return Err(format!(
                    "Drawdown {dd:.1}% > max {:.1}%",
                    self.max_drawdown_pct
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safe_check() -> RiskCheck {
        RiskCheck {
            max_daily_loss: 50_000.0,
            max_position_size: 10,
            max_drawdown_pct: 5.0,
            current_daily_pnl: 1_000.0,
            peak_equity: 1_000_000.0,
            current_equity: 990_000.0,
        }
    }

    #[test]
    fn test_all_safe_conditions_pass() {
        assert!(safe_check().is_safe(5).is_ok());
    }

    #[test]
    fn test_daily_loss_limit_blocks_trading() {
        let mut rc = safe_check();
        rc.current_daily_pnl = -60_000.0;
        let err = rc.is_safe(1).unwrap_err();
        assert!(err.contains("Daily loss"), "err = {err}");
    }

    #[test]
    fn test_daily_loss_exactly_at_limit_passes() {
        let mut rc = safe_check();
        rc.current_daily_pnl = -50_000.0; // exactly at limit, not over
        assert!(rc.is_safe(1).is_ok());
    }

    #[test]
    fn test_position_size_over_max_blocked() {
        let err = safe_check().is_safe(11).unwrap_err();
        assert!(err.contains("Position size"), "err = {err}");
    }

    #[test]
    fn test_position_size_at_max_passes() {
        assert!(safe_check().is_safe(10).is_ok());
    }

    #[test]
    fn test_drawdown_over_limit_blocked() {
        let mut rc = safe_check();
        rc.current_equity = 940_000.0; // 6% drawdown, limit is 5%
        let err = rc.is_safe(1).unwrap_err();
        assert!(err.contains("Drawdown"), "err = {err}");
    }

    #[test]
    fn test_zero_peak_equity_skips_drawdown_check() {
        let mut rc = safe_check();
        rc.peak_equity = 0.0;
        rc.current_equity = 0.0;
        assert!(rc.is_safe(1).is_ok());
    }

    #[test]
    fn test_daily_loss_checked_before_position_size() {
        let mut rc = safe_check();
        rc.current_daily_pnl = -100_000.0;
        let err = rc.is_safe(99).unwrap_err();
        // Daily loss is checked first
        assert!(err.contains("Daily loss"), "err = {err}");
    }
}
