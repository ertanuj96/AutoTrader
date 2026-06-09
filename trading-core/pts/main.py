"""PTS Main — Asyncio orchestrator that starts all trading services."""

from __future__ import annotations

import asyncio
import logging
import signal
import sys

import nats
from dotenv import load_dotenv

# Load .env before importing config
load_dotenv()

from pts.config import settings
from pts.market_data import MarketDataService
from pts.strategy_engine import StrategyEngine, EMACrossoverStrategy
from pts.risk_engine import RiskEngine
from pts.execution_engine import ExecutionEngine
from pts.broker_adapter import create_broker_adapter
from pts.state_manager import StateManager
from pts.persistence import Persistence
from pts.metrics import start_metrics_server

# ── Logging ──
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s │ %(name)-20s │ %(levelname)-7s │ %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("pts.main")


async def main():
    logger.info("=" * 60)
    logger.info("  PTS — Personal Trading System v0.1.0")
    logger.info(f"  Mode: {'🔶 PAPER' if settings.paper_trade else '🔴 LIVE'}")
    logger.info(f"  Instruments: {settings.instruments}")
    logger.info("=" * 60)

    # ── Start metrics server ──
    start_metrics_server()

    # ── Connect to infrastructure ──
    logger.info("Connecting to infrastructure...")

    nc = await nats.connect(settings.nats_url)
    logger.info("✅ NATS connected")

    state = StateManager()
    await state.connect()

    persistence = Persistence()
    await persistence.connect()

    # ── Create components ──
    broker = create_broker_adapter()

    market_data = MarketDataService(nc, state, persistence)
    strategy_engine = StrategyEngine(nc, state)
    risk_engine = RiskEngine(nc, state)
    execution_engine = ExecutionEngine(nc, broker, state, persistence)

    # ── Register strategies ──
    ema_strategy = EMACrossoverStrategy(
        strategy_id="ema_crossover_v1",
        instruments=settings.instruments,
        fast_period=settings.ema_fast_period,
        slow_period=settings.ema_slow_period,
        quantity=1,
    )
    strategy_engine.register(ema_strategy)

    # ── Graceful shutdown ──
    shutdown_event = asyncio.Event()

    def _shutdown(sig, frame):
        logger.info(f"Received signal {sig}, shutting down...")
        shutdown_event.set()

    signal.signal(signal.SIGINT, _shutdown)
    signal.signal(signal.SIGTERM, _shutdown)

    # ── Start all services ──
    tasks = [
        asyncio.create_task(market_data.start(), name="market_data"),
        asyncio.create_task(strategy_engine.start(), name="strategy_engine"),
        asyncio.create_task(risk_engine.start(), name="risk_engine"),
        asyncio.create_task(execution_engine.start(), name="execution_engine"),
    ]

    logger.info("🚀 All services started — trading is ACTIVE")

    # Wait for shutdown
    await shutdown_event.wait()

    # ── Cleanup ──
    logger.info("Shutting down services...")
    await market_data.stop()
    await strategy_engine.stop()
    await risk_engine.stop()
    await execution_engine.stop()

    for task in tasks:
        task.cancel()

    await nc.close()
    await state.close()
    persistence.close()

    logger.info("👋 PTS shutdown complete")


def run():
    """Entry point."""
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    run()
