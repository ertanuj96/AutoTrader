"""Shared fixtures for dashboard-api tests.

Redis and NATS are mocked so tests run without infrastructure.
"""
from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock
import pytest
from fastapi.testclient import TestClient

import app.deps as deps
from app.main import app


@pytest.fixture(autouse=True)
def mock_redis(monkeypatch):
    """Replace the Redis connection with an in-memory mock."""
    redis_mock = AsyncMock()
    # Default: empty hashes and no keys
    redis_mock.hgetall.return_value = {}
    redis_mock.get.return_value = None
    redis_mock.hget.return_value = None

    async def _get_redis():
        return redis_mock

    monkeypatch.setattr(deps, "_redis", None)
    monkeypatch.setattr(deps, "get_redis", _get_redis)
    return redis_mock


@pytest.fixture(autouse=True)
def mock_nats(monkeypatch):
    """Replace the NATS connection with a mock."""
    nats_mock = AsyncMock()
    nats_mock.is_closed = False

    async def _get_nats():
        return nats_mock

    monkeypatch.setattr(deps, "_nc", None)
    monkeypatch.setattr(deps, "get_nats", _get_nats)
    return nats_mock


@pytest.fixture
def client():
    """Synchronous TestClient — avoids lifespan startup that hits Redis."""
    with TestClient(app, raise_server_exceptions=True) as c:
        yield c
