"""PTS State Manager — Redis-backed live state per PTS-003 key design."""

from __future__ import annotations

import logging
from typing import Optional

import orjson
import redis.asyncio as aioredis

from pts.config import settings

logger = logging.getLogger("pts.state")


class StateManager:
    """Manages live trading state in Redis per PTS-003 key design."""

    # Redis keys per PTS-003
    POSITIONS_KEY = "pts:positions:active"
    PORTFOLIO_KEY = "pts:portfolio:current"
    GREEKS_KEY = "pts:greeks:portfolio"
    METRICS_KEY = "pts:metrics:latency"
    PRICES_KEY = "pts:prices:live"

    @staticmethod
    def strategy_status_key(strategy_id: str) -> str:
        return f"pts:strategy:{strategy_id}:status"

    def __init__(self):
        self._redis: Optional[aioredis.Redis] = None

    async def connect(self):
        self._redis = aioredis.from_url(
            settings.redis_url, decode_responses=False, encoding="utf-8",
        )
        await self._redis.ping()
        logger.info("✅ Redis connected")

    async def close(self):
        if self._redis:
            await self._redis.aclose()

    # ── Prices ──

    async def update_price(self, symbol: str, exchange: str, price: float):
        key = f"{exchange}:{symbol}"
        await self._redis.hset(self.PRICES_KEY, key, str(price))

    async def get_price(self, symbol: str, exchange: str = "NSE") -> Optional[float]:
        val = await self._redis.hget(self.PRICES_KEY, f"{exchange}:{symbol}")
        return float(val) if val else None

    async def get_price_by_instrument(self, instrument: str) -> Optional[float]:
        val = await self._redis.hget(self.PRICES_KEY, instrument)
        return float(val) if val else None

    async def get_all_prices(self) -> dict[str, float]:
        raw = await self._redis.hgetall(self.PRICES_KEY)
        return {k.decode(): float(v) for k, v in raw.items()}

    # ── Positions (pts:positions:active) ──

    async def update_position(self, symbol: str, strategy_id: str, position: dict):
        key = f"{strategy_id}:{symbol}"
        await self._redis.hset(self.POSITIONS_KEY, key, orjson.dumps(position))
        await self._redis.publish("pts:updates:positions", orjson.dumps({
            "type": "position_update", "symbol": symbol,
            "strategy_id": strategy_id, **position,
        }))

    async def remove_position(self, symbol: str, strategy_id: str):
        key = f"{strategy_id}:{symbol}"
        await self._redis.hdel(self.POSITIONS_KEY, key)
        await self._redis.publish("pts:updates:positions", orjson.dumps({
            "type": "position_closed", "symbol": symbol,
            "strategy_id": strategy_id,
        }))

    async def get_all_positions(self) -> list[dict]:
        raw = await self._redis.hgetall(self.POSITIONS_KEY)
        positions = []
        for key, val in raw.items():
            pos = orjson.loads(val)
            parts = key.decode().split(":", 1)
            if len(parts) == 2:
                pos.setdefault("strategy_id", parts[0])
                pos.setdefault("symbol", parts[1])
            positions.append(pos)
        return positions

    # ── Portfolio (pts:portfolio:current) ──

    async def update_portfolio(self, portfolio: dict):
        await self._redis.set(self.PORTFOLIO_KEY, orjson.dumps(portfolio))
        await self._redis.publish("pts:updates:portfolio", orjson.dumps({
            "type": "portfolio_update", **portfolio,
        }))

    async def get_portfolio(self) -> dict:
        raw = await self._redis.get(self.PORTFOLIO_KEY)
        return orjson.loads(raw) if raw else {
            "net_pnl": 0, "realized_pnl": 0, "unrealized_pnl": 0,
            "gross_exposure": 0, "margin_used": 0,
        }

    # ── PnL (stored in portfolio) ──

    async def update_pnl(self, strategy_id: str, realized: float, unrealized: float):
        pnl_key = "pts:pnl:by_strategy"
        data = {"realized": realized, "unrealized": unrealized, "net": realized + unrealized}
        await self._redis.hset(pnl_key, strategy_id, orjson.dumps(data))
        await self._redis.publish("pts:updates:pnl", orjson.dumps({
            "type": "pnl_update", "strategy_id": strategy_id, **data,
        }))

    async def get_pnl(self) -> dict[str, dict]:
        raw = await self._redis.hgetall("pts:pnl:by_strategy")
        return {k.decode(): orjson.loads(v) for k, v in raw.items()}

    async def get_total_pnl(self) -> dict:
        pnl = await self.get_pnl()
        total_r = sum(p.get("realized", 0) for p in pnl.values())
        total_u = sum(p.get("unrealized", 0) for p in pnl.values())
        return {"realized": total_r, "unrealized": total_u, "net": total_r + total_u, "by_strategy": pnl}

    # ── Exposure ──

    async def update_exposure(self, strategy_id: str, exposure: float):
        await self._redis.hset("pts:exposure", strategy_id, str(exposure))

    async def get_exposure(self) -> dict[str, float]:
        raw = await self._redis.hgetall("pts:exposure")
        return {k.decode(): float(v) for k, v in raw.items()}

    # ── Strategy States (pts:strategy:{id}:status per PTS-003) ──

    async def set_strategy_state(self, strategy_id: str, state: str):
        await self._redis.set(self.strategy_status_key(strategy_id), state)
        # Also maintain a hash for easy enumeration
        await self._redis.hset("pts:strategies:all", strategy_id, state)
        await self._redis.publish("pts:updates:strategies", orjson.dumps({
            "type": "strategy_state", "strategy_id": strategy_id, "state": state,
        }))

    async def get_strategy_state(self, strategy_id: str) -> Optional[str]:
        val = await self._redis.get(self.strategy_status_key(strategy_id))
        return val.decode() if val else None

    async def get_all_strategy_states(self) -> dict[str, str]:
        raw = await self._redis.hgetall("pts:strategies:all")
        return {k.decode(): v.decode() for k, v in raw.items()}

    # ── Kill Switch ──

    async def activate_kill_switch(self):
        await self._redis.set("pts:system:kill_switch", "1")
        await self._redis.publish("pts:updates:system", orjson.dumps({
            "type": "kill_switch_activated",
        }))

    async def is_kill_switch_active(self) -> bool:
        val = await self._redis.get("pts:system:kill_switch")
        return val == b"1"

    async def deactivate_kill_switch(self):
        await self._redis.delete("pts:system:kill_switch")

    # ── Daily Stats ──

    async def update_daily_stats(self, stats: dict):
        await self._redis.set("pts:stats:daily", orjson.dumps(stats))

    async def get_daily_stats(self) -> dict:
        raw = await self._redis.get("pts:stats:daily")
        return orjson.loads(raw) if raw else {}

    @property
    def redis(self) -> aioredis.Redis:
        assert self._redis is not None, "StateManager not connected"
        return self._redis
