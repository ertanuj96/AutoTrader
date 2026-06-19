"""PTS Persistence — ClickHouse schema and storage per PTS-003 spec."""

from __future__ import annotations

import logging
from typing import Optional

import clickhouse_connect

from pts.config import settings

logger = logging.getLogger("pts.persistence")

# Database names per PTS-003 + PTS-004 (v2)
DB_MARKET = "pts_market"
DB_TRADING = "pts_trading"
DB_ANALYTICS = "pts_analytics"
DB_SYSTEM = "pts_system"
DB_QUANT = "pts_quant"


class Persistence:
    """ClickHouse persistence layer per PTS-003 Data Model spec."""

    def __init__(self):
        self._client = None

    async def connect(self):
        """Connect to ClickHouse and ensure schema exists."""
        self._client = clickhouse_connect.get_client(
            host=settings.clickhouse_host,
            port=settings.clickhouse_port,
            username=settings.clickhouse_user,
            password=settings.clickhouse_password,
        )
        self._create_schema()
        logger.info("✅ ClickHouse connected")

    def _create_schema(self):
        """Create databases and tables per PTS-003 (idempotent)."""

        # Create databases
        for db in [DB_MARKET, DB_TRADING, DB_ANALYTICS, DB_SYSTEM, DB_QUANT]:
            self._client.command(f"CREATE DATABASE IF NOT EXISTS {db}")

        # ── pts_market ──

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_MARKET}.tick_data (
                ts DateTime64(6),
                symbol String,
                exchange String,
                ltp Float64,
                bid Float64,
                ask Float64,
                bid_qty UInt64,
                ask_qty UInt64,
                volume UInt64,
                oi UInt64
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(ts)
            ORDER BY (symbol, ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_MARKET}.option_chain_snapshot (
                ts DateTime64(6),
                underlying String,
                expiry Date,
                strike Float64,
                option_type String,
                ltp Float64,
                bid Float64,
                ask Float64,
                oi UInt64,
                volume UInt64
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(ts)
            ORDER BY (underlying, expiry, strike, ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_MARKET}.greeks_snapshot (
                ts DateTime64(6),
                symbol String,
                iv Float64,
                delta Float64,
                gamma Float64,
                theta Float64,
                vega Float64
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(ts)
            ORDER BY (symbol, ts)
        """)

        # ── pts_trading ──

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_TRADING}.signal_history (
                signal_id UUID,
                ts DateTime64(6),
                strategy_id String,
                symbol String,
                action String,
                quantity UInt64,
                confidence Float64,
                reason String
            ) ENGINE = MergeTree()
            ORDER BY (strategy_id, ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_TRADING}.order_history (
                order_id UUID,
                broker_order_id String,
                ts DateTime64(6),
                strategy_id String,
                symbol String,
                side String,
                quantity UInt64,
                price Float64,
                status String
            ) ENGINE = MergeTree()
            ORDER BY (strategy_id, ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_TRADING}.trade_fills (
                fill_id UUID,
                order_id UUID,
                ts DateTime64(6),
                symbol String,
                quantity UInt64,
                fill_price Float64
            ) ENGINE = MergeTree()
            ORDER BY (ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_TRADING}.position_history (
                position_id UUID,
                strategy_id String,
                symbol String,
                entry_time DateTime64(6),
                exit_time DateTime64(6),
                quantity Int64,
                entry_price Float64,
                exit_price Float64,
                realized_pnl Float64
            ) ENGINE = MergeTree()
            ORDER BY (strategy_id, entry_time)
        """)

        # ── pts_analytics ──

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_ANALYTICS}.portfolio_snapshot (
                ts DateTime64(6),
                net_pnl Float64,
                gross_exposure Float64,
                margin_used Float64,
                delta Float64,
                gamma Float64,
                theta Float64,
                vega Float64
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(ts)
            ORDER BY (ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_ANALYTICS}.execution_metrics (
                order_id UUID,
                ts DateTime64(6),
                strategy_latency_ms Float64,
                risk_latency_ms Float64,
                broker_latency_ms Float64,
                fill_latency_ms Float64,
                total_latency_ms Float64
            ) ENGINE = MergeTree()
            ORDER BY (ts)
        """)

        # ── v2: pts_market extensions ──

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_MARKET}.candle_data (
                ts DateTime64(6),
                symbol String,
                exchange String,
                timeframe String,
                open Float64,
                high Float64,
                low Float64,
                close Float64,
                volume UInt64,
                oi UInt64
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(ts)
            ORDER BY (symbol, timeframe, ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_MARKET}.pcr_history (
                ts DateTime64(6),
                underlying String,
                pcr_oi Float64,
                pcr_volume Float64,
                total_call_oi UInt64,
                total_put_oi UInt64
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(ts)
            ORDER BY (underlying, ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_MARKET}.vix_history (
                ts DateTime64(6),
                vix_value Float64,
                vix_change_pct Float64
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(ts)
            ORDER BY (ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_MARKET}.commodity_data (
                ts DateTime64(6),
                symbol String,
                ltp Float64,
                open Float64,
                high Float64,
                low Float64,
                close Float64,
                volume UInt64
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(ts)
            ORDER BY (symbol, ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_MARKET}.instrument_master (
                symbol String,
                exchange String,
                segment String,
                instrument_type String,
                lot_size Float64,
                expiry Date,
                strike Float64,
                option_type String,
                underlying String,
                updated_at DateTime64(6)
            ) ENGINE = ReplacingMergeTree(updated_at)
            ORDER BY (symbol, exchange)
        """)

        # ── v2: pts_quant ──

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_QUANT}.regime_history (
                ts DateTime64(6),
                symbol String,
                timeframe String,
                p_trending Float64,
                p_ranging Float64,
                p_high_vol Float64,
                hurst_exponent Float64,
                garch_variance Float64,
                regime String
            ) ENGINE = MergeTree()
            PARTITION BY toYYYYMM(ts)
            ORDER BY (symbol, timeframe, ts)
        """)

        self._client.command(f"""
            CREATE TABLE IF NOT EXISTS {DB_QUANT}.signal_scores (
                signal_id UUID,
                ts DateTime64(6),
                symbol String,
                confidence Float64,
                regime_contribution Float64,
                trend_contribution Float64,
                reversal_contribution Float64,
                movement_contribution Float64,
                kelly_lots UInt32,
                payoff_ratio Float64
            ) ENGINE = MergeTree()
            ORDER BY (ts)
        """)

        logger.info("✅ ClickHouse schema ready (5 databases, 15 tables)")

    # ── Helper: convert ms timestamp to DateTime64 string ──

    @staticmethod
    def _ms_to_dt(ms: int) -> str:
        from datetime import datetime, timezone
        return datetime.fromtimestamp(ms / 1000, tz=timezone.utc).strftime("%Y-%m-%d %H:%M:%S.%f")

    # ── Insert helpers ──

    def insert_tick(self, tick: dict):
        """Insert a single tick into pts_market.tick_data."""
        ts = self._ms_to_dt(tick.get("timestamp_ms", 0))
        self._client.insert(f"{DB_MARKET}.tick_data", [[
            ts, tick.get("symbol", ""), tick.get("exchange", "NSE"),
            tick.get("ltp", 0), tick.get("bid", 0), tick.get("ask", 0),
            tick.get("bid_qty", 0), tick.get("ask_qty", 0),
            tick.get("volume", 0), tick.get("oi", 0),
        ]], column_names=["ts", "symbol", "exchange", "ltp", "bid", "ask", "bid_qty", "ask_qty", "volume", "oi"])

    def insert_ticks_batch(self, ticks: list[dict]):
        """Batch insert ticks for high throughput."""
        if not ticks:
            return
        rows = []
        for t in ticks:
            rows.append([
                self._ms_to_dt(t.get("timestamp_ms", 0)),
                t.get("symbol", ""), t.get("exchange", "NSE"),
                t.get("ltp", 0), t.get("bid", 0), t.get("ask", 0),
                t.get("bid_qty", 0), t.get("ask_qty", 0),
                t.get("volume", 0), t.get("oi", 0),
            ])
        self._client.insert(
            f"{DB_MARKET}.tick_data", rows,
            column_names=["ts", "symbol", "exchange", "ltp", "bid", "ask", "bid_qty", "ask_qty", "volume", "oi"],
        )

    def insert_signal(self, signal: dict):
        ts = self._ms_to_dt(signal.get("timestamp_ms", 0))
        self._client.insert(f"{DB_TRADING}.signal_history", [[
            signal.get("signal_id", ""), ts, signal.get("strategy_id", ""),
            signal.get("symbol", ""), signal.get("action", ""),
            signal.get("quantity", 0), signal.get("confidence", 0),
            signal.get("reason", ""),
        ]], column_names=["signal_id", "ts", "strategy_id", "symbol", "action", "quantity", "confidence", "reason"])

    def insert_order(self, order: dict):
        ts = self._ms_to_dt(order.get("timestamp_ms", 0))
        self._client.insert(f"{DB_TRADING}.order_history", [[
            order.get("order_id", ""), order.get("broker_order_id", ""),
            ts, order.get("strategy_id", ""), order.get("symbol", ""),
            order.get("side", ""), order.get("quantity", 0),
            order.get("price", 0), order.get("status", ""),
        ]], column_names=["order_id", "broker_order_id", "ts", "strategy_id", "symbol", "side", "quantity", "price", "status"])

    def insert_fill(self, fill: dict):
        ts = self._ms_to_dt(fill.get("timestamp_ms", 0))
        self._client.insert(f"{DB_TRADING}.trade_fills", [[
            fill.get("fill_id", ""), fill.get("order_id", ""),
            ts, fill.get("symbol", ""), fill.get("quantity", 0),
            fill.get("fill_price", 0),
        ]], column_names=["fill_id", "order_id", "ts", "symbol", "quantity", "fill_price"])

    def insert_trade(self, trade: dict):
        self._client.insert(f"{DB_TRADING}.position_history", [[
            trade.get("position_id", ""), trade.get("strategy_id", ""),
            trade.get("symbol", ""), self._ms_to_dt(trade.get("entry_time", 0)),
            self._ms_to_dt(trade.get("exit_time", 0)),
            trade.get("quantity", 0), trade.get("entry_price", 0),
            trade.get("exit_price", 0), trade.get("realized_pnl", 0),
        ]], column_names=["position_id", "strategy_id", "symbol", "entry_time", "exit_time", "quantity", "entry_price", "exit_price", "realized_pnl"])

    def insert_latency(self, latency: dict):
        ts = self._ms_to_dt(latency.get("timestamp_ms", 0))
        self._client.insert(f"{DB_ANALYTICS}.execution_metrics", [[
            latency.get("order_id", ""), ts,
            latency.get("strategy_latency_ms", 0),
            latency.get("risk_latency_ms", 0),
            latency.get("broker_latency_ms", 0),
            latency.get("fill_latency_ms", 0),
            latency.get("total_latency_ms", 0),
        ]], column_names=["order_id", "ts", "strategy_latency_ms", "risk_latency_ms", "broker_latency_ms", "fill_latency_ms", "total_latency_ms"])

    # ── Query helpers ──

    def get_recent_orders(self, limit: int = 50) -> list[dict]:
        result = self._client.query(
            f"SELECT * FROM {DB_TRADING}.order_history ORDER BY ts DESC LIMIT {limit}"
        )
        return [dict(zip(result.column_names, row)) for row in result.result_rows]

    def get_recent_trades(self, limit: int = 50) -> list[dict]:
        result = self._client.query(
            f"SELECT * FROM {DB_TRADING}.position_history ORDER BY entry_time DESC LIMIT {limit}"
        )
        return [dict(zip(result.column_names, row)) for row in result.result_rows]

    def get_recent_signals(self, limit: int = 50) -> list[dict]:
        result = self._client.query(
            f"SELECT * FROM {DB_TRADING}.signal_history ORDER BY ts DESC LIMIT {limit}"
        )
        return [dict(zip(result.column_names, row)) for row in result.result_rows]

    def close(self):
        if self._client:
            self._client.close()
