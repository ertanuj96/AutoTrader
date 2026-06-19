//! Integration tests: full regime detection pipeline.
//!
//! These tests exercise HMM → Hurst → GARCH together as the regime engine
//! would use them at runtime, ensuring the pipeline produces valid output
//! without panicking on realistic return series shapes.

use autotrader_regime::{
    garch::Garch,
    hmm::GaussianHMM,
    hurst::hurst_exponent,
};

// ── Fixtures ──

fn make_trending_returns(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.0008 + (i as f64 * 0.17).sin() * 0.0003).collect()
}

fn make_ranging_returns(n: usize) -> Vec<f64> {
    (0..n).map(|i| (i as f64 * 1.9).sin() * 0.0006).collect()
}

fn make_high_vol_returns(n: usize) -> Vec<f64> {
    (0..n).map(|i| (i as f64 * 0.8).sin() * 0.025).collect()
}

fn price_series_from_returns(returns: &[f64], start: f64) -> Vec<f64> {
    let mut prices = vec![start];
    for &r in returns {
        let last = *prices.last().unwrap();
        prices.push(last * (1.0 + r));
    }
    prices
}

// ── Pipeline helper ──

struct RegimeOutput {
    hurst: Option<f64>,
    garch_var: f64,
    state_probs: Vec<f64>,
    most_likely_state: usize,
}

fn run_regime_pipeline(prices: &[f64]) -> RegimeOutput {
    let returns: Vec<f64> = prices.windows(2).map(|w| (w[1] / w[0]).ln()).collect();

    let hurst = hurst_exponent(&returns);

    let garch = Garch::fit(&returns);
    let garch_var = garch.current_variance;

    let mut hmm = GaussianHMM::new(3);
    hmm.fit(&returns, 50, 1e-6);
    let state_probs = hmm.current_state_probabilities(&returns);
    let most_likely_state = state_probs
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    RegimeOutput { hurst, garch_var, state_probs, most_likely_state }
}

// ── Tests ──

#[test]
fn test_pipeline_trending_produces_valid_output() {
    let returns = make_trending_returns(300);
    let prices = price_series_from_returns(&returns, 18000.0);
    let out = run_regime_pipeline(&prices);

    assert!(out.garch_var > 0.0, "GARCH variance must be positive");
    assert_eq!(out.state_probs.len(), 3);
    let prob_sum: f64 = out.state_probs.iter().sum();
    assert!((prob_sum - 1.0).abs() < 1e-9, "probs sum = {prob_sum}");
    assert!(out.most_likely_state < 3);
    if let Some(h) = out.hurst {
        assert!(h >= 0.0 && h <= 1.0, "h = {h}");
    }
}

#[test]
fn test_pipeline_ranging_produces_valid_output() {
    let returns = make_ranging_returns(300);
    let prices = price_series_from_returns(&returns, 18000.0);
    let out = run_regime_pipeline(&prices);

    assert!(out.garch_var > 0.0);
    let prob_sum: f64 = out.state_probs.iter().sum();
    assert!((prob_sum - 1.0).abs() < 1e-9);
}

#[test]
fn test_pipeline_high_vol_garch_variance_elevated() {
    let low_vol_returns = make_trending_returns(300);
    let high_vol_returns = make_high_vol_returns(300);
    let low_prices = price_series_from_returns(&low_vol_returns, 18000.0);
    let high_prices = price_series_from_returns(&high_vol_returns, 18000.0);

    let low_out = run_regime_pipeline(&low_prices);
    let high_out = run_regime_pipeline(&high_prices);

    assert!(
        high_out.garch_var > low_out.garch_var,
        "high-vol GARCH {:.6} should exceed low-vol {:.6}",
        high_out.garch_var, low_out.garch_var
    );
}

#[test]
fn test_pipeline_trending_hurst_above_half() {
    // Strong persistent trend → H > 0.5
    let prices: Vec<f64> = (0..512).map(|i| 18000.0 + i as f64 * 5.0).collect();
    let out = run_regime_pipeline(&prices);
    if let Some(h) = out.hurst {
        assert!(h > 0.5, "trending series should have H > 0.5, got {h}");
    }
}

#[test]
fn test_pipeline_does_not_panic_on_short_series() {
    let prices: Vec<f64> = (0..25).map(|i| 18000.0 + i as f64).collect();
    let out = run_regime_pipeline(&prices);
    assert!(out.garch_var > 0.0);
}

#[test]
fn test_garch_forecast_consistency() {
    let returns = make_high_vol_returns(200);
    let mut garch = Garch::fit(&returns);
    // Feed a large shock then check forecast reverts toward long-run
    garch.update(0.10); // 10% shock
    let f1 = garch.forecast(1);
    let f10 = garch.forecast(10);
    let f50 = garch.forecast(50);
    assert!(f1 > f10, "variance should decay: f1={f1} f10={f10}");
    assert!(f10 > f50, "variance should continue decaying: f10={f10} f50={f50}");
}
