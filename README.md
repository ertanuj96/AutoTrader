<div align="center">

# 🤖 AutoTrader

### Build Your Own Algorithmic Trading System — From Zero to Production

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.12+](https://img.shields.io/badge/Python-3.12+-3776AB?logo=python&logoColor=white)](https://python.org)
[![React 18](https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![NATS](https://img.shields.io/badge/NATS-JetStream-27AAE1?logo=nats.io)](https://nats.io)
[![Docker](https://img.shields.io/badge/Docker-Compose-2496ED?logo=docker&logoColor=white)](https://docker.com)

**An open-source, event-driven algorithmic trading platform designed to be your go-to framework for building, testing, and deploying automated trading strategies.**

[Quick Start](#-quick-start) · [Architecture](#-architecture) · [Documentation](#-documentation) · [Contributing](#-contributing) · [Roadmap](#-roadmap)

</div>

---

## 🎯 What is AutoTrader?

AutoTrader is an **institutional-grade personal trading system** built from the ground up with a focus on:

- **Event-driven architecture** — Every trading action (tick, signal, order, fill) flows as an immutable event through NATS
- **Low-latency execution** — Designed for the lowest practical latency achievable through retail broker APIs
- **Complete observability** — Real-time dashboard with PnL, positions, Greeks, latency analytics, and exposure monitoring
- **Strategy experimentation** — Plug in your own strategies with a clean, decoupled interface
- **Risk-first design** — Built-in risk engine with drawdown protection, exposure limits, and kill switch
- **Historical data accumulation** — Every tick, signal, order, and fill is persisted for backtesting and ML

Whether you're a quantitative developer, an algo trading enthusiast, or someone who wants to learn how professional trading systems work — AutoTrader gives you a production-ready foundation to build on.

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 📊 **Market Data Service** | Real-time tick ingestion via broker WebSocket, normalization, and event publishing |
| 🧠 **Strategy Engine** | Pluggable strategy framework with built-in EMA crossover example |
| ⚡ **Execution Engine** | Order placement, modification, cancellation, and fill tracking |
| 🛡️ **Risk Engine** | Real-time margin checks, exposure limits, drawdown protection, and kill switch |
| 📈 **React Dashboard** | Live PnL charts, position management, strategy controls, latency monitoring |
| 🔌 **Broker Adapter** | Modular broker integration (currently INDstocks, easily extensible) |
| 💾 **Dual Storage** | Redis for hot state + ClickHouse for historical analytics |
| 📡 **NATS Messaging** | JetStream-powered event bus with replay capability |
| 📉 **Grafana + Prometheus** | Infrastructure and trading metrics monitoring |
| 🧾 **Paper Trading** | Full simulation mode for strategy development without risking capital |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    React Dashboard (Vite)                    │
├─────────────────────────────────────────────────────────────┤
│                  Dashboard API (FastAPI)                     │
├─────────────────────────────────────────────────────────────┤
│                   Redis (Live State)                         │
├─────────────────────────────────────────────────────────────┤
│                     NATS (Event Bus)                         │
├──────────────┬──────────────┬───────────────┬───────────────┤
│  Market Data │   Strategy   │  Risk Engine  │  Execution    │
│   Service    │    Engine    │               │    Engine     │
├──────────────┴──────────────┴───────────────┴───────────────┤
│                    Broker Adapter                            │
├─────────────────────────────────────────────────────────────┤
│                   Exchange (NSE / MCX)                       │
└─────────────────────────────────────────────────────────────┘
         │                                          │
    ClickHouse                              Prometheus
  (Historical)                              + Grafana
```

### Tech Stack

| Component | Technology |
|-----------|------------|
| Strategy Layer | Python 3.12+ |
| Event Bus | NATS JetStream |
| Live State | Redis 7 |
| Historical Storage | ClickHouse |
| Dashboard Backend | FastAPI |
| Dashboard Frontend | React 18 + Vite |
| Monitoring | Prometheus + Grafana |
| Containerization | Docker Compose |

---

## 🚀 Quick Start

### Prerequisites

- [Docker & Docker Compose](https://docs.docker.com/get-docker/)
- [Python 3.12+](https://python.org)
- [Node.js 20+](https://nodejs.org)

### 1. Clone & Configure

```bash
git clone https://github.com/ertanuj96/AutoTrader.git
cd AutoTrader

# Copy and configure your environment
cp .env.example .env
# Edit .env with your broker API credentials
```

### 2. Start Infrastructure

```bash
# Start NATS, Redis, ClickHouse, Prometheus, and Grafana
docker-compose up -d
```

### 3. Start the Trading Core

```bash
cd trading-core
pip install -e .
python -m pts.main
```

### 4. Start the Dashboard API

```bash
cd dashboard-api
pip install -e .
uvicorn app.main:app --port 8001
```

### 5. Start the Dashboard UI

```bash
cd dashboard-ui
npm install
npm run dev
```

Open [http://localhost:5173](http://localhost:5173) to see your trading dashboard.

---

## 📂 Project Structure

```
AutoTrader/
├── trading-core/              # Core trading engine (Python)
│   └── pts/
│       ├── main.py            # Async orchestrator — starts all services
│       ├── market_data.py     # WebSocket tick ingestion & normalization
│       ├── strategy_engine.py # Pluggable strategy framework
│       ├── risk_engine.py     # Risk checks, exposure limits, kill switch
│       ├── execution_engine.py# Order lifecycle management
│       ├── broker_adapter.py  # Broker API abstraction layer
│       ├── state_manager.py   # Redis state management
│       ├── persistence.py     # ClickHouse historical storage
│       ├── events.py          # Canonical event definitions
│       ├── config.py          # Configuration management
│       └── metrics.py         # Prometheus metrics
│
├── dashboard-api/             # REST + WebSocket API (FastAPI)
│   └── app/
│       ├── main.py            # FastAPI app with WebSocket support
│       ├── deps.py            # Dependency injection
│       └── routes/
│           ├── orders.py      # Order endpoints
│           ├── positions.py   # Position endpoints
│           ├── strategies.py  # Strategy control endpoints
│           └── system.py      # System health & metrics
│
├── dashboard-ui/              # Real-time trading dashboard (React + Vite)
│   └── src/
│       ├── App.jsx            # Main dashboard layout
│       └── components/
│           ├── Header.jsx         # Connection status & branding
│           ├── StatsRow.jsx       # Key trading metrics
│           ├── PnlChart.jsx       # Real-time PnL visualization
│           ├── StrategyManager.jsx# Strategy start/stop/pause
│           ├── PositionsPanel.jsx # Active positions table
│           ├── OrderBook.jsx      # Order history
│           ├── LatencyMonitor.jsx # Execution latency analytics
│           └── ExposureGauge.jsx  # Portfolio exposure visualization
│
├── platform-docs/             # Architecture & design documents
│   ├── PTS-001-Architecture-and-Vision-v0.1.md
│   ├── PTS-002-Event-Model-and-Message-Contracts-v0.1.md
│   └── PTS-003-Data-Model-and-Storage-Architecture-v0.1.md
│
├── monitoring/                # Observability configuration
│   ├── prometheus.yml
│   └── grafana/
│       ├── provisioning/
│       └── dashboards/
│
├── docker-compose.yml         # Infrastructure services
├── .env.example               # Environment variable template
└── .gitignore
```

---

## 📚 Documentation

Detailed design documents are in the [`platform-docs/`](platform-docs/) directory:

| Document | Description |
|----------|-------------|
| [PTS-001](platform-docs/PTS-001-Architecture-and-Vision-v0.1.md) | Architecture & Vision — System overview, design principles, technology choices |
| [PTS-002](platform-docs/PTS-002-Event-Model-and-Message-Contracts-v0.1.md) | Event Model — NATS subjects, event envelopes, all message contracts |
| [PTS-003](platform-docs/PTS-003-Data-Model-and-Storage-Architecture-v0.1.md) | Data Model — ClickHouse schemas, Redis key design, retention policies |

---

## 🤝 Contributing

We welcome contributions from the community! Whether it's fixing bugs, adding strategies, improving documentation, or building new features — every contribution matters.

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### How to Contribute

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-strategy`)
3. **Commit** your changes (`git commit -m 'Add amazing strategy'`)
4. **Push** to the branch (`git push origin feature/amazing-strategy`)
5. **Open** a Pull Request

### Ideas for Contributions

- 🧠 New trading strategies (mean reversion, momentum, pairs trading, etc.)
- 🔌 Broker adapters for other platforms (Zerodha, Alpaca, Interactive Brokers)
- 📊 Additional dashboard components (volatility surface, order flow)
- 🧪 Backtesting framework
- 📖 Tutorials and documentation
- 🐛 Bug fixes and performance improvements

---

## 🗺️ Roadmap

### Phase 1 — Foundation ✅
- [x] Market data ingestion
- [x] Single strategy execution
- [x] Execution engine
- [x] Real-time dashboard
- [x] Historical storage (ClickHouse)

### Phase 2 — Multi-Strategy & Risk
- [ ] Multi-strategy portfolio management
- [ ] Advanced risk engine with position-level controls
- [ ] Portfolio-level Greeks aggregation

### Phase 3 — Analytics
- [ ] Greeks engine (real-time IV computation)
- [ ] Volatility surface visualization
- [ ] Slippage and execution quality analysis

### Phase 4 — Backtesting
- [ ] Historical event replay from ClickHouse
- [ ] Walk-forward testing framework
- [ ] Strategy comparison and ranking

### Phase 5 — Intelligence
- [ ] Adaptive execution algorithms
- [ ] Liquidity-aware order routing
- [ ] ML-assisted trade signal ranking

---

## ⚠️ Disclaimer

**AutoTrader is for educational and research purposes.** Trading in financial markets involves substantial risk. Past performance is not indicative of future results. Always paper trade first and understand the risks before deploying with real capital.

---

## 📝 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

<div align="center">

**Built with ❤️ for the algo trading community**

If you find AutoTrader useful, please ⭐ star the repo — it helps others discover it!

</div>

