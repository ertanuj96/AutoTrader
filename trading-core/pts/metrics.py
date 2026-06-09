"""PTS Prometheus Metrics — Trading, execution, and infrastructure metrics."""

from __future__ import annotations

from prometheus_client import (
    Counter, Gauge, Histogram, Info,
    start_http_server,
)

from pts.config import settings


# ─── Trading Metrics ───

pnl_net = Gauge(
    "pts_pnl_net_total",
    "Net PnL (realized + unrealized) in INR",
)

pnl_realized = Gauge(
    "pts_pnl_realized_total",
    "Realized PnL in INR",
)

win_rate = Gauge(
    "pts_win_rate_ratio",
    "Win rate (0.0 to 1.0)",
)

drawdown_pct = Gauge(
    "pts_drawdown_percent",
    "Current drawdown percentage",
)

open_positions = Gauge(
    "pts_open_positions_count",
    "Number of open positions",
)

exposure = Gauge(
    "pts_exposure_total",
    "Total exposure in INR",
    ["strategy"],
)


# ─── Order Metrics ───

orders_total = Counter(
    "pts_orders_total",
    "Total orders by status",
    ["status"],
)

signals_total = Counter(
    "pts_signals_total",
    "Total signals generated",
    ["strategy", "side"],
)


# ─── Latency Metrics ───

signal_to_fill = Histogram(
    "pts_signal_to_fill_seconds",
    "End-to-end latency from signal generation to fill",
    buckets=[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0],
)

signal_to_order = Histogram(
    "pts_signal_to_order_seconds",
    "Latency from signal to order submission",
    buckets=[0.001, 0.005, 0.01, 0.025, 0.05, 0.1],
)

order_to_ack = Histogram(
    "pts_order_to_ack_seconds",
    "Latency from order submission to broker acknowledgement",
    buckets=[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5],
)

ack_to_fill = Histogram(
    "pts_ack_to_fill_seconds",
    "Latency from broker ack to fill",
    buckets=[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0],
)


# ─── Tick Processing ───

ticks_processed = Counter(
    "pts_ticks_processed_total",
    "Total ticks processed",
    ["instrument"],
)

tick_processing_time = Histogram(
    "pts_tick_processing_seconds",
    "Time to process a single tick",
    buckets=[0.0001, 0.0005, 0.001, 0.005, 0.01],
)


# ─── System Info ───

system_info = Info(
    "pts_system",
    "PTS system information",
)


def start_metrics_server():
    """Start the Prometheus metrics HTTP server."""
    port = settings.metrics_port
    start_http_server(port)
    system_info.info({
        "version": "0.1.0",
        "mode": "paper" if settings.paper_trade else "live",
    })
    import logging
    logging.getLogger("pts.metrics").info(f"✅ Metrics server on :{port}")
