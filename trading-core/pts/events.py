"""PTS Event Models — Canonical event contracts per PTS-002 spec."""

from __future__ import annotations

import time
from enum import Enum
from typing import Optional, Any

import orjson
from pydantic import BaseModel, Field


def _now_iso() -> str:
    """Current time in ISO 8601 with microseconds."""
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%fZ")


def _now_ms() -> int:
    """Current time in milliseconds since epoch."""
    return int(time.time() * 1000)


def _uid() -> str:
    """Generate a UUID."""
    import uuid
    return str(uuid.uuid4())


# ─── Enums ───

class Side(str, Enum):
    BUY = "BUY"
    SELL = "SELL"


class OrderType(str, Enum):
    MARKET = "MARKET"
    LIMIT = "LIMIT"
    SL = "SL"
    SL_MARKET = "SL_MARKET"


class OrderStatus(str, Enum):
    SUBMITTED = "SUBMITTED"
    ACKNOWLEDGED = "ACKNOWLEDGED"
    PARTIAL_FILL = "PARTIAL_FILL"
    FILLED = "FILLED"
    CANCELLED = "CANCELLED"
    REJECTED = "REJECTED"


class StrategyState(str, Enum):
    RUNNING = "RUNNING"
    PAUSED = "PAUSED"
    STOPPED = "STOPPED"


class Product(str, Enum):
    MIS = "MIS"       # Intraday
    NRML = "NRML"     # Carry forward
    CNC = "CNC"       # Delivery


# ─── Standard Event Envelope (PTS-002) ───

class EventEnvelope(BaseModel):
    """Standard event envelope per PTS-002 contract."""
    event_id: str = Field(default_factory=_uid)
    event_type: str = ""
    event_version: str = "1.0"
    source: str = ""
    timestamp: str = Field(default_factory=_now_iso)
    timestamp_ms: int = Field(default_factory=_now_ms)
    payload: dict = Field(default_factory=dict)

    def to_bytes(self) -> bytes:
        return orjson.dumps(self.model_dump())

    @classmethod
    def from_bytes(cls, data: bytes):
        return cls.model_validate(orjson.loads(data))


# ─── Market Events ───

class TickEvent(EventEnvelope):
    """A normalized market tick (PTS-002: pts.market.tick)."""
    event_type: str = "TickEvent"
    source: str = "market-data-service"
    # Payload fields promoted to top level for convenience
    symbol: str = ""             # e.g. "NIFTY26JUN25000CE"
    exchange: str = "NSE"
    ltp: float = 0.0
    bid: Optional[float] = None
    ask: Optional[float] = None
    bid_qty: Optional[int] = None
    ask_qty: Optional[int] = None
    volume: Optional[int] = None
    oi: Optional[int] = None

    @property
    def instrument(self) -> str:
        """Combined key for internal use."""
        return f"{self.exchange}:{self.symbol}"


class OptionChainEvent(EventEnvelope):
    """Option chain snapshot (PTS-002: pts.market.option_chain)."""
    event_type: str = "OptionChainEvent"
    source: str = "market-data-service"
    underlying: str = ""
    expiry: str = ""
    spot_price: float = 0.0
    contracts: list[dict] = Field(default_factory=list)


class GreeksEvent(EventEnvelope):
    """Greeks snapshot (PTS-002: pts.market.greeks)."""
    event_type: str = "GreeksEvent"
    source: str = "market-data-service"
    symbol: str = ""
    iv: float = 0.0
    delta: float = 0.0
    gamma: float = 0.0
    theta: float = 0.0
    vega: float = 0.0


# ─── Signal Events ───

class SignalGeneratedEvent(EventEnvelope):
    """A trading signal from a strategy (PTS-002: pts.signal.generated)."""
    event_type: str = "SignalGeneratedEvent"
    source: str = "strategy-engine"
    strategy_id: str = ""
    signal_id: str = Field(default_factory=_uid)
    action: str = "BUY"           # BUY or SELL
    symbol: str = ""
    exchange: str = "NSE"
    quantity: int = 0
    confidence: float = 0.0
    reason: str = ""
    order_type: OrderType = OrderType.MARKET
    limit_price: Optional[float] = None
    product: Product = Product.MIS

    @property
    def side(self) -> Side:
        return Side(self.action)

    @property
    def instrument(self) -> str:
        return f"{self.exchange}:{self.symbol}"


