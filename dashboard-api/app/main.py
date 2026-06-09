"""PTS Dashboard API — FastAPI application with WebSocket support."""

from __future__ import annotations

import asyncio
import logging

import orjson
import redis.asyncio as aioredis
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from prometheus_client import make_asgi_app

from app.deps import get_redis, get_nats
from app.routes import strategies, positions, orders, system

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s │ %(name)-20s │ %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("pts.api")

app = FastAPI(
    title="PTS Dashboard API",
    version="0.1.0",
    description="Personal Trading System — Dashboard Backend",
)

# CORS for React frontend
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173", "http://localhost:3000", "http://127.0.0.1:5173"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Mount Prometheus metrics
metrics_app = make_asgi_app()
app.mount("/metrics", metrics_app)

# Include route modules
app.include_router(strategies.router, prefix="/api", tags=["strategies"])
app.include_router(positions.router, prefix="/api", tags=["positions"])
app.include_router(orders.router, prefix="/api", tags=["orders"])
app.include_router(system.router, prefix="/api", tags=["system"])


# ── WebSocket for real-time updates ──

class ConnectionManager:
    """Manages active WebSocket connections."""

    def __init__(self):
        self.active: list[WebSocket] = []

    async def connect(self, ws: WebSocket):
        await ws.accept()
        self.active.append(ws)
        logger.info(f"WS client connected ({len(self.active)} total)")

    def disconnect(self, ws: WebSocket):
        self.active.remove(ws)
        logger.info(f"WS client disconnected ({len(self.active)} total)")

    async def broadcast(self, message: dict):
        data = orjson.dumps(message).decode()
        dead = []
        for ws in self.active:
            try:
                await ws.send_text(data)
            except Exception:
                dead.append(ws)
        for ws in dead:
            self.active.remove(ws)


manager = ConnectionManager()


@app.websocket("/ws")
async def websocket_endpoint(ws: WebSocket):
    await manager.connect(ws)
    try:
        # Start a background task to relay Redis pub/sub
        relay_task = asyncio.create_task(_relay_redis_updates(ws))
        # Keep the connection alive — listen for any client messages
        while True:
            await ws.receive_text()
    except WebSocketDisconnect:
        pass
    finally:
        manager.disconnect(ws)
        relay_task.cancel()


async def _relay_redis_updates(ws: WebSocket):
    """Subscribe to Redis pub/sub and forward updates to WebSocket client."""
    try:
        r = await get_redis()
        pubsub = r.pubsub()
        await pubsub.subscribe(
            "pts:updates:positions",
            "pts:updates:pnl",
            "pts:updates:strategies",
            "pts:updates:system",
        )
        async for message in pubsub.listen():
            if message["type"] == "message":
                try:
                    data = orjson.loads(message["data"])
                    await ws.send_text(orjson.dumps(data).decode())
                except Exception:
                    break
    except asyncio.CancelledError:
        pass
    except Exception as e:
        logger.error(f"Redis relay error: {e}")


@app.on_event("startup")
async def startup():
    logger.info("🚀 PTS Dashboard API starting...")
    await get_redis()
    logger.info("✅ Dashboard API ready")


if __name__ == "__main__":
    import uvicorn
    import os
    port = int(os.getenv("DASHBOARD_API_PORT", "8001"))
    uvicorn.run("app.main:app", host="0.0.0.0", port=port, reload=True)
