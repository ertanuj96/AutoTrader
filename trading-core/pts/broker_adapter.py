"""PTS Broker Adapter — Abstract interface + INDstocks and Paper implementations."""

from __future__ import annotations

import asyncio
import logging
import time
from abc import ABC, abstractmethod
from typing import Optional

import httpx

from pts.config import settings
from pts.events import Side, OrderType, OrderStatus, Product, _uid, _now_ms

logger = logging.getLogger("pts.broker")


# ─── Internal order result (not an event — events are published by execution engine) ───

class OrderResult:
    """Internal result from broker order placement."""
    def __init__(self, order_id: str, broker_order_id: str, instrument: str,
                 side: Side, quantity: int, order_type: OrderType,
                 limit_price: Optional[float], product: Product,
                 strategy_id: str = ""):
        self.order_id = order_id
        self.broker_order_id = broker_order_id
        self.instrument = instrument
        self.side = side
        self.quantity = quantity
        self.order_type = order_type
        self.limit_price = limit_price
        self.product = product
        self.strategy_id = strategy_id
        self.timestamp_ms = _now_ms()


class FillResult:
    """Internal result from a fill."""
    def __init__(self, order_id: str, broker_order_id: str, fill_price: float,
                 fill_quantity: int, instrument: str = "", side: Side = Side.BUY,
                 strategy_id: str = ""):
        self.order_id = order_id
        self.broker_order_id = broker_order_id
        self.fill_price = fill_price
        self.fill_quantity = fill_quantity
        self.instrument = instrument
        self.side = side
        self.strategy_id = strategy_id
        self.timestamp_ms = _now_ms()


# ─── Abstract Base ───

class BrokerAdapter(ABC):
    @abstractmethod
    async def place_order(self, instrument: str, side: Side, quantity: int,
                          order_type: OrderType, limit_price: Optional[float] = None,
                          product: Product = Product.MIS) -> OrderResult:
        ...

    @abstractmethod
    async def modify_order(self, broker_order_id: str, **kwargs) -> bool:
        ...

    @abstractmethod
    async def cancel_order(self, broker_order_id: str) -> bool:
        ...

    @abstractmethod
    async def cancel_all_orders(self) -> int:
        ...

    @abstractmethod
    async def get_positions(self) -> list[dict]:
        ...


# ─── Paper Trading Adapter ───

class PaperBrokerAdapter(BrokerAdapter):
    def __init__(self):
        self._orders: dict[str, dict] = {}
        self._fill_delay_ms: int = 50

    async def place_order(self, instrument: str, side: Side, quantity: int,
                          order_type: OrderType, limit_price: Optional[float] = None,
                          product: Product = Product.MIS) -> OrderResult:
        order_id = _uid()
        broker_order_id = f"PAPER-{_uid()[:12]}"

        self._orders[broker_order_id] = {
            "order_id": order_id, "instrument": instrument, "side": side,
            "quantity": quantity, "order_type": order_type,
            "limit_price": limit_price, "product": product,
            "status": OrderStatus.SUBMITTED, "submitted_at": _now_ms(),
        }

        logger.info(f"[PAPER] Order: {side.value} {quantity} {instrument} @ {order_type.value} | ID={broker_order_id}")

        return OrderResult(
            order_id=order_id, broker_order_id=broker_order_id,
            instrument=instrument, side=side, quantity=quantity,
            order_type=order_type, limit_price=limit_price, product=product,
        )

    async def modify_order(self, broker_order_id: str, **kwargs) -> bool:
        if broker_order_id in self._orders:
            self._orders[broker_order_id].update(kwargs)
            return True
        return False

    async def cancel_order(self, broker_order_id: str) -> bool:
        if broker_order_id in self._orders:
            self._orders[broker_order_id]["status"] = OrderStatus.CANCELLED
            return True
        return False

    async def cancel_all_orders(self) -> int:
        count = 0
        for order in self._orders.values():
            if order["status"] in (OrderStatus.SUBMITTED, OrderStatus.ACKNOWLEDGED):
                order["status"] = OrderStatus.CANCELLED
                count += 1
        return count

    async def get_positions(self) -> list[dict]:
        return []

    async def simulate_fill(self, broker_order_id: str, fill_price: float) -> Optional[FillResult]:
        order = self._orders.get(broker_order_id)
        if not order:
            return None
        await asyncio.sleep(self._fill_delay_ms / 1000)
        order["status"] = OrderStatus.FILLED
        return FillResult(
            order_id=order["order_id"], broker_order_id=broker_order_id,
            fill_price=fill_price, fill_quantity=order["quantity"],
            instrument=order["instrument"], side=order["side"],
        )


