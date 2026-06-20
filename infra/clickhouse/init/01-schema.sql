-- AutoTrader ClickHouse schema — 5 databases, 15 tables.
--
-- Applied automatically at container startup via the bind-mount into
-- /docker-entrypoint-initdb.d/. Kept in sync with trading-core/pts/persistence.py
-- (the Python service also creates these idempotently with CREATE TABLE IF NOT EXISTS).
--
-- Column order in tick_data / candle_data MUST match the Go ingestion writer's
-- batch.Append() call order (internal/publisher/clickhouse.go).

CREATE DATABASE IF NOT EXISTS pts_market;
CREATE DATABASE IF NOT EXISTS pts_trading;
CREATE DATABASE IF NOT EXISTS pts_analytics;
CREATE DATABASE IF NOT EXISTS pts_system;
CREATE DATABASE IF NOT EXISTS pts_quant;

-- ─────────────────────────────────────────────────────────────────────────────
-- pts_market
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS pts_market.tick_data (
    ts DateTime64(6),
    symbol String,
    exchange String,
    ltp Float64,
    bid Float64,
    ask Float64,
    bid_qty UInt64,
    ask_qty UInt64,
    volume UInt64,
    oi UInt64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(ts)
ORDER BY (symbol, ts);

CREATE TABLE IF NOT EXISTS pts_market.candle_data (
    ts DateTime64(6),
    symbol String,
    exchange String,
    timeframe String,
    open Float64,
    high Float64,
    low Float64,
    close Float64,
    volume UInt64,
    oi UInt64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(ts)
ORDER BY (symbol, timeframe, ts);

CREATE TABLE IF NOT EXISTS pts_market.option_chain_snapshot (
    ts DateTime64(6),
    underlying String,
    expiry Date,
    strike Float64,
    option_type String,
    ltp Float64,
    bid Float64,
    ask Float64,
    oi UInt64,
    volume UInt64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(ts)
ORDER BY (underlying, expiry, strike, ts);

CREATE TABLE IF NOT EXISTS pts_market.greeks_snapshot (
    ts DateTime64(6),
    symbol String,
    iv Float64,
    delta Float64,
    gamma Float64,
    theta Float64,
    vega Float64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(ts)
ORDER BY (symbol, ts);

CREATE TABLE IF NOT EXISTS pts_market.pcr_history (
    ts DateTime64(6),
    underlying String,
    pcr_oi Float64,
    pcr_volume Float64,
    total_call_oi UInt64,
    total_put_oi UInt64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(ts)
ORDER BY (underlying, ts);

CREATE TABLE IF NOT EXISTS pts_market.vix_history (
    ts DateTime64(6),
    vix_value Float64,
    vix_change_pct Float64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(ts)
ORDER BY (ts);

CREATE TABLE IF NOT EXISTS pts_market.commodity_data (
    ts DateTime64(6),
    symbol String,
    ltp Float64,
    open Float64,
    high Float64,
    low Float64,
    close Float64,
    volume UInt64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(ts)
ORDER BY (symbol, ts);

CREATE TABLE IF NOT EXISTS pts_market.instrument_master (
    symbol String,
    exchange String,
    segment String,
    instrument_type String,
    lot_size Float64,
    expiry Date,
    strike Float64,
    option_type String,
    underlying String,
    updated_at DateTime64(6)
) ENGINE = ReplacingMergeTree(updated_at)
ORDER BY (symbol, exchange);

-- ─────────────────────────────────────────────────────────────────────────────
-- pts_trading
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS pts_trading.signal_history (
    signal_id UUID,
    ts DateTime64(6),
    strategy_id String,
    symbol String,
    action String,
    quantity UInt64,
    confidence Float64,
    reason String
) ENGINE = MergeTree()
ORDER BY (strategy_id, ts);

CREATE TABLE IF NOT EXISTS pts_trading.order_history (
    order_id UUID,
    broker_order_id String,
    ts DateTime64(6),
    strategy_id String,
    symbol String,
    side String,
    quantity UInt64,
    price Float64,
    status String
) ENGINE = MergeTree()
ORDER BY (strategy_id, ts);

CREATE TABLE IF NOT EXISTS pts_trading.trade_fills (
    fill_id UUID,
    order_id UUID,
    ts DateTime64(6),
    symbol String,
    quantity UInt64,
    fill_price Float64
) ENGINE = MergeTree()
ORDER BY (ts);

CREATE TABLE IF NOT EXISTS pts_trading.position_history (
    position_id UUID,
    strategy_id String,
    symbol String,
    entry_time DateTime64(6),
    exit_time DateTime64(6),
    quantity Int64,
    entry_price Float64,
    exit_price Float64,
    realized_pnl Float64
) ENGINE = MergeTree()
ORDER BY (strategy_id, entry_time);

-- ─────────────────────────────────────────────────────────────────────────────
-- pts_analytics
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS pts_analytics.portfolio_snapshot (
    ts DateTime64(6),
    net_pnl Float64,
    gross_exposure Float64,
    margin_used Float64,
    delta Float64,
    gamma Float64,
    theta Float64,
    vega Float64
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(ts)
ORDER BY (ts);

CREATE TABLE IF NOT EXISTS pts_analytics.execution_metrics (
    order_id UUID,
    ts DateTime64(6),
    strategy_latency_ms Float64,
    risk_latency_ms Float64,
    broker_latency_ms Float64,
    fill_latency_ms Float64,
    total_latency_ms Float64
) ENGINE = MergeTree()
ORDER BY (ts);

-- ─────────────────────────────────────────────────────────────────────────────
-- pts_quant (v2)
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS pts_quant.regime_history (
    ts DateTime64(6),
    symbol String,
    timeframe String,
    p_trending Float64,
    p_ranging Float64,
    p_high_vol Float64,
    hurst_exponent Float64,
    garch_variance Float64,
    regime String
) ENGINE = MergeTree()
PARTITION BY toYYYYMM(ts)
ORDER BY (symbol, timeframe, ts);

CREATE TABLE IF NOT EXISTS pts_quant.signal_scores (
    signal_id UUID,
    ts DateTime64(6),
    symbol String,
    confidence Float64,
    regime_contribution Float64,
    trend_contribution Float64,
    reversal_contribution Float64,
    movement_contribution Float64,
    kelly_lots UInt32,
    payoff_ratio Float64
) ENGINE = MergeTree()
ORDER BY (ts);
