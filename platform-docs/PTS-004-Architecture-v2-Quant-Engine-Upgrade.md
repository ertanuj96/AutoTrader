# PTS-004
# Architecture v2.0 — Quant Engine Upgrade
## AutoTrader (Personal Trading System)

Version: 0.1 | Status: Draft

---

## 1. Purpose

Upgrade AutoTrader from a single-strategy Python EMA crossover system to a polyglot,
institutional-grade quantitative trading platform with six mathematical engines,
multi-broker support, and a Bloomberg-like terminal.

**What changes**: The v1 Python strategy engine is replaced by a Rust-based quant engine
pipeline. Go handles data ingestion. Erlang/OTP manages broker connections with fault tolerance.
The existing React dashboard and FastAPI backend continue to serve as the user interface layer.

**What stays the same**: NATS as the event bus, Redis for hot state, ClickHouse for history,
Prometheus + Grafana for metrics, Docker Compose for infrastructure.

---

## 2. System Architecture — v2 Overview

```mermaid
graph TB
    subgraph "Cloudflare Edge"
        CF_PAGES["CF Pages<br/>Terminal UI"]
        CF_WORKERS["CF Workers<br/>Auth Gateway"]
    end

    subgraph "Mumbai VM"
        subgraph "Go: Data Ingestion"
            DHAN_WS["Dhan WS"]
            FYERS_WS["Fyers WS"]
            IND_WS["INDstocks WS"]
            NORMALIZER["Tick Normalizer"]
            CANDLE["Candle Materializer<br/>1m/5m/15m/1h/4h"]
            CHAIN["Option Chain Poller"]
            EOD["EOD Bhavcopy + VIX"]
        end

        subgraph "Rust: Quant Engine"
            REGIME["Regime Engine<br/>HMM + Hurst + GARCH"]
            TREND["Trend Analyzer<br/>Kalman + Mann-Kendall"]
            REVERSAL["Reversal Analyzer<br/>Changepoint + CUSUM"]
            MOVEMENT["Movement Analyzer<br/>GARCH Vol + ATR"]
            SIGNAL["Signal Engine<br/>Bayesian Fusion"]
            EXECUTOR["Executor Engine<br/>Kelly + Risk + Throttle"]
        end

        subgraph "Erlang/OTP: Broker Supervision"
            BROKER_SUP["Supervisor Tree"]
            DHAN_SESSION["Dhan Session"]
            FYERS_SESSION["Fyers Session"]
            IND_SESSION["INDstocks Session"]
            RATE_LIMITER["Rate Limiter<br/>SEBI < 10 OPS"]
            KILL_SW["Kill Switch"]
        end

        subgraph "Python: Research"
            BACKTEST["Backtester<br/>Full Indian Costs"]
            WALKFWD["Walk-Forward<br/>Validator"]
            SENTIMENT["News Sentiment"]
        end

        subgraph "Infrastructure"
            NATS["NATS JetStream"]
            REDIS["Redis 7"]
            CH["ClickHouse"]
        end

        DASHBOARD_API["Dashboard API<br/>FastAPI + WS"]
        DASHBOARD_UI["Dashboard UI<br/>React + Vite"]
    end

    %% Data flow
    DHAN_WS --> NORMALIZER
    FYERS_WS --> NORMALIZER
    IND_WS --> NORMALIZER
    NORMALIZER --> NATS
    NORMALIZER --> CH
    NORMALIZER --> CANDLE
    CANDLE --> NATS
    CHAIN --> NATS
    CHAIN --> CH
    EOD --> CH

    NATS --> REGIME
    NATS --> TREND
    NATS --> REVERSAL
    NATS --> MOVEMENT
    REGIME --> SIGNAL
    TREND --> SIGNAL
    REVERSAL --> SIGNAL
    MOVEMENT --> SIGNAL
    SIGNAL --> EXECUTOR

    EXECUTOR --> NATS
    NATS --> BROKER_SUP
    BROKER_SUP --> DHAN_SESSION
    BROKER_SUP --> FYERS_SESSION
    BROKER_SUP --> IND_SESSION
    BROKER_SUP --> RATE_LIMITER
    BROKER_SUP --> KILL_SW

    NATS --> DASHBOARD_API
    REDIS --> DASHBOARD_API
    DASHBOARD_API --> DASHBOARD_UI
    CF_WORKERS --> DASHBOARD_API
    CF_PAGES --> CF_WORKERS

    DHAN_SESSION --> |"NSE/MCX"| DHAN_WS
    FYERS_SESSION --> FYERS_WS
    IND_SESSION --> IND_WS
```