# ─── INDstocks Live Adapter ───

class INDstocksBrokerAdapter(BrokerAdapter):
    ORDER_RATE_LIMIT = 10

    def __init__(self):
        self._client = httpx.AsyncClient(
            base_url=settings.indstocks_base_url,
            headers={"Authorization": settings.indstocks_api_token, "Content-Type": "application/json"},
            timeout=10.0,
        )
        self._last_order_time = 0.0
        self._order_count_in_second = 0

    async def _rate_limit(self):
        now = time.time()
        if now - self._last_order_time < 1.0:
            self._order_count_in_second += 1
            if self._order_count_in_second >= self.ORDER_RATE_LIMIT:
                sleep_time = 1.0 - (now - self._last_order_time)
                if sleep_time > 0:
                    await asyncio.sleep(sleep_time)
                self._order_count_in_second = 0
                self._last_order_time = time.time()
        else:
            self._order_count_in_second = 1
            self._last_order_time = now

    async def place_order(self, instrument: str, side: Side, quantity: int,
                          order_type: OrderType, limit_price: Optional[float] = None,
                          product: Product = Product.MIS) -> OrderResult:
        await self._rate_limit()
        parts = instrument.split(":")
        exchange = parts[0] if len(parts) > 1 else "NSE"
        security_id = parts[1] if len(parts) > 1 else parts[0]

        payload = {
            "txn_type": side.value, "exchange": exchange, "segment": "FNO",
            "security_id": security_id, "qty": quantity,
            "order_type": order_type.value, "validity": "DAY",
            "product": product.value, "is_amo": False,
        }
        if limit_price and order_type in (OrderType.LIMIT, OrderType.SL):
            payload["limit_price"] = limit_price

        order_id = _uid()
        resp = await self._client.post("/order", json=payload)
        resp.raise_for_status()
        data = resp.json()
        broker_order_id = data.get("order_id", _uid())

        logger.info(f"[LIVE] Order: {side.value} {quantity} {instrument} | broker_id={broker_order_id}")
        return OrderResult(
            order_id=order_id, broker_order_id=broker_order_id,
            instrument=instrument, side=side, quantity=quantity,
            order_type=order_type, limit_price=limit_price, product=product,
        )

    async def modify_order(self, broker_order_id: str, **kwargs) -> bool:
        await self._rate_limit()
        try:
            resp = await self._client.put("/order", json={"order_id": broker_order_id, **kwargs})
            resp.raise_for_status()
            return True
        except httpx.HTTPError:
            return False

    async def cancel_order(self, broker_order_id: str) -> bool:
        await self._rate_limit()
        try:
            resp = await self._client.delete(f"/order/{broker_order_id}")
            resp.raise_for_status()
            return True
        except httpx.HTTPError:
            return False

    async def cancel_all_orders(self) -> int:
        logger.warning("[LIVE] cancel_all_orders — not implemented for INDstocks")
        return 0

    async def get_positions(self) -> list[dict]:
        try:
            resp = await self._client.get("/portfolio/positions")
            resp.raise_for_status()
            return resp.json().get("data", [])
        except httpx.HTTPError:
            return []

    async def close(self):
        await self._client.aclose()


def create_broker_adapter() -> BrokerAdapter:
    if settings.paper_trade:
        logger.info("🔶 Using PAPER broker adapter")
        return PaperBrokerAdapter()
    else:
        logger.warning("🔴 Using LIVE broker adapter — REAL ORDERS WILL BE PLACED")
        return INDstocksBrokerAdapter()