class SignalCancelledEvent(EventEnvelope):
    """Signal cancelled (PTS-002: pts.signal.cancelled)."""
    event_type: str = "SignalCancelledEvent"
    source: str = "strategy-engine"
    signal_id: str = ""
    reason: str = ""


# ─── Risk Events ───

class RiskApprovedEvent(EventEnvelope):
    """Risk engine approved a signal (PTS-002: pts.risk.approved)."""
    event_type: str = "RiskApprovedEvent"
    source: str = "risk-engine"
    signal_id: str = ""
    signal: dict = Field(default_factory=dict)  # Original signal data
    approved: bool = True
    checks_passed: list[str] = Field(default_factory=list)


class RiskRejectedEvent(EventEnvelope):
    """Risk engine rejected a signal (PTS-002: pts.risk.rejected)."""
    event_type: str = "RiskRejectedEvent"
    source: str = "risk-engine"
    signal_id: str = ""
    reason: str = ""
    checks_failed: list[str] = Field(default_factory=list)


# ─── Order Events ───

class OrderSubmittedEvent(EventEnvelope):
    """Order submitted to broker (PTS-002: pts.order.submitted)."""
    event_type: str = "OrderSubmittedEvent"
    source: str = "execution-engine"
    order_id: str = Field(default_factory=_uid)
    strategy_id: str = ""
    symbol: str = ""
    exchange: str = "NSE"
    side: Side = Side.BUY
    quantity: int = 0
    price: float = 0.0
    order_type: OrderType = OrderType.MARKET
    product: Product = Product.MIS
    broker_order_id: Optional[str] = None

    @property
    def instrument(self) -> str:
        return f"{self.exchange}:{self.symbol}"


class OrderAcknowledgedEvent(EventEnvelope):
    """Broker acknowledged the order (PTS-002: pts.order.acknowledged)."""
    event_type: str = "OrderAcknowledgedEvent"
    source: str = "execution-engine"
    order_id: str = ""
    broker_order_id: str = ""


class OrderFilledEvent(EventEnvelope):
    """Order filled (PTS-002: pts.order.filled)."""
    event_type: str = "OrderFilledEvent"
    source: str = "execution-engine"
    order_id: str = ""
    filled_qty: int = 0
    average_price: float = 0.0
    broker_order_id: str = ""
    strategy_id: str = ""
    symbol: str = ""
    exchange: str = "NSE"
    side: Side = Side.BUY

    @property
    def instrument(self) -> str:
        return f"{self.exchange}:{self.symbol}"


class OrderCancelledEvent(EventEnvelope):
    """Order cancelled (PTS-002: pts.order.cancelled)."""
    event_type: str = "OrderCancelledEvent"
    source: str = "execution-engine"
    order_id: str = ""
    reason: str = ""


# ─── Position Events ───

class PositionOpenedEvent(EventEnvelope):
    """Position opened (PTS-002: pts.position.opened)."""
    event_type: str = "PositionOpenedEvent"
    source: str = "execution-engine"
    position_id: str = Field(default_factory=_uid)
    symbol: str = ""
    exchange: str = "NSE"
    quantity: int = 0
    entry_price: float = 0.0
    strategy_id: str = ""


class PositionUpdatedEvent(EventEnvelope):
    """Position MTM update (PTS-002: pts.position.updated)."""
    event_type: str = "PositionUpdatedEvent"
    source: str = "execution-engine"
    position_id: str = ""
    mtm: float = 0.0
    pnl: float = 0.0


class PositionClosedEvent(EventEnvelope):
    """Position fully closed (PTS-002: pts.position.closed)."""
    event_type: str = "PositionClosedEvent"
    source: str = "execution-engine"
    position_id: str = ""
    strategy_id: str = ""
    symbol: str = ""
    exchange: str = "NSE"
    exit_price: float = 0.0
    entry_price: float = 0.0
    quantity: int = 0
    realized_pnl: float = 0.0
    side: Side = Side.BUY
    holding_time_ms: int = 0


