"""PTS Market Data Service — Live and simulated feeds per PTS-002 event contracts."""

from __future__ import annotations

import asyncio
import logging
import random
import time

import nats

from pts.config import settings
from pts.events import TickEvent, Subjects
from pts.state_manager import StateManager
from pts.persistence import Persistence
from pts.metrics import ticks_processed, tick_processing_time

logger = logging.getLogger("pts.market_data")


class MarketDataService:
    """Publishes TickEvent to pts.market.tick, persists to ClickHouse, updates Redis."""

    def __init__(self, nc: nats.NATS, state: StateManager, persistence: Persistence):
        self._nc = nc
        self._state = state
        self._persistence = persistence
        self._running = False
        self._tick_buffer: list[dict] = []
        self._flush_interval = 5.0
        self._instruments = settings.instruments  # ["NSE:NIFTY", "NSE:BANKNIFTY"]

    async def start(self):
        self._running = True
        logger.info(f"📊 Market data starting for: {self._instruments}")

        if settings.paper_trade or not settings.indstocks_api_token:
            logger.info("📊 Using SIMULATED market data feed")
            await asyncio.gather(
                self._run_simulated_feed(),
                self._flush_tick_buffer(),
            )
        else:
            logger.info("📊 Connecting to INDstocks WebSocket")
            await asyncio.gather(
                self._run_live_feed(),
                self._flush_tick_buffer(),
            )

    async def stop(self):
        self._running = False
        await self._do_flush()
        logger.info("📊 Market data stopped")

    async def _run_simulated_feed(self):
        """Generate realistic simulated ticks."""
        base_prices = {}
        for inst in self._instruments:
            parts = inst.split(":")
            exchange = parts[0] if len(parts) > 1 else "NSE"
            symbol = parts[1] if len(parts) > 1 else parts[0]
            if "NIFTY" in symbol and "BANK" not in symbol:
                base_prices[inst] = {"price": 24500.0, "symbol": symbol, "exchange": exchange}
            elif "BANKNIFTY" in symbol:
                base_prices[inst] = {"price": 52000.0, "symbol": symbol, "exchange": exchange}
            else:
                base_prices[inst] = {"price": 1000.0 + random.random() * 5000, "symbol": symbol, "exchange": exchange}

        tick_count = 0
        while self._running:
            for inst, info in base_prices.items():
                price = info["price"]
                volatility = price * 0.0002
                drift = (price * 1.0001 - price) * 0.01
                price += random.gauss(drift, volatility)
                info["price"] = price

                spread = price * 0.0001
                bid = round(price - spread / 2, 2)
                ask = round(price + spread / 2, 2)

                tick = TickEvent(
                    symbol=info["symbol"],
                    exchange=info["exchange"],
                    ltp=round(price, 2),
                    bid=bid,
                    ask=ask,
                    bid_qty=random.randint(100, 5000),
                    ask_qty=random.randint(100, 5000),
                    volume=random.randint(100, 10000),
                    oi=random.randint(50000, 500000),
                )
                await self._process_tick(tick)
                tick_count += 1

            await asyncio.sleep(0.1)
            if tick_count % 100 == 0:
                logger.debug(f"📊 Simulated {tick_count} ticks")

    async def _run_live_feed(self):
        """Connect to INDstocks WebSocket for live market data."""
        import websockets
        import json

        url = settings.indstocks_ws_url
        headers = {"Authorization": settings.indstocks_api_token}

        while self._running:
            try:
                async with websockets.connect(url, extra_headers=headers) as ws:
                    logger.info("📊 WebSocket connected to INDstocks")
                    sub_msg = {"action": "subscribe", "mode": "ltp", "instruments": self._instruments}
                    await ws.send(json.dumps(sub_msg))

                    async for message in ws:
                        if not self._running:
                            break
                        try:
                            data = json.loads(message)
                            tick = TickEvent(
                                symbol=data.get("symbol", ""),
                                exchange=data.get("exchange", "NSE"),
                                ltp=float(data.get("ltp", 0)),
                                bid=float(data.get("bid", 0)) if data.get("bid") else None,
                                ask=float(data.get("ask", 0)) if data.get("ask") else None,
                                volume=int(data.get("volume", 0)) if data.get("volume") else None,
                                oi=int(data.get("oi", 0)) if data.get("oi") else None,
                            )
                            await self._process_tick(tick)
                        except Exception as e:
                            logger.error(f"📊 Tick parse error: {e}")
            except Exception as e:
                logger.error(f"📊 WebSocket error: {e}, reconnecting in 5s...")
                await asyncio.sleep(5)

    async def _process_tick(self, tick: TickEvent):
        start = time.monotonic()

        # 1. Publish to NATS (pts.market.tick per PTS-002)
        await self._nc.publish(Subjects.TICK, tick.to_bytes())

        # 2. Update Redis with latest price
        await self._state.update_price(tick.symbol, tick.exchange, tick.ltp)

        # 3. Buffer for ClickHouse batch insert
        self._tick_buffer.append({
            "timestamp_ms": tick.timestamp_ms,
            "symbol": tick.symbol,
            "exchange": tick.exchange,
            "ltp": tick.ltp,
            "bid": tick.bid or 0.0,
            "ask": tick.ask or 0.0,
            "bid_qty": tick.bid_qty or 0,
            "ask_qty": tick.ask_qty or 0,
            "volume": tick.volume or 0,
            "oi": tick.oi or 0,
        })

        elapsed = time.monotonic() - start
        ticks_processed.labels(instrument=tick.instrument).inc()
        tick_processing_time.observe(elapsed)

    async def _flush_tick_buffer(self):
        while self._running:
            await asyncio.sleep(self._flush_interval)
            await self._do_flush()

    async def _do_flush(self):
        if self._tick_buffer:
            batch = self._tick_buffer.copy()
            self._tick_buffer.clear()
            try:
                self._persistence.insert_ticks_batch(batch)
                logger.debug(f"📊 Flushed {len(batch)} ticks to ClickHouse")
            except Exception as e:
                logger.error(f"📊 ClickHouse flush error: {e}")
                self._tick_buffer.extend(batch)
