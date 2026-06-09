"""PTS Strategy Engine — Per PTS-002 event contracts."""

from __future__ import annotations

import asyncio
import logging
import time
from abc import ABC, abstractmethod
from typing import Optional

import nats
import orjson

from pts.config import settings
from pts.events import (
    TickEvent, SignalGeneratedEvent, Subjects,
    OrderType, Product, StrategyState, StrategyCommand,
)
from pts.state_manager import StateManager
from pts.metrics import signals_total

logger = logging.getLogger("pts.strategy")


class EMA:
    """Exponential Moving Average calculator."""
    def __init__(self, period: int):
        self.period = period
        self.multiplier = 2.0 / (period + 1)
        self.value: Optional[float] = None
        self._count = 0
        self._sum = 0.0

    def update(self, price: float) -> Optional[float]:
        self._count += 1
        if self._count <= self.period:
            self._sum += price
            if self._count == self.period:
                self.value = self._sum / self.period
            return self.value
        elif self.value is not None:
            self.value = (price - self.value) * self.multiplier + self.value
        return self.value

    def is_ready(self) -> bool:
        return self.value is not None


class Strategy(ABC):
    """Abstract base class for all trading strategies."""
    def __init__(self, strategy_id: str, instruments: list[str]):
        self.strategy_id = strategy_id
        self.instruments = instruments
        self.state = StrategyState.STOPPED
        self._positions: dict[str, dict] = {}

    @abstractmethod
    async def on_tick(self, tick: TickEvent) -> Optional[SignalGeneratedEvent]:
        ...

    def is_active(self) -> bool:
        return self.state == StrategyState.RUNNING


class EMACrossoverStrategy(Strategy):
    """EMA Crossover: BUY on bullish cross, SELL on bearish cross."""

    def __init__(self, strategy_id: str = "ema_crossover", instruments: Optional[list[str]] = None,
                 fast_period: int = 0, slow_period: int = 0, quantity: int = 1):
        instruments = instruments or settings.instruments
        super().__init__(strategy_id, instruments)
        self.fast_period = fast_period or settings.ema_fast_period
        self.slow_period = slow_period or settings.ema_slow_period
        self.quantity = quantity
        self._fast_emas: dict[str, EMA] = {}
        self._slow_emas: dict[str, EMA] = {}
        self._prev_fast_above: dict[str, Optional[bool]] = {}

        for inst in instruments:
            self._fast_emas[inst] = EMA(self.fast_period)
            self._slow_emas[inst] = EMA(self.slow_period)
            self._prev_fast_above[inst] = None

        logger.info(f"📈 EMACrossover: fast={self.fast_period}, slow={self.slow_period}, instruments={instruments}")

    async def on_tick(self, tick: TickEvent) -> Optional[SignalGeneratedEvent]:
        inst = tick.instrument
        if inst not in self._fast_emas:
            return None

        fast_val = self._fast_emas[inst].update(tick.ltp)
        slow_val = self._slow_emas[inst].update(tick.ltp)

        if not self._fast_emas[inst].is_ready() or not self._slow_emas[inst].is_ready():
            return None

        fast_above = fast_val > slow_val
        prev = self._prev_fast_above[inst]
        self._prev_fast_above[inst] = fast_above

        if prev is None:
            return None

        in_position = inst in self._positions

        if fast_above and not prev and not in_position:
            self._positions[inst] = {"entry_price": tick.ltp, "entry_time": tick.timestamp_ms}
            signal = SignalGeneratedEvent(
                strategy_id=self.strategy_id, symbol=tick.symbol, exchange=tick.exchange,
                action="BUY", quantity=self.quantity, order_type=OrderType.MARKET,
                product=Product.MIS, confidence=0.7,
                reason=f"EMA crossover BUY: fast({fast_val:.2f}) > slow({slow_val:.2f})",
            )
            logger.info(f"📈 Signal: {signal.reason} @ {tick.ltp}")
            signals_total.labels(strategy=self.strategy_id, side="BUY").inc()
            return signal

        elif not fast_above and prev and in_position:
            self._positions.pop(inst)
            signal = SignalGeneratedEvent(
                strategy_id=self.strategy_id, symbol=tick.symbol, exchange=tick.exchange,
                action="SELL", quantity=self.quantity, order_type=OrderType.MARKET,
                product=Product.MIS, confidence=0.7,
                reason=f"EMA crossover SELL: fast({fast_val:.2f}) < slow({slow_val:.2f})",
            )
            logger.info(f"📈 Signal: {signal.reason} @ {tick.ltp}")
            signals_total.labels(strategy=self.strategy_id, side="SELL").inc()
            return signal

        return None


class StrategyEngine:
    """Manages strategy lifecycle, tick distribution, and NATS subscriptions."""

    def __init__(self, nc: nats.NATS, state: StateManager):
        self._nc = nc
        self._state = state
        self._strategies: dict[str, Strategy] = {}
        self._running = False

    def register(self, strategy: Strategy):
        self._strategies[strategy.strategy_id] = strategy
        logger.info(f"📈 Strategy registered: {strategy.strategy_id}")

    async def start(self):
        self._running = True

        for sid, strategy in self._strategies.items():
            strategy.state = StrategyState.RUNNING
            await self._state.set_strategy_state(sid, StrategyState.RUNNING.value)

        # Subscribe to tick events (PTS-002 subject)
        await self._nc.subscribe(Subjects.TICK, cb=self._on_tick)

        # Subscribe to strategy control commands
        for subj in [Subjects.STRATEGY_START, Subjects.STRATEGY_STOP,
                     Subjects.STRATEGY_PAUSE, Subjects.STRATEGY_RESUME]:
            await self._nc.subscribe(subj, cb=self._on_control)

        logger.info(f"📈 Strategy engine started with {len(self._strategies)} strategies")
        while self._running:
            await asyncio.sleep(1)

    async def stop(self):
        self._running = False
        for sid, strategy in self._strategies.items():
            strategy.state = StrategyState.STOPPED
            await self._state.set_strategy_state(sid, StrategyState.STOPPED.value)

    async def _on_tick(self, msg):
        try:
            tick = TickEvent.from_bytes(msg.data)
        except Exception as e:
            logger.error(f"📈 Tick parse error: {e}")
            return

        for strategy in self._strategies.values():
            if not strategy.is_active():
                continue
            if tick.instrument not in strategy.instruments:
                continue

            signal = await strategy.on_tick(tick)
            if signal:
                await self._nc.publish(Subjects.SIGNAL_GENERATED, signal.to_bytes())

    async def _on_control(self, msg):
        try:
            cmd = StrategyCommand.from_bytes(msg.data)
            sid = cmd.strategy_id
            action = cmd.action

            if sid not in self._strategies:
                return

            strategy = self._strategies[sid]
            if action in ("start", "resume"):
                strategy.state = StrategyState.RUNNING
            elif action == "pause":
                strategy.state = StrategyState.PAUSED
            elif action == "stop":
                strategy.state = StrategyState.STOPPED

            await self._state.set_strategy_state(sid, strategy.state.value)
            logger.info(f"📈 Strategy {sid} → {strategy.state.value}")
        except Exception as e:
            logger.error(f"📈 Control error: {e}")

    def get_strategies_info(self) -> list[dict]:
        return [
            {"strategy_id": sid, "state": s.state.value,
             "instruments": s.instruments, "type": s.__class__.__name__}
            for sid, s in self._strategies.items()
        ]
