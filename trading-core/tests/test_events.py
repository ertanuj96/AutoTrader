"""Unit tests for PTS event models and contracts (pts/events.py).

These tests verify serialization round-trips, enum contracts, NATS subject
naming, and LatencyTracker calculations — all without any external services.
"""
from __future__ import annotations

import sys
import os
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from pts.events import (
    TickEvent, SignalGeneratedEvent, RiskApprovedEvent, RiskRejectedEvent,
    OrderSubmittedEvent, OrderFilledEvent, KillSwitchEvent,
    PositionClosedEvent, PortfolioUpdatedEvent,
    Side, OrderType, OrderStatus, StrategyState, Product,
    Subjects, LatencyTracker, EventEnvelope,
)


# ── EventEnvelope ──

class TestEventEnvelope:
    def test_auto_generates_event_id(self):
        e1, e2 = EventEnvelope(), EventEnvelope()
        assert e1.event_id != e2.event_id

    def test_serialization_round_trip(self):
        env = EventEnvelope(event_type="TestEvent", source="test")
        restored = EventEnvelope.from_bytes(env.to_bytes())
        assert restored.event_id == env.event_id
        assert restored.event_type == env.event_type
        assert restored.source == env.source

    def test_to_bytes_is_bytes(self):
        assert isinstance(EventEnvelope().to_bytes(), bytes)

    def test_timestamp_is_iso_format(self):
        env = EventEnvelope()
        assert "T" in env.timestamp
        assert env.timestamp.endswith("Z")

    def test_timestamp_ms_is_positive_integer(self):
        env = EventEnvelope()
        assert isinstance(env.timestamp_ms, int)
        assert env.timestamp_ms > 0


# ── TickEvent ──

class TestTickEvent:
    def test_instrument_property(self):
        tick = TickEvent(symbol="NIFTY26JUN25000CE", exchange="NSE", ltp=150.0)
        assert tick.instrument == "NSE:NIFTY26JUN25000CE"

    def test_default_exchange_is_nse(self):
        tick = TickEvent(symbol="NIFTY")
        assert tick.exchange == "NSE"

    def test_serialization_round_trip(self):
        tick = TickEvent(symbol="NIFTY", ltp=18432.5, volume=5000, oi=150000)
        restored = TickEvent.from_bytes(tick.to_bytes())
        assert restored.symbol == "NIFTY"
        assert restored.ltp == 18432.5
        assert restored.volume == 5000

    def test_optional_fields_default_none(self):
        tick = TickEvent(symbol="NIFTY")
        assert tick.bid is None
        assert tick.ask is None
        assert tick.bid_qty is None


# ── SignalGeneratedEvent ──

class TestSignalGeneratedEvent:
    def test_side_property_buy(self):
        sig = SignalGeneratedEvent(action="BUY", symbol="NIFTY")
        assert sig.side == Side.BUY

    def test_side_property_sell(self):
        sig = SignalGeneratedEvent(action="SELL", symbol="NIFTY")
        assert sig.side == Side.SELL

    def test_instrument_property(self):
        sig = SignalGeneratedEvent(symbol="BANKNIFTY", exchange="NSE")
        assert sig.instrument == "NSE:BANKNIFTY"

    def test_unique_signal_ids(self):
        s1 = SignalGeneratedEvent(symbol="NIFTY")
        s2 = SignalGeneratedEvent(symbol="NIFTY")
        assert s1.signal_id != s2.signal_id

    def test_serialization_preserves_confidence(self):
        sig = SignalGeneratedEvent(symbol="NIFTY", confidence=0.78)
        restored = SignalGeneratedEvent.from_bytes(sig.to_bytes())
        assert abs(restored.confidence - 0.78) < 1e-6

    def test_default_product_is_mis(self):
        sig = SignalGeneratedEvent(symbol="NIFTY")
        assert sig.product == Product.MIS

    def test_default_order_type_is_market(self):
        sig = SignalGeneratedEvent(symbol="NIFTY")
        assert sig.order_type == OrderType.MARKET


# ── Risk events ──

