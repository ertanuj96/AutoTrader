"""System health and kill switch — PTS-002 subjects."""

from __future__ import annotations

import orjson
from fastapi import APIRouter

from app.deps import get_redis, get_nats

router = APIRouter()


@router.get("/health")
async def health():
    checks = {}
    try:
        r = await get_redis()
        await r.ping()
        checks["redis"] = "ok"
    except Exception as e:
        checks["redis"] = f"error: {e}"

    try:
        nc = await get_nats()
        checks["nats"] = "ok" if nc.is_connected else "disconnected"
    except Exception as e:
        checks["nats"] = f"error: {e}"

    all_ok = all(v == "ok" for v in checks.values())
    return {"status": "healthy" if all_ok else "degraded", "checks": checks}


@router.post("/kill-switch")
async def activate_kill_switch():
    nc = await get_nats()
    # PTS-002: pts.system.kill_switch
    await nc.publish("pts.system.kill_switch", orjson.dumps({
        "event_type": "KillSwitchActivated",
        "source": "dashboard-api",
        "reason": "Manual activation from dashboard",
    }))
    r = await get_redis()
    await r.set("pts:system:kill_switch", "1")
    return {"status": "kill_switch_activated"}


@router.post("/kill-switch/deactivate")
async def deactivate_kill_switch():
    r = await get_redis()
    await r.delete("pts:system:kill_switch")
    return {"status": "kill_switch_deactivated"}


@router.get("/kill-switch/status")
async def kill_switch_status():
    r = await get_redis()
    val = await r.get("pts:system:kill_switch")
    return {"active": val == b"1"}
