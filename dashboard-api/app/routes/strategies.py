"""Strategy management routes — PTS-002 strategy control events."""

from __future__ import annotations

import orjson
from fastapi import APIRouter

from app.deps import get_redis, get_nats

router = APIRouter()


@router.get("/strategies")
async def list_strategies():
    r = await get_redis()
    raw = await r.hgetall("pts:strategies:all")
    strategies = []
    for sid, state in raw.items():
        strategies.append({
            "strategy_id": sid.decode(),
            "state": state.decode(),
        })
    return {"strategies": strategies}


@router.post("/strategies/{strategy_id}/start")
async def start_strategy(strategy_id: str):
    nc = await get_nats()
    # PTS-002: pts.strategy.start
    await nc.publish("pts.strategy.start", orjson.dumps({
        "event_type": "StrategyCommand",
        "source": "dashboard-api",
        "strategy_id": strategy_id,
        "action": "start",
    }))
    r = await get_redis()
    await r.set(f"pts:strategy:{strategy_id}:status", "RUNNING")
    await r.hset("pts:strategies:all", strategy_id, "RUNNING")
    return {"status": "ok", "strategy_id": strategy_id, "state": "RUNNING"}


@router.post("/strategies/{strategy_id}/stop")
async def stop_strategy(strategy_id: str):
    nc = await get_nats()
    await nc.publish("pts.strategy.stop", orjson.dumps({
        "event_type": "StrategyCommand",
        "source": "dashboard-api",
        "strategy_id": strategy_id,
        "action": "stop",
    }))
    r = await get_redis()
    await r.set(f"pts:strategy:{strategy_id}:status", "STOPPED")
    await r.hset("pts:strategies:all", strategy_id, "STOPPED")
    return {"status": "ok", "strategy_id": strategy_id, "state": "STOPPED"}


@router.post("/strategies/{strategy_id}/pause")
async def pause_strategy(strategy_id: str):
    nc = await get_nats()
    await nc.publish("pts.strategy.pause", orjson.dumps({
        "event_type": "StrategyCommand",
        "source": "dashboard-api",
        "strategy_id": strategy_id,
        "action": "pause",
    }))
    r = await get_redis()
    await r.set(f"pts:strategy:{strategy_id}:status", "PAUSED")
    await r.hset("pts:strategies:all", strategy_id, "PAUSED")
    return {"status": "ok", "strategy_id": strategy_id, "state": "PAUSED"}
