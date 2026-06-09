"""PTS Dashboard API — Shared dependencies."""

from __future__ import annotations

import os
from functools import lru_cache

import redis.asyncio as aioredis
import clickhouse_connect
import nats as nats_client
from dotenv import load_dotenv

load_dotenv()

REDIS_URL = os.getenv("REDIS_URL", "redis://localhost:6379")
NATS_URL = os.getenv("NATS_URL", "nats://localhost:4222")
CH_HOST = os.getenv("CLICKHOUSE_HOST", "localhost")
CH_PORT = int(os.getenv("CLICKHOUSE_PORT", "8123"))
CH_DB = os.getenv("CLICKHOUSE_DB", "pts")
CH_USER = os.getenv("CLICKHOUSE_USER", "pts")
CH_PASS = os.getenv("CLICKHOUSE_PASSWORD", "pts_secret")

# ── Redis ──
_redis: aioredis.Redis | None = None

async def get_redis() -> aioredis.Redis:
    global _redis
    if _redis is None:
        _redis = aioredis.from_url(REDIS_URL, decode_responses=False)
    return _redis

# ── ClickHouse ──
_ch = None

def get_clickhouse():
    global _ch
    if _ch is None:
        _ch = clickhouse_connect.get_client(
            host=CH_HOST, port=CH_PORT,
            database=CH_DB, username=CH_USER, password=CH_PASS,
        )
    return _ch

# ── NATS ──
_nc = None

async def get_nats():
    global _nc
    if _nc is None or _nc.is_closed:
        _nc = await nats_client.connect(NATS_URL)
    return _nc