class TestRiskEvents:
    def test_approved_event_round_trip(self):
        approved = RiskApprovedEvent(
            signal_id="sig-001",
            approved=True,
            checks_passed=["daily_loss_ok", "position_size_ok"],
        )
        restored = RiskApprovedEvent.from_bytes(approved.to_bytes())
        assert restored.signal_id == "sig-001"
        assert restored.approved is True
        assert "daily_loss_ok" in restored.checks_passed

    def test_rejected_event_preserves_reason(self):
        rej = RiskRejectedEvent(
            signal_id="sig-002",
            reason="daily_loss_limit",
            checks_failed=["daily_loss_limit"],
        )
        restored = RiskRejectedEvent.from_bytes(rej.to_bytes())
        assert restored.reason == "daily_loss_limit"


# ── OrderFilledEvent ──

class TestOrderFilledEvent:
    def test_instrument_property(self):
        filled = OrderFilledEvent(symbol="NIFTY", exchange="NSE")
        assert filled.instrument == "NSE:NIFTY"

    def test_round_trip_preserves_fill_data(self):
        filled = OrderFilledEvent(
            order_id="ord-001",
            filled_qty=50,
            average_price=18200.0,
            side=Side.BUY,
        )
        restored = OrderFilledEvent.from_bytes(filled.to_bytes())
        assert restored.filled_qty == 50
        assert restored.average_price == 18200.0


# ── PositionClosedEvent ──

class TestPositionClosedEvent:
    def test_realized_pnl_preserved(self):
        closed = PositionClosedEvent(
            symbol="NIFTY",
            entry_price=18000.0,
            exit_price=18200.0,
            quantity=50,
            realized_pnl=10000.0,
        )
        restored = PositionClosedEvent.from_bytes(closed.to_bytes())
        assert restored.realized_pnl == 10000.0


# ── LatencyTracker ──

class TestLatencyTracker:
    def test_all_none_when_empty(self):
        lt = LatencyTracker()
        assert lt.strategy_latency_ms() is None
        assert lt.risk_latency_ms() is None
        assert lt.broker_latency_ms() is None
        assert lt.fill_latency_ms() is None
        assert lt.total_latency_ms() is None

    def test_strategy_latency_computed(self):
        lt = LatencyTracker()
        lt.tick_received_time = 1000
        lt.signal_generated_time = 1050
        assert lt.strategy_latency_ms() == 50

    def test_risk_latency_computed(self):
        lt = LatencyTracker()
        lt.signal_generated_time = 1050
        lt.risk_approved_time = 1060
        assert lt.risk_latency_ms() == 10

    def test_total_latency_computed(self):
        lt = LatencyTracker()
        lt.tick_received_time = 1000
        lt.fill_time = 1250
        assert lt.total_latency_ms() == 250

    def test_partial_pipeline_returns_none(self):
        lt = LatencyTracker()
        lt.tick_received_time = 1000
        # fill_time not set → total is None
        assert lt.total_latency_ms() is None


# ── Subjects ──

class TestSubjects:
    def test_well_known_subjects_match_spec(self):
        assert Subjects.TICK == "pts.market.tick"
        assert Subjects.SIGNAL_GENERATED == "pts.signal.generated"
        assert Subjects.RISK_APPROVED == "pts.risk.approved"
        assert Subjects.ORDER_FILLED == "pts.order.filled"
        assert Subjects.KILL_SWITCH == "pts.system.kill_switch"

    def test_strategy_control_builder(self):
        assert Subjects.strategy_control("start") == "pts.strategy.start"
        assert Subjects.strategy_control("pause") == "pts.strategy.pause"


# ── Enum contracts ──

class TestEnums:
    def test_side_values(self):
        assert Side.BUY == "BUY"
        assert Side.SELL == "SELL"

    def test_order_type_values(self):
        assert OrderType.MARKET == "MARKET"
        assert OrderType.LIMIT == "LIMIT"

    def test_strategy_state_values(self):
        assert StrategyState.RUNNING == "RUNNING"
        assert StrategyState.PAUSED == "PAUSED"
        assert StrategyState.STOPPED == "STOPPED"

    def test_product_values(self):
        assert Product.MIS == "MIS"
        assert Product.NRML == "NRML"
