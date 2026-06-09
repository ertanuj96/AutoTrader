# Personal Trading System (PTS)
## Architecture & Vision Document v0.1

### Purpose
Build a personal institutional-grade algorithmic trading platform for:
- NSE F&O
- MCX Futures & Options
- Multi-strategy trading
- Single-strategy trading
- Semi-automated trading
- Fully automated trading

Primary objectives:
1. Lowest practical execution latency achievable through retail broker APIs
2. Complete observability of trading lifecycle
3. Strategy experimentation and continuous improvement
4. Historical market data accumulation
5. Risk-controlled execution
6. Modular architecture for future broker migration

---

## Design Principles

### Lean First
Avoid unnecessary infrastructure.

### Event Driven
All trading activity is represented as events:
- TickReceived
- SignalGenerated
- RiskApproved
- OrderSubmitted
- OrderAcknowledged
- OrderFilled
- PositionClosed

### Execution Isolation
Execution engine must never depend on dashboard, analytics, or reporting services.

### Data Ownership
- Market Data → Immutable
- Orders → Immutable
- Trades → Immutable
- Analytics → Derived

---

## System Overview

```text
React Dashboard
      |
Dashboard API
      |
Redis State
      |
NATS
 |        |
Market   Strategy Layer (Python)
Data     |
Service  |
     Risk Engine (Rust)
           |
     Execution Core (Rust)
           |
      Broker Adapter
        (INDMoney)
           |
       NSE / MCX
```

---

## Technology Stack

| Component | Technology |
|------------|------------|
| Strategy Layer | Python |
| Execution Core | Rust |
| Risk Engine | Rust |
| Messaging | NATS |
| Live State | Redis |
| Historical Storage | ClickHouse |
| Dashboard Backend | FastAPI |
| Dashboard Frontend | React |
| Monitoring | Grafana |
| Metrics | Prometheus |

---

## Core Components

### Market Data Service
- Connect to broker websocket
- Normalize ticks
- Publish market events
- Persist historical data

### Strategy Engine
- Generate signals
- Manage entries and exits
- Portfolio allocation
- No direct broker interaction

### Risk Engine
- Margin checks
- Exposure checks
- Drawdown protection
- Kill switch management

### Execution Engine
- Place orders
- Modify orders
- Cancel orders
- Track fills

### Dashboard
- Start/Stop/Pause/Resume strategies
- Schedule execution
- Live PnL
- Positions
- Greeks
- Exposure
- Latency analytics

---

## Trading Modes

### Manual
Human executes trades.

### Assisted
System generates signals; human approves.

### Automated
System executes automatically.

### Hybrid
Automation level configurable per strategy.

---

## Data Storage Strategy

### Redis
Stores:
- Live positions
- Live PnL
- Greeks
- Exposure
- Runtime metrics

### ClickHouse
Stores:
- Tick data
- Signals
- Orders
- Trades
- Latency metrics

---

## Core Metrics

### Trading Metrics
- Net PnL
- Gross PnL
- Win Rate
- Profit Factor
- Drawdown
- Sharpe Ratio

### Execution Metrics
- Signal → Order
- Order → ACK
- ACK → Fill
- Total Fill Latency

### Infrastructure Metrics
- CPU
- RAM
- Disk
- Network

---

## Risk Controls

### Hard Limits
Configurable daily loss limits.

### Exposure Limits
Configurable capital allocation per strategy.

### Kill Switch
- Cancel Orders
- Square Off Positions
- Disable Strategies
- Notify User

---

## Roadmap

### Phase 1
- Market data
- Single strategy
- Execution engine
- Dashboard
- Historical storage

### Phase 2
- Multi-strategy support
- Risk engine
- Portfolio management

### Phase 3
- Greeks engine
- Volatility analytics
- Slippage analysis

### Phase 4
- Backtesting
- Walk-forward testing
- Strategy comparison

### Phase 5
- Adaptive execution
- Liquidity-aware routing
- ML-assisted trade ranking

---

## Guiding Principle

The Personal Trading System should behave like a small proprietary trading platform:
execution-focused, observable, modular, and continuously improving through data collection and post-trade analysis.