---

## 3. v1 → v2 Integration Map

The v2 system is **additive** — it does not break v1. The existing dashboard continues
to work with v1's Python strategy engine. v2 engines publish to new NATS subjects
(`pts.quant.*`) that the dashboard API can subscribe to alongside existing `pts.signal.*`.

```mermaid
flowchart LR
    subgraph "v1 (Existing — Unchanged)"
        V1_MD["Python<br/>Market Data"]
        V1_STRAT["Python<br/>EMA Strategy"]
        V1_RISK["Python<br/>Risk Engine"]
        V1_EXEC["Python<br/>Execution Engine"]
        V1_API["FastAPI<br/>Dashboard API"]
        V1_UI["React<br/>Dashboard UI"]
    end

    subgraph "v2 (New — Parallel)"
        V2_INGEST["Go<br/>Ingestion"]
        V2_QUANT["Rust<br/>Quant Engines"]
        V2_BROKER["Erlang<br/>Broker Supervisor"]
    end

    NATS_BUS["NATS JetStream<br/>(Shared Event Bus)"]

    V1_MD -->|pts.market.tick| NATS_BUS
    V2_INGEST -->|pts.market.tick<br/>pts.market.candle| NATS_BUS

    NATS_BUS -->|pts.market.*| V1_STRAT
    NATS_BUS -->|pts.market.*| V2_QUANT

    V1_STRAT -->|pts.signal.generated| NATS_BUS
    V2_QUANT -->|pts.quant.signal| NATS_BUS

    NATS_BUS --> V1_RISK
    NATS_BUS --> V1_EXEC
    NATS_BUS --> V2_BROKER

    NATS_BUS --> V1_API --> V1_UI

    style V1_MD fill:#4a5568
    style V1_STRAT fill:#4a5568
    style V1_RISK fill:#4a5568
    style V1_EXEC fill:#4a5568
    style V2_INGEST fill:#2d6a4f
    style V2_QUANT fill:#9b2226
    style V2_BROKER fill:#6a4c93
```

**Migration strategy**: Run v1 and v2 in parallel. v1 continues handling `pts.signal.generated`.
v2's scored signals go through `pts.quant.signal`. The dashboard API subscribes to both.
Once v2 is validated via paper trading, gradually route more capital through v2.

---

## 4. Quant Engine Pipeline — Signal Flow

```mermaid
flowchart TD
    TICK["Tick Event<br/>(every ~100ms)"]
    CANDLE["Candle Event<br/>(1m/5m/15m/1h/4h)"]

    TICK --> REGIME_ENGINE
    CANDLE --> REGIME_ENGINE
    TICK --> TREND_ENGINE
    CANDLE --> TREND_ENGINE

    subgraph "Analysis Layer (parallel per timeframe)"
        REGIME_ENGINE["🔴 Regime Engine<br/>P(trending), P(ranging), P(high_vol)<br/>Hurst exponent, GARCH variance"]
        TREND_ENGINE["🟢 Trend Analyzer<br/>Kalman slope, R², Mann-Kendall p-value<br/>Direction + Strength"]
        REVERSAL_ENGINE["🟡 Reversal Analyzer<br/>P(reversal within N bars)<br/>CUSUM, OI/Vol divergence"]
        MOVEMENT_ENGINE["🔵 Movement Analyzer<br/>GARCH vol, ATR, implied move<br/>Expected range"]
    end

    TICK --> REVERSAL_ENGINE
    TICK --> MOVEMENT_ENGINE

    REGIME_ENGINE --> FUSION
    TREND_ENGINE --> FUSION
    REVERSAL_ENGINE --> FUSION
    MOVEMENT_ENGINE --> FUSION

    subgraph "Decision Layer"
        FUSION["Bayesian Fusion<br/>P(signal_correct | all_evidence)<br/>Confidence decomposition"]
        FILTER["Confidence Filter<br/>Only trade above threshold"]
        CALIBRATION["Brier Score<br/>Calibration tracking"]
    end

    FUSION --> FILTER
    FUSION --> CALIBRATION
    FILTER --> EXECUTOR_ENGINE

    subgraph "Execution Layer"
        EXECUTOR_ENGINE["Executor Engine<br/>Kelly sizing → Risk check → OPS throttle"]
    end

    EXECUTOR_ENGINE --> ORDER["Order Intent<br/>→ Erlang Broker Supervisor"]

    style REGIME_ENGINE fill:#9b2226
    style TREND_ENGINE fill:#2d6a4f
    style REVERSAL_ENGINE fill:#e9c46a,color:#000
    style MOVEMENT_ENGINE fill:#264653
    style FUSION fill:#6a4c93
```

