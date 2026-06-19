//! Integration tests: Kelly sizing → risk check pipeline.
//!
//! Validates that the executor never proposes trades that violate risk limits
//! and that Kelly sizing is always conservative enough to stay within bounds.

use autotrader_executor::{
    risk::RiskCheck,
    sizing::{kelly_fraction, kelly_lots},
};

struct PortfolioState {
    capital: f64,
    daily_pnl: f64,
    peak_equity: f64,
}

fn build_risk_check(state: &PortfolioState) -> RiskCheck {
    RiskCheck {
        max_daily_loss: 50_000.0,
        max_position_size: 15,
        max_drawdown_pct: 5.0,
        current_daily_pnl: state.daily_pnl,
        peak_equity: state.peak_equity,
        current_equity: state.capital,
    }
}

fn propose_trade(win_prob: f64, payoff_ratio: f64, state: &PortfolioState) -> (u32, Result<(), String>) {
    let lot_value = 50_000.0; // typical Nifty lot
    let fraction = kelly_fraction(win_prob, payoff_ratio, 0.10); // max 10% Kelly
    let proposed_lots = kelly_lots(fraction, state.capital, lot_value, 15);
    let risk = build_risk_check(state);
    let result = risk.is_safe(proposed_lots);
    (proposed_lots, result)
}

#[test]
fn test_healthy_portfolio_good_signal_approved() {
    let state = PortfolioState {
        capital: 1_000_000.0,
        daily_pnl: 5_000.0,
        peak_equity: 1_000_000.0,
    };
    let (lots, result) = propose_trade(0.65, 1.5, &state);
    assert!(result.is_ok(), "should be approved, got: {:?}", result);
    assert!(lots <= 15);
}

#[test]
fn test_daily_loss_breached_blocks_even_good_signal() {
    let state = PortfolioState {
        capital: 950_000.0,
        daily_pnl: -55_000.0,
        peak_equity: 1_000_000.0,
    };
    let (_, result) = propose_trade(0.8, 2.0, &state);
    assert!(result.is_err(), "should be blocked by daily loss limit");
    assert!(result.unwrap_err().contains("Daily loss"));
}

#[test]
fn test_drawdown_breached_blocks_trading() {
    let state = PortfolioState {
        capital: 930_000.0,   // 7% drawdown from 1M peak
        daily_pnl: -5_000.0,  // under daily loss limit
        peak_equity: 1_000_000.0,
    };
    let (_, result) = propose_trade(0.6, 1.2, &state);
    assert!(result.is_err(), "should be blocked by drawdown");
    assert!(result.unwrap_err().contains("Drawdown"));
}

#[test]
fn test_no_edge_produces_zero_lots() {
    let state = PortfolioState {
        capital: 1_000_000.0,
        daily_pnl: 0.0,
        peak_equity: 1_000_000.0,
    };
    let (lots, _) = propose_trade(0.5, 1.0, &state); // no edge
    assert_eq!(lots, 0, "zero edge should produce 0 lots");
}

#[test]
fn test_kelly_lots_never_exceeds_max_across_scenarios() {
    let scenarios = [
        (0.55, 1.2, 500_000.0),
        (0.70, 2.0, 5_000_000.0),
        (0.90, 5.0, 10_000_000.0),
    ];
    let lot_value = 50_000.0;
    for (win_prob, payoff, capital) in scenarios {
        let frac = kelly_fraction(win_prob, payoff, 0.10);
        let lots = kelly_lots(frac, capital, lot_value, 15);
        assert!(lots <= 15, "lots {lots} exceeded max for scenario ({win_prob}, {payoff}, {capital})");
    }
}

#[test]
fn test_risk_pipeline_is_deterministic() {
    let state = PortfolioState {
        capital: 1_000_000.0,
        daily_pnl: 2_000.0,
        peak_equity: 1_000_000.0,
    };
    let (lots1, r1) = propose_trade(0.6, 1.5, &state);
    let (lots2, r2) = propose_trade(0.6, 1.5, &state);
    assert_eq!(lots1, lots2);
    assert_eq!(r1.is_ok(), r2.is_ok());
}
