"""Positions and PnL routes — PTS-003 Redis key design."""

from __future__ import annotations

import orjson
from fastapi import APIRouter

from app.deps import get_redis

router = APIRouter()


@router.get("/positions")
async def get_positions():
    r = await get_redis()
    raw = await r.hgetall("pts:positions:active")
    positions = []
    for key, val in raw.items():
        pos = orjson.loads(val)
        parts = key.decode().split(":", 1)
        if len(parts) == 2:
            pos.setdefault("strategy_id", parts[0])
            pos.setdefault("symbol", parts[1])
        pos["key"] = key.decode()
        positions.append(pos)
    return {"positions": positions}


@router.get("/pnl")
async def get_pnl():
    r = await get_redis()
    raw = await r.hgetall("pts:pnl:by_strategy")
    by_strategy = {}
    total_realized = 0
    total_unrealized = 0
    for k, v in raw.items():
        data = orjson.loads(v)
        by_strategy[k.decode()] = data
        total_realized += data.get("realized", 0)
        total_unrealized += data.get("unrealized", 0)
    return {
        "realized": total_realized,
        "unrealized": total_unrealized,
        "net": total_realized + total_unrealized,
        "by_strategy": by_strategy,
    }


@router.get("/exposure")
async def get_exposure():
    r = await get_redis()
    raw = await r.hgetall("pts:exposure")
    exposure = {k.decode(): float(v) for k, v in raw.items()}
    return {"exposure": exposure, "total": sum(exposure.values())}


@router.get("/prices")
async def get_prices():
    r = await get_redis()
    raw = await r.hgetall("pts:prices:live")
    prices = {k.decode(): float(v) for k, v in raw.items()}
    return {"prices": prices}


@router.get("/portfolio")
async def get_portfolio():
    r = await get_redis()
    raw = await r.get("pts:portfolio:current")
    portfolio = orjson.loads(raw) if raw else {
        "net_pnl": 0, "realized_pnl": 0, "unrealized_pnl": 0,
        "gross_exposure": 0, "margin_used": 0,
    }
    return {"portfolio": portfolio}