---

## 5. Entity-Relationship Diagram (ClickHouse)

```mermaid
erDiagram
    INSTRUMENT_MASTER {
        String symbol PK
        String exchange
        String segment
        String instrument_type
        Float64 lot_size
        Date expiry
        Float64 strike
        String option_type
        String underlying
    }

    TICK_DATA {
        DateTime64 ts PK
        String symbol FK
        String exchange
        Float64 ltp
        Float64 bid
        Float64 ask
        UInt64 bid_qty
        UInt64 ask_qty
        UInt64 volume
        UInt64 oi
    }

    CANDLE_DATA {
        DateTime64 ts PK
        String symbol FK
        String timeframe
        Float64 open
        Float64 high
        Float64 low
        Float64 close
        UInt64 volume
        UInt64 oi
    }

    OPTION_CHAIN_SNAPSHOT {
        DateTime64 ts PK
        String underlying FK
        Date expiry
        Float64 strike
        String option_type
        Float64 ltp
        Float64 bid
        Float64 ask
        UInt64 oi
        UInt64 volume
    }

    GREEKS_SNAPSHOT {
        DateTime64 ts PK
        String symbol FK
        Float64 iv
        Float64 delta
        Float64 gamma
        Float64 theta
        Float64 vega
    }

    SIGNAL_HISTORY {
        UUID signal_id PK
        DateTime64 ts
        String strategy_id
        String symbol FK
        String action
        UInt64 quantity
        Float64 confidence
        String reason
    }

    REGIME_HISTORY {
        DateTime64 ts PK
        String symbol FK
        String timeframe
        Float64 p_trending
        Float64 p_ranging
        Float64 p_high_vol
        Float64 hurst
        Float64 garch_var
        String regime
    }

    SIGNAL_SCORES {
        UUID signal_id PK
        DateTime64 ts
        String symbol FK
        Float64 confidence
        Float64 regime_contrib
        Float64 trend_contrib
        Float64 reversal_contrib
        Float64 movement_contrib
        UInt32 kelly_lots
        Float64 payoff_ratio
    }

    ORDER_HISTORY {
        UUID order_id PK
        String broker_order_id
        DateTime64 ts
        String strategy_id
        String symbol FK
        String side
        UInt64 quantity
        Float64 price
        String status
    }

    POSITION_HISTORY {
        UUID position_id PK
        String strategy_id
        String symbol FK
        DateTime64 entry_time
        DateTime64 exit_time
        Int64 quantity
        Float64 entry_price
        Float64 exit_price
        Float64 realized_pnl
    }

    PCR_HISTORY {
        DateTime64 ts PK
        String underlying FK
        Float64 pcr_oi
        Float64 pcr_volume
        UInt64 total_call_oi
        UInt64 total_put_oi
    }

    VIX_HISTORY {
        DateTime64 ts PK
        Float64 vix_value
        Float64 vix_change_pct
    }

    EXECUTION_METRICS {
        UUID order_id PK
        DateTime64 ts
        Float64 strategy_latency_ms
        Float64 risk_latency_ms
        Float64 broker_latency_ms
        Float64 fill_latency_ms
        Float64 total_latency_ms
    }

    INSTRUMENT_MASTER ||--o{ TICK_DATA : "has ticks"
    INSTRUMENT_MASTER ||--o{ CANDLE_DATA : "has candles"
    INSTRUMENT_MASTER ||--o{ GREEKS_SNAPSHOT : "has greeks"
    INSTRUMENT_MASTER ||--o{ SIGNAL_HISTORY : "generates signals"
    INSTRUMENT_MASTER ||--o{ ORDER_HISTORY : "has orders"
    INSTRUMENT_MASTER ||--o{ POSITION_HISTORY : "has positions"
    INSTRUMENT_MASTER ||--o{ REGIME_HISTORY : "has regime"
    SIGNAL_HISTORY ||--o| SIGNAL_SCORES : "has score"
    ORDER_HISTORY ||--o| EXECUTION_METRICS : "has latency"
```

---

## 6. NATS Subject Topology

