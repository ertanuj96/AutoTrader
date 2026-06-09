# PTS-003
# Data Model & Storage Architecture
## Personal Trading System (PTS)

Version: 0.1

---

# Purpose

Define the canonical storage architecture for:

- Market Data
- Option Chains
- Greeks
- Signals
- Orders
- Positions
- Portfolio State
- Execution Analytics

Goals:

- Low latency
- Efficient historical analysis
- Replay capability
- Future ML readiness
- Long-term scalability

---

# Storage Layers

## Layer 1: Redis (Hot Data)

Retention:

```text
Seconds -> Hours
```

Purpose:

- Live dashboard state
- Active positions
- Portfolio state
- Greeks snapshots
- Runtime metrics

Characteristics:

- Memory resident
- Extremely low latency
- Non-authoritative

---

## Layer 2: ClickHouse (Historical Source of Truth)

Retention:

```text
Unlimited
```

Purpose:

- Tick history
- Option chains
- Orders
- Trades
- Signals
- Execution analytics

Characteristics:

- Columnar storage
- Compression
- High-speed analytics

---

# Database Naming

```text
pts_market
pts_trading
pts_analytics
pts_system
```

---

# Market Data Schema

## tick_data

Purpose:

Store every normalized tick received.

Fields:

| Column | Type |
|----------|----------|
| ts | DateTime64 |
| symbol | String |
| exchange | String |
| ltp | Float64 |
| bid | Float64 |
| ask | Float64 |
| bid_qty | UInt64 |
| ask_qty | UInt64 |
| volume | UInt64 |
| oi | UInt64 |

Partition:

```sql
toYYYYMM(ts)
```

Order By:

```sql
(symbol, ts)
```

---

# Option Chain Storage

## option_chain_snapshot

Snapshot frequency:

```text
1 second
or
event driven
```

Fields:

| Column | Type |
|----------|----------|
| ts | DateTime64 |
| underlying | String |
| expiry | Date |
| strike | Float64 |
| option_type | String |
| ltp | Float64 |
| bid | Float64 |
| ask | Float64 |
| oi | UInt64 |
| volume | UInt64 |

---

# Greeks Storage

## greeks_snapshot

Fields:

| Column | Type |
|----------|----------|
| ts | DateTime64 |
| symbol | String |
| iv | Float64 |
| delta | Float64 |
| gamma | Float64 |
| theta | Float64 |
| vega | Float64 |

---

# Signal Storage

## signal_history

Fields:

| Column | Type |
|----------|----------|
| signal_id | UUID |
| ts | DateTime64 |
| strategy_id | String |
| symbol | String |
| action | String |
| quantity | UInt64 |
| confidence | Float64 |
| reason | String |

Purpose:

Store every signal generated regardless of execution.

---

# Order Storage

## order_history

Fields:

| Column | Type |
|----------|----------|
| order_id | UUID |
| broker_order_id | String |
| ts | DateTime64 |
| strategy_id | String |
| symbol | String |
| side | String |
| quantity | UInt64 |
| price | Float64 |
| status | String |

Statuses:

```text
SUBMITTED
ACKNOWLEDGED
PARTIAL_FILL
FILLED
CANCELLED
REJECTED
```

---

# Fill Storage

## trade_fills

Fields:

| Column | Type |
|----------|----------|
| fill_id | UUID |
| order_id | UUID |
| ts | DateTime64 |
| symbol | String |
| quantity | UInt64 |
| fill_price | Float64 |

---

# Position Storage

## position_history

Fields:

| Column | Type |
|----------|----------|
| position_id | UUID |
| strategy_id | String |
| symbol | String |
| entry_time | DateTime64 |
| exit_time | DateTime64 |
| quantity | Int64 |
| entry_price | Float64 |
| exit_price | Float64 |
| realized_pnl | Float64 |

---

# Portfolio Storage

## portfolio_snapshot

Snapshot interval:

```text
1 second
```

Fields:

| Column | Type |
|----------|----------|
| ts | DateTime64 |
| net_pnl | Float64 |
| gross_exposure | Float64 |
| margin_used | Float64 |
| delta | Float64 |
| gamma | Float64 |
| theta | Float64 |
| vega | Float64 |

---

# Execution Analytics

## execution_metrics

Fields:

| Column | Type |
|----------|----------|
| order_id | UUID |
| strategy_latency_ms | Float64 |
| risk_latency_ms | Float64 |
| broker_latency_ms | Float64 |
| fill_latency_ms | Float64 |
| total_latency_ms | Float64 |

Purpose:

Execution quality analysis.

---

# Redis Key Design

## Portfolio

```text
pts:portfolio:current
```

---

## Positions

```text
pts:positions:active
```

---

## Greeks

```text
pts:greeks:portfolio
```

---

## Strategy State

```text
pts:strategy:{strategy_id}:status
```

Examples:

```text
pts:strategy:OPTION_SCALPER:status
pts:strategy:MCX_BREAKOUT:status
```

---

## Live Metrics

```text
pts:metrics:latency
```

---

# Retention Policy

## Redis

| Data | Retention |
|--------|--------|
| Runtime metrics | 24h |
| Positions | Active lifecycle |
| Portfolio state | Active lifecycle |

---

## ClickHouse

| Data | Retention |
|--------|--------|
| Orders | Permanent |
| Trades | Permanent |
| Signals | Permanent |
| Tick Data | Permanent |
| Greeks | Permanent |

---

# Compression Strategy

Enable ClickHouse compression for:

- Tick data
- Option chains
- Greeks
- Portfolio snapshots

Benefits:

- Reduced storage cost
- Faster scans

---

# Historical Replay Requirements

Replay should support:

- Single symbol
- Strategy-specific
- Full market session
- Specific date ranges

Replay speed:

```text
1x
10x
100x
1000x
```

---

# Research Readiness

The following datasets must be directly usable for:

- Backtesting
- Walk-forward testing
- Feature engineering
- Machine learning
- Slippage analysis

Required datasets:

- Tick Data
- Option Chains
- Greeks
- Signals
- Orders
- Trades
- Portfolio Snapshots

---

# Future Expansion

Planned additions:

- Volatility surfaces
- Order book snapshots
- Liquidity metrics
- News events
- Economic calendar events
- Cross-broker execution metrics

---

# Guiding Principle

Redis stores operational state.
ClickHouse stores history.
NATS transports events.

No business-critical information should exist exclusively in Redis.
ClickHouse remains the authoritative historical source of truth for the Personal Trading System.
