//! Average True Range (ATR) — volatility-adjusted range indicator.

/// Compute ATR from OHLC data. Each element is (high, low, close).
pub fn atr(ohlc: &[(f64, f64, f64)], period: usize) -> Option<f64> {
    if ohlc.len() < period + 1 { return None; }
    let mut trs = Vec::with_capacity(ohlc.len() - 1);
    for i in 1..ohlc.len() {
        let (h, l, _) = ohlc[i];
        let prev_close = ohlc[i - 1].2;
        let tr = (h - l).max((h - prev_close).abs()).max((l - prev_close).abs());
        trs.push(tr);
    }
    // EMA-style ATR over last `period` values
    let slice = &trs[trs.len().saturating_sub(period)..];
    Some(slice.iter().sum::<f64>() / slice.len() as f64)
}

