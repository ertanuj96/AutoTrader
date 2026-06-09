# PTS-002
# Event Model & Message Contracts
## Personal Trading System (PTS)

Version: 0.1

---

# Purpose

This document defines the canonical event contracts used across all PTS services.

Goals:

- Decouple services
- Enable horizontal scaling
- Maintain auditability
- Provide replay capability
- Ensure broker independence

All services communicate through NATS events.

---

# Event Principles

1. Events are immutable.
2. Every event contains metadata.
3. Services communicate only through contracts.
4. Consumers never assume publisher internals.
5. Events must be versioned.

---

# Standard Event Envelope

```json
{
  "event_id": "uuid",
  "event_type": "TickEvent",
  "event_version": "1.0",
  "source": "market-data-service",
  "timestamp": "2026-06-01T09:15:00.123456Z",
  "payload": {}
}
```

---

# NATS Subject Naming Convention

```text
pts.market.*
pts.signal.*
pts.risk.*
pts.order.*
pts.position.*
pts.portfolio.*
pts.strategy.*
pts.system.*
```

Examples:

```text
pts.market.tick
pts.signal.generated
pts.order.submitted
pts.order.filled
pts.position.updated
```

---

# Market Events

## TickEvent

Subject:

```text
pts.market.tick
```

Payload:

```json
{
  "symbol": "NIFTY26JUN25000CE",
  "exchange": "NSE",
  "ltp": 123.45,
  "bid": 123.40,
  "ask": 123.50,
  "bid_qty": 500,
  "ask_qty": 300,
  "volume": 100000,
  "oi": 250000
}
```

---

## OptionChainEvent

Subject:

```text
pts.market.option_chain
```

Payload:

```json
{
  "underlying": "NIFTY",
  "expiry": "2026-06-25",
  "spot_price": 25010.0,
  "contracts": []
}
```

---

## GreeksEvent

Subject:

```text
pts.market.greeks
```

Payload:

```json
{
  "symbol": "NIFTY26JUN25000CE",
  "iv": 12.5,
  "delta": 0.52,
  "gamma": 0.004,
  "theta": -2.1,
  "vega": 8.3
}
```

---

# Signal Events

## SignalGeneratedEvent

Subject:

```text
pts.signal.generated
```

Payload:

```json
{
  "strategy_id": "OPTION_SCALPER",
  "signal_id": "uuid",
  "action": "BUY",
  "symbol": "NIFTY26JUN25000CE",
  "quantity": 75,
  "confidence": 0.82,
  "reason": "Momentum breakout"
}
```

---

## SignalCancelledEvent

Subject:

```text
pts.signal.cancelled
```

Payload:

```json
{
  "signal_id": "uuid",
  "reason": "Market conditions changed"
}
```

---

# Risk Events

## RiskApprovedEvent

Subject:

```text
pts.risk.approved
```

Payload:

```json
{
  "signal_id": "uuid",
  "approved": true
}
```

---

## RiskRejectedEvent

Subject:

```text
pts.risk.rejected
```

Payload:

```json
{
  "signal_id": "uuid",
  "reason": "Margin exceeded"
}
```

---

# Order Events

## OrderSubmittedEvent

Subject:

```text
pts.order.submitted
```

Payload:

```json
{
  "order_id": "uuid",
  "strategy_id": "OPTION_SCALPER",
  "symbol": "NIFTY26JUN25000CE",
  "side": "BUY",
  "quantity": 75,
  "price": 123.50
}
```

---

## OrderAcknowledgedEvent

Subject:

```text
pts.order.acknowledged
```

Payload:

```json
{
  "order_id": "uuid",
  "broker_order_id": "ABC123"
}
```

---

## OrderFilledEvent

Subject:

```text
pts.order.filled
```

Payload:

```json
{
  "order_id": "uuid",
  "filled_qty": 75,
  "average_price": 123.45
}
```

---

## OrderCancelledEvent

Subject:

```text
pts.order.cancelled
```

Payload:

```json
{
  "order_id": "uuid",
  "reason": "User request"
}
```

---

# Position Events

## PositionOpenedEvent

Subject:

```text
pts.position.opened
```

Payload:

```json
{
  "position_id": "uuid",
  "symbol": "NIFTY26JUN25000CE",
  "quantity": 75,
  "entry_price": 123.45
}
```

---

## PositionUpdatedEvent

Subject:

```text
pts.position.updated
```

Payload:

```json
{
  "position_id": "uuid",
  "mtm": 450.0,
  "pnl": 450.0
}
```

---

## PositionClosedEvent

Subject:

```text
pts.position.closed
```

Payload:

```json
{
  "position_id": "uuid",
  "exit_price": 130.25,
  "realized_pnl": 510.0
}
```

---

# Portfolio Events

## PortfolioUpdatedEvent

Subject:

```text
pts.portfolio.updated
```

Payload:

```json
{
  "net_pnl": 12000,
  "gross_exposure": 450000,
  "margin_used": 175000,
  "delta": 220,
  "gamma": 1.2,
  "theta": -1500,
  "vega": 450
}
```

---

# Strategy Control Events

## StrategyStartCommand

Subject:

```text
pts.strategy.start
```

Payload:

```json
{
  "strategy_id": "OPTION_SCALPER"
}
```

## StrategyStopCommand

Subject:

```text
pts.strategy.stop
```

Payload:

```json
{
  "strategy_id": "OPTION_SCALPER"
}
```

## StrategyPauseCommand

Subject:

```text
pts.strategy.pause
```

Payload:

```json
{
  "strategy_id": "OPTION_SCALPER"
}
```

## StrategyResumeCommand

Subject:

```text
pts.strategy.resume
```

Payload:

```json
{
  "strategy_id": "OPTION_SCALPER"
}
```

---

# System Events

## KillSwitchActivated

Subject:

```text
pts.system.kill_switch
```

Payload:

```json
{
  "reason": "Daily drawdown exceeded"
}
```

---

# Latency Tracking

Every order lifecycle should track:

```text
tick_received_time
signal_generated_time
risk_approved_time
order_sent_time
broker_ack_time
fill_time
```

Derived Metrics:

```text
Strategy Latency
Risk Latency
Broker Latency
Fill Latency
End-to-End Latency
```

---

# Event Replay

Supported event categories:

- Market Events
- Signal Events
- Risk Events
- Order Events
- Position Events
- Portfolio Events

Replay source:

ClickHouse historical storage.

---

# Versioning Rules

Backward compatible changes:

- Add optional fields

Breaking changes:

- Remove fields
- Rename fields
- Change data types

Breaking changes require:

```text
event_version += 1
```

---

# Guiding Principle

Services communicate through events, never through implementation details.
The event model is the contract that allows independent evolution of
Python strategies, Rust execution engines, dashboards, storage systems,
and future broker integrations.