# ─── Portfolio Events ───

class PortfolioUpdatedEvent(EventEnvelope):
    """Portfolio state update (PTS-002: pts.portfolio.updated)."""
    event_type: str = "PortfolioUpdatedEvent"
    source: str = "execution-engine"
    net_pnl: float = 0.0
    gross_exposure: float = 0.0
    margin_used: float = 0.0
    delta: float = 0.0
    gamma: float = 0.0
    theta: float = 0.0
    vega: float = 0.0


# ─── Strategy Control Events ───

class StrategyCommand(EventEnvelope):
    """Strategy control command (PTS-002: pts.strategy.*)."""
    event_type: str = "StrategyCommand"
    source: str = "dashboard-api"
    strategy_id: str = ""
    action: str = ""  # start, stop, pause, resume


# ─── System Events ───

class KillSwitchEvent(EventEnvelope):
    """Kill switch activation (PTS-002: pts.system.kill_switch)."""
    event_type: str = "KillSwitchActivated"
    source: str = "risk-engine"
    reason: str = ""


# ─── NATS Subjects (PTS-002 naming convention) ───

class Subjects:
    """NATS subject naming per PTS-002."""
    # Market
    TICK = "pts.market.tick"
    OPTION_CHAIN = "pts.market.option_chain"
    GREEKS = "pts.market.greeks"

    # Signals
    SIGNAL_GENERATED = "pts.signal.generated"
    SIGNAL_CANCELLED = "pts.signal.cancelled"

    # Risk
    RISK_APPROVED = "pts.risk.approved"
    RISK_REJECTED = "pts.risk.rejected"

    # Orders
    ORDER_SUBMITTED = "pts.order.submitted"
    ORDER_ACKNOWLEDGED = "pts.order.acknowledged"
    ORDER_FILLED = "pts.order.filled"
    ORDER_CANCELLED = "pts.order.cancelled"

    # Positions
    POSITION_OPENED = "pts.position.opened"
    POSITION_UPDATED = "pts.position.updated"
    POSITION_CLOSED = "pts.position.closed"

    # Portfolio
    PORTFOLIO_UPDATED = "pts.portfolio.updated"

    # Strategy control
    STRATEGY_START = "pts.strategy.start"
    STRATEGY_STOP = "pts.strategy.stop"
    STRATEGY_PAUSE = "pts.strategy.pause"
    STRATEGY_RESUME = "pts.strategy.resume"

    # System
    KILL_SWITCH = "pts.system.kill_switch"

    @staticmethod
    def strategy_control(action: str) -> str:
        return f"pts.strategy.{action}"


# ─── Latency Tracking (PTS-002) ───

class LatencyTracker:
    """Track execution latency through the full pipeline."""
    def __init__(self):
        self.tick_received_time: Optional[int] = None
        self.signal_generated_time: Optional[int] = None
        self.risk_approved_time: Optional[int] = None
        self.order_sent_time: Optional[int] = None
        self.broker_ack_time: Optional[int] = None
        self.fill_time: Optional[int] = None

    def strategy_latency_ms(self) -> Optional[float]:
        if self.tick_received_time and self.signal_generated_time:
            return self.signal_generated_time - self.tick_received_time
        return None

    def risk_latency_ms(self) -> Optional[float]:
        if self.signal_generated_time and self.risk_approved_time:
            return self.risk_approved_time - self.signal_generated_time
        return None

    def broker_latency_ms(self) -> Optional[float]:
        if self.order_sent_time and self.broker_ack_time:
            return self.broker_ack_time - self.order_sent_time
        return None

    def fill_latency_ms(self) -> Optional[float]:
        if self.broker_ack_time and self.fill_time:
            return self.fill_time - self.broker_ack_time
        return None

    def total_latency_ms(self) -> Optional[float]:
        if self.tick_received_time and self.fill_time:
            return self.fill_time - self.tick_received_time
        return None
