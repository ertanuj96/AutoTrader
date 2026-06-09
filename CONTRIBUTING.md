<![CDATA[# Contributing to AutoTrader

Thank you for your interest in contributing to AutoTrader! This project aims to be the go-to open-source framework for building algorithmic trading systems, and your contributions help make that possible.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Architecture Guidelines](#architecture-guidelines)

---

## Code of Conduct

Be respectful, inclusive, and constructive. We're building something together.

---

## Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/AutoTrader.git
   cd AutoTrader
   ```
3. **Add upstream** remote:
   ```bash
   git remote add upstream https://github.com/ertanuj96/AutoTrader.git
   ```
4. **Create a branch** for your work:
   ```bash
   git checkout -b feature/your-feature-name
   ```

---

## Development Setup

### Infrastructure
```bash
# Start all services (NATS, Redis, ClickHouse, Prometheus, Grafana)
docker-compose up -d
```

### Trading Core (Python)
```bash
cd trading-core
python -m venv .venv
source .venv/bin/activate
pip install -e ".[dev]"
```

### Dashboard API (Python)
```bash
cd dashboard-api
pip install -e ".[dev]"
uvicorn app.main:app --reload --port 8001
```

### Dashboard UI (React)
```bash
cd dashboard-ui
npm install
npm run dev
```

---

## How to Contribute

### 🐛 Bug Reports
- Use GitHub Issues
- Include steps to reproduce
- Include expected vs actual behavior
- Include environment details (OS, Python version, etc.)

### 💡 Feature Requests
- Use GitHub Issues with the `enhancement` label
- Describe the use case and expected behavior
- Reference relevant architecture docs if applicable

### 🧠 New Strategies
- Place strategy code in `trading-core/pts/strategies/`
- Extend the base `Strategy` class
- Include paper trading test results
- Document the strategy logic in a docstring

### 🔌 Broker Adapters
- Place adapters in `trading-core/pts/adapters/`
- Follow the existing `BrokerAdapter` interface
- Include connection, order placement, and WebSocket methods
- Test with paper trading first

### 📖 Documentation
- Architecture docs go in `platform-docs/`
- Follow the existing PTS-XXX numbering convention
- Use clear, concise language

---

## Pull Request Process

1. **Ensure** your branch is up to date with `main`:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```
2. **Test** your changes locally
3. **Write** clear commit messages
4. **Open** a PR with a descriptive title and explanation
5. **Link** any related issues
6. **Wait** for review — we aim to respond within 48 hours

---

## Coding Standards

### Python
- Follow [PEP 8](https://pep8.org/)
- Use type hints
- Write docstrings for public functions
- Use `async/await` for I/O operations

### JavaScript/React
- Use functional components with hooks
- Follow ESLint configuration
- Use descriptive component and variable names

### General
- Keep functions small and focused
- Prefer composition over inheritance
- Write self-documenting code
- Add comments only for non-obvious logic

---

## Architecture Guidelines

AutoTrader follows an **event-driven architecture**. When contributing:

1. **Services communicate through NATS events** — never through direct function calls between services
2. **Events are immutable** — never modify an event after publishing
3. **Follow existing event contracts** — see [PTS-002](platform-docs/PTS-002-Event-Model-and-Message-Contracts-v0.1.md)
4. **Redis is for hot state, ClickHouse is for history** — see [PTS-003](platform-docs/PTS-003-Data-Model-and-Storage-Architecture-v0.1.md)
5. **The execution engine must never depend on the dashboard**

---

## Questions?

Open a GitHub Issue or start a Discussion. We're happy to help!
]]>