```mermaid
graph LR
    subgraph "pts.market.* (Go → All)"
        M1["pts.market.tick"]
        M2["pts.market.candle"]
        M3["pts.market.option_chain"]
        M4["pts.market.greeks"]
    end

    subgraph "pts.quant.* (Rust → Dashboard)"
        Q1["pts.quant.regime"]
        Q2["pts.quant.trend"]
        Q3["pts.quant.reversal"]
        Q4["pts.quant.movement"]
        Q5["pts.quant.signal"]
        Q6["pts.quant.execution"]
    end

    subgraph "pts.signal.* (Python v1)"
        S1["pts.signal.generated"]
        S2["pts.signal.cancelled"]
    end

    subgraph "pts.order.* (Execution)"
        O1["pts.order.submitted"]
        O2["pts.order.filled"]
        O3["pts.order.cancelled"]
    end

    subgraph "pts.system.* (Control)"
        SY1["pts.system.kill_switch"]
        SY2["pts.system.health"]
    end
```

---

## 7. Deployment Architecture

```mermaid
graph TB
    subgraph "User"
        BROWSER["Browser"]
    end

    subgraph "Cloudflare"
        CF_DNS["DNS + CDN"]
        CF_PAGES["Pages: Terminal UI"]
        CF_R2["R2: Cold Storage<br/>Daily Parquet Archives"]
    end

    subgraph "Mumbai VM (AWS ap-south-1)"
        DOCKER["Docker Compose"]
        GO_SVC["Go Ingestion"]
        RUST_SVC["Rust Quant Engines"]
        ERLANG_SVC["Erlang Broker Sup"]
        PY_SVC["Python Research"]
        API_SVC["FastAPI Dashboard"]
        NATS_SVC["NATS"]
        REDIS_SVC["Redis"]
        CH_SVC["ClickHouse"]
        PROM["Prometheus"]
        GRAF["Grafana"]
    end

    subgraph "Exchanges"
        NSE["NSE"]
        MCX["MCX"]
    end

    BROWSER --> CF_DNS --> CF_PAGES
    CF_PAGES --> API_SVC
    API_SVC --> NATS_SVC
    API_SVC --> REDIS_SVC
    GO_SVC --> NATS_SVC
    GO_SVC --> CH_SVC
    RUST_SVC --> NATS_SVC
    ERLANG_SVC --> NATS_SVC
    ERLANG_SVC --> NSE
    ERLANG_SVC --> MCX
    CH_SVC -.->|daily archive| CF_R2
```

---

## 8. Technology Stack — v2

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Quant Engines** | Rust (workspace, 6 crates) | Sub-ms math: HMM, Kalman, GARCH, Bayesian fusion |
| **Data Ingestion** | Go | High-throughput tick ingestion, candle materialization |
| **Broker Mgmt** | Erlang/OTP | Fault-tolerant supervisor trees, auto-reconnect, rate limiting |
| **Research** | Python | Backtesting, walk-forward validation, strategy prototyping |
| **Event Bus** | NATS JetStream | Persistent event streaming with replay |
| **Hot State** | Redis 7 | Live positions, portfolio, prices, regime |
| **Cold Storage** | ClickHouse | Tick history, signals, orders, analytics (partitioned by month) |
| **Archive** | Cloudflare R2 | Daily Parquet snapshots for research |
| **Dashboard API** | FastAPI (Python) | REST + WebSocket for React frontend |
| **Dashboard UI** | React 18 + Vite | Bloomberg-like terminal |
| **Monitoring** | Prometheus + Grafana | Infrastructure + trading metrics |
| **Deployment** | Docker Compose → Mumbai VM | Low-latency to NSE |

---

## 9. Redis Key Design — v2 Extensions

| Key Pattern | Type | Purpose |
|-------------|------|---------|
| `pts:regime:{symbol}:{timeframe}` | Hash | Current regime state per symbol |
| `pts:trend:{symbol}:{timeframe}` | Hash | Current trend state |
| `pts:reversal:{symbol}` | Hash | Current reversal probability |
| `pts:movement:{symbol}` | Hash | Current movement forecast |
| `pts:signal:scored:latest` | List | Last N scored signals |
| `pts:calibration:brier` | String | Current Brier score |
| `pts:prices:live` | Hash | (existing) Live prices |
| `pts:positions:active` | Hash | (existing) Active positions |
| `pts:portfolio:current` | String | (existing) Portfolio state |

---

## 10. Guiding Principles — v2

1. **v1 and v2 coexist** — NATS decoupling means both can run in parallel.
2. **Each language does what it's best at** — Rust for math, Go for I/O, Erlang for reliability, Python for research.
3. **Calibrated probabilities, not certainty** — Track Brier scores. Act only when confidence is high and sizing is correct.
4. **The regime engine gates everything** — No trend/reversal/movement signal fires without knowing the current regime.
5. **Paper trade 4-8 weeks before real capital** — Always.

