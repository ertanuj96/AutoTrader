"""Order and trade history — PTS-003 ClickHouse schema."""

from __future__ import annotations

import orjson
from fastapi import APIRouter, Query

from app.deps import get_clickhouse, get_nats

router = APIRouter()


@router.get("/orders")
async def get_orders(limit: int = Query(default=50, le=500)):
    ch = get_clickhouse()
    try:
        result = ch.query(f"SELECT * FROM pts_trading.order_history ORDER BY ts DESC LIMIT {limit}")
        cols = result.column_names
        rows = [dict(zip(cols, row)) for row in result.result_rows]
        # Serialize UUID and datetime for JSON
        for row in rows:
            for k, v in row.items():
                if hasattr(v, 'isoformat'):
                    row[k] = v.isoformat()
                elif hasattr(v, 'hex'):
                    row[k] = str(v)
        return {"orders": rows, "count": len(rows)}
    except Exception as e:
        return {"orders": [], "count": 0, "error": str(e)}


@router.get("/trades")
async def get_trades(limit: int = Query(default=50, le=500)):
    ch = get_clickhouse()
    try:
        result = ch.query(f"SELECT * FROM pts_trading.position_history ORDER BY entry_time DESC LIMIT {limit}")
        cols = result.column_names
        rows = [dict(zip(cols, row)) for row in result.result_rows]
        for row in rows:
            for k, v in row.items():
                if hasattr(v, 'isoformat'):
                    row[k] = v.isoformat()
                elif hasattr(v, 'hex'):
                    row[k] = str(v)
        return {"trades": rows, "count": len(rows)}
    except Exception as e:
        return {"trades": [], "count": 0, "error": str(e)}


@router.post("/orders/cancel-all")
async def cancel_all_orders():
    nc = await get_nats()
    # PTS-002: pts.system.kill_switch
    await nc.publish("pts.system.kill_switch", orjson.dumps({
        "event_type": "KillSwitchActivated",
        "source": "dashboard-api",
        "reason": "Manual cancel all from dashboard",
    }))
    return {"status": "kill_switch_sent"}
