"""Unit tests for /api/system routes and WebSocket manager."""
from __future__ import annotations

from app.main import ConnectionManager


# ── ConnectionManager unit tests (no WebSocket needed) ──

def test_connection_manager_starts_empty():
    mgr = ConnectionManager()
    assert mgr.active == []


def test_connection_manager_disconnect_removes_ws():
    mgr = ConnectionManager()
    fake_ws = object()
    mgr.active.append(fake_ws)
    mgr.disconnect(fake_ws)
    assert fake_ws not in mgr.active


def test_connection_manager_disconnect_nonexistent_raises():
    """Disconnecting a WS that was never connected should raise ValueError."""
    mgr = ConnectionManager()
    fake_ws = object()
    try:
        mgr.disconnect(fake_ws)
        assert False, "Expected ValueError"
    except ValueError:
        pass  # expected
