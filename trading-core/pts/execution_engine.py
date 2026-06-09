"""PTS Execution Engine — Per PTS-002 event contracts."""

from __future__ import annotations

import asyncio
import logging
import time

import nats
import orjson

from pts.config import settings
from pts.events import (
    RiskApprovedEvent, SignalGeneratedEvent,
    OrderSubmittedEvent, OrderFilledEvent, PositionOpenedEvent, PositionClosedEvent,
    Subjects, OrderStatus, Side, OrderType, Product,
    _uid,
)
from pts.broker_adapter import BrokerAdapter, PaperBrokerAdapter
from pts.state_manager import StateManager
from pts.persistence import Persistence
from pts.metrics import (
    orders_total, signal_to_fill, signal_to_order,
    pnl_net, pnl_realized, open_positions,
)

logger = logging.getLogger("pts.execution")


class ExecutionEngine:
    """Subscribes to pts.risk.approved, places orders, tracks fills."""

    def __init__(self, nc: nats.NATS, broker: BrokerAdapter, state: StateManager, persistence: Persistence):
        self._nc = nc
        self._broker = broker
        self._state = state
        self._persistence = persistence
        self._running = False
        self._positions: dict[str, dict] = {}  # instrument -> position
        self._realized_pnl = 0.0

    async def start(self):
        self._running = True
        await self._nc.subscribe(Subjects.RISK_APPROVED, cb=self._on_risk_approved)
        await self._nc.subscribe(Subjects.KILL_SWITCH, cb=self._on_kill_switch)
        logger.info("⚡ Execution engine started")
        while self._running:
            await asyncio.sleep(1)

    async def stop(self):
        self._running = False

    async def _on_risk_approved(self, msg):
        try:
            approval = RiskApprovedEvent.from_bytes(msg.data)
        except Exception as e:
            logger.error(f"⚡ Parse error: {e}")
            return

        signal_data = approval.signal
        order_start = time.time()

        try:
            side = Side(signal_data.get("action", "BUY"))
            submitted = await self._broker.place_order(
                instrument=f"{signal_data.get('exchange', 'NSE')}:{signal_data.get('symbol', '')}",
                side=side,
                quantity=signal_data.get("quantity", 0),
                order_type=OrderType(signal_data.get("order_type", "MARKET")),
                limit_price=signal_data.get("limit_price"),
                product=Product(signal_data.get("product", "MIS")),
            )
            submitted.strategy_id = signal_data.get("strategy_id", "")

            order_latency = time.time() - order_start
            signal_to_order.observe(order_latency)

            # Publish OrderSubmitted event
            order_event = OrderSubmittedEvent(
                order_id=submitted.order_id,
                strategy_id=submitted.strategy_id,
                symbol=signal_data.get("symbol", ""),
                exchange=signal_data.get("exchange", "NSE"),
                side=side,
                quantity=signal_data.get("quantity", 0),
                price=signal_data.get("limit_price") or 0,
                order_type=OrderType(signal_data.get("order_type", "MARKET")),
                product=Product(signal_data.get("product", "MIS")),
                broker_order_id=submitted.broker_order_id,
            )
            await self._nc.publish(Subjects.ORDER_SUBMITTED, order_event.to_bytes())
            orders_total.labels(status="submitted").inc()

            # Persist order
            self._persistence.insert_order({
                "order_id": submitted.order_id,
                "broker_order_id": submitted.broker_order_id or "",
                "timestamp_ms": order_event.timestamp_ms,
                "strategy_id": submitted.strategy_id,
                "symbol": signal_data.get("symbol", ""),
                "side": side.value,
                "quantity": signal_data.get("quantity", 0),
                "price": signal_data.get("limit_price") or 0,
                "status": OrderStatus.SUBMITTED.value,
            })

            # Paper trading: simulate fill
            if isinstance(self._broker, PaperBrokerAdapter):
                instrument = f"{signal_data.get('exchange', 'NSE')}:{signal_data.get('symbol', '')}"
                price = await self._state.get_price_by_instrument(instrument) or 0
                fill = await self._broker.simulate_fill(submitted.broker_order_id, price)
                if fill:
                    fill.strategy_id = submitted.strategy_id
                    fill.instrument = instrument
                    fill.side = side
                    await self._handle_fill(fill, signal_data)
                    signal_to_fill.observe(time.time() - order_start)

            logger.info(f"⚡ Order: {side.value} {signal_data.get('quantity')} {signal_data.get('symbol')} | id={submitted.order_id}")

        except Exception as e:
            logger.error(f"⚡ Execution failed: {e}")
            orders_total.labels(status="failed").inc()

    async def _handle_fill(self, fill, signal_data: dict):
        instrument = fill.instrument
        side = fill.side
        strategy_id = fill.strategy_id
        symbol = signal_data.get("symbol", "")
        exchange = signal_data.get("exchange", "NSE")

        orders_total.labels(status="filled").inc()

        # Publish OrderFilled event
        fill_event = OrderFilledEvent(
            order_id=fill.order_id,
            filled_qty=fill.fill_quantity,
            average_price=fill.fill_price,
            broker_order_id=fill.broker_order_id,
            strategy_id=strategy_id,
            symbol=symbol,
            exchange=exchange,
            side=side,
        )
        await self._nc.publish(Subjects.ORDER_FILLED, fill_event.to_bytes())

        if instrument in self._positions:
            # Closing position
            pos = self._positions.pop(instrument)
            entry_price = pos["entry_price"]
            if pos["side"] == "BUY":
                trade_pnl = (fill.fill_price - entry_price) * fill.fill_quantity
            else:
                trade_pnl = (entry_price - fill.fill_price) * fill.fill_quantity

            self._realized_pnl += trade_pnl

            closed = PositionClosedEvent(
                position_id=pos.get("position_id", _uid()),
                strategy_id=strategy_id,
                symbol=symbol, exchange=exchange,
                exit_price=fill.fill_price,
                entry_price=entry_price,
                quantity=fill.fill_quantity,
                realized_pnl=trade_pnl,
                side=side,
                holding_time_ms=fill.timestamp_ms - pos.get("entry_time", 0),
            )
            await self._nc.publish(Subjects.POSITION_CLOSED, closed.to_bytes())
            await self._state.remove_position(symbol, strategy_id)

            self._persistence.insert_trade({
                "position_id": closed.position_id,
                "strategy_id": strategy_id,
                "symbol": symbol,
                "entry_time": pos.get("entry_time", 0),
                "exit_time": fill.timestamp_ms,
                "quantity": fill.fill_quantity,
                "entry_price": entry_price,
                "exit_price": fill.fill_price,
                "realized_pnl": trade_pnl,
            })

            logger.info(f"⚡ Position closed: {symbol} PnL={trade_pnl:+.2f}")
        else:
            # Opening position
            position_id = _uid()
            self._positions[instrument] = {
                "position_id": position_id,
                "side": side.value,
                "entry_price": fill.fill_price,
                "entry_time": fill.timestamp_ms,
                "quantity": fill.fill_quantity,
                "strategy_id": strategy_id,
            }

            opened = PositionOpenedEvent(
                position_id=position_id,
                symbol=symbol, exchange=exchange,
                quantity=fill.fill_quantity,
                entry_price=fill.fill_price,
                strategy_id=strategy_id,
            )
            await self._nc.publish(Subjects.POSITION_OPENED, opened.to_bytes())
            await self._state.update_position(symbol, strategy_id, {
                "side": side.value, "entry_price": fill.fill_price,
                "quantity": fill.fill_quantity, "unrealized_pnl": 0,
                "instrument": instrument,
            })

        open_positions.set(len(self._positions))
        pnl_realized.set(self._realized_pnl)
        pnl_net.set(self._realized_pnl)
        await self._state.update_pnl(strategy_id, self._realized_pnl, 0)

    async def _on_kill_switch(self, msg):
        logger.critical("⚡ KILL SWITCH — cancelling all orders")
        cancelled = await self._broker.cancel_all_orders()
        logger.critical(f"⚡ Cancelled {cancelled} orders")
