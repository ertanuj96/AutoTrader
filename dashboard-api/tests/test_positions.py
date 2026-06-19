"""Unit tests for /api/positions, /api/pnl, /api/exposure, /api/prices, /api/portfolio."""
from __future__ import annotations

import orjson


# ── /api/positions ──

def test_positions_empty_redis_returns_empty_list(client, mock_redis):
    mock_redis.hgetall.return_value = {}
    r = client.get("/api/positions")
    assert r.status_code == 200
    assert r.json() == {"positions": []}


def test_positions_parses_redis_hash(client, mock_redis):
    pos_data = {"qty": 50, "entry_price": 18000.0, "side": "BUY"}
    mock_redis.hgetall.return_value = {
        b"ema_crossover:NIFTY26JUN25000CE": orjson.dumps(pos_data)
    }
    r = client.get("/api/positions")
    assert r.status_code == 200
    positions = r.json()["positions"]
    assert len(positions) == 1
    p = positions[0]
    assert p["strategy_id"] == "ema_crossover"
    assert p["symbol"] == "NIFTY26JUN25000CE"
    assert p["qty"] == 50


def test_positions_multiple_entries(client, mock_redis):
    mock_redis.hgetall.return_value = {
        b"strat_a:NIFTY": orjson.dumps({"qty": 10}),
        b"strat_b:BANKNIFTY": orjson.dumps({"qty": 5}),
    }
    r = client.get("/api/positions")
    assert len(r.json()["positions"]) == 2


# ── /api/pnl ──

def test_pnl_empty_redis_returns_zeros(client, mock_redis):
    mock_redis.hgetall.return_value = {}
    r = client.get("/api/pnl")
    assert r.status_code == 200
    data = r.json()
    assert data["realized"] == 0
    assert data["unrealized"] == 0
    assert data["net"] == 0


def test_pnl_aggregates_by_strategy(client, mock_redis):
    mock_redis.hgetall.return_value = {
        b"strat_a": orjson.dumps({"realized": 5000.0, "unrealized": -1000.0}),
        b"strat_b": orjson.dumps({"realized": 3000.0, "unrealized": 500.0}),
    }
    r = client.get("/api/pnl")
    data = r.json()
    assert data["realized"] == 8000.0
    assert data["unrealized"] == -500.0
    assert data["net"] == 7500.0
    assert "strat_a" in data["by_strategy"]
    assert "strat_b" in data["by_strategy"]


# ── /api/exposure ──

def test_exposure_empty_redis(client, mock_redis):
    mock_redis.hgetall.return_value = {}
    r = client.get("/api/exposure")
    assert r.status_code == 200
    assert r.json() == {"exposure": {}, "total": 0.0}


def test_exposure_sums_correctly(client, mock_redis):
    mock_redis.hgetall.return_value = {
        b"ema_crossover": b"150000.0",
        b"momentum":      b"75000.0",
    }
    r = client.get("/api/exposure")
    data = r.json()
    assert data["total"] == 225000.0
    assert data["exposure"]["ema_crossover"] == 150000.0


# ── /api/prices ──

def test_prices_empty_redis(client, mock_redis):
    mock_redis.hgetall.return_value = {}
    r = client.get("/api/prices")
    assert r.status_code == 200
    assert r.json() == {"prices": {}}


def test_prices_returns_float_values(client, mock_redis):
    mock_redis.hgetall.return_value = {
        b"NSE:NIFTY": b"18432.50",
        b"NSE:BANKNIFTY": b"43210.75",
    }
    r = client.get("/api/prices")
    prices = r.json()["prices"]
    assert prices["NSE:NIFTY"] == 18432.50
    assert prices["NSE:BANKNIFTY"] == 43210.75


# ── /api/portfolio ──

def test_portfolio_empty_redis_returns_defaults(client, mock_redis):
    mock_redis.get.return_value = None
    r = client.get("/api/portfolio")
    assert r.status_code == 200
    portfolio = r.json()["portfolio"]
    assert portfolio["net_pnl"] == 0
    assert portfolio["gross_exposure"] == 0


def test_portfolio_parses_redis_value(client, mock_redis):
    state = {"net_pnl": 25000.0, "gross_exposure": 500000.0, "margin_used": 100000.0,
             "realized_pnl": 15000.0, "unrealized_pnl": 10000.0}
    mock_redis.get.return_value = orjson.dumps(state)
    r = client.get("/api/portfolio")
    portfolio = r.json()["portfolio"]
    assert portfolio["net_pnl"] == 25000.0
    assert portfolio["gross_exposure"] == 500000.0
