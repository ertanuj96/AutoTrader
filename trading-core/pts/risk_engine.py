"""PTS Risk Engine — Per PTS-002 event contracts."""

from __future__ import annotations

import asyncio
import logging

import nats

from pts.config import settings
from pts.events import (
    SignalGeneratedEvent, RiskApprovedEvent, RiskRejectedEvent,
    KillSwitchEvent, Subjects,
)
from pts.state_manager import StateManager
from pts.metrics import orders_total

logger = logging.getLogger("pts.risk")


class RiskEngine:
    """Validates signals against risk limits per PTS-002 contracts."""

    def __init__(self, nc: nats.NATS, state: StateManager):
        self._nc = nc
        self._state = state
        self._running = False

    async def start(self):
        self._running = True
        await self._nc.subscribe(Subjects.SIGNAL_GENERATED, cb=self._on_signal)
        await self._nc.subscribe(Subjects.KILL_SWITCH, cb=self._on_kill_switch)
        logger.info("🛡️  Risk engine started")
        while self._running:
            await asyncio.sleep(1)

    async def stop(self):
        self._running = False

    async def _on_signal(self, msg):
        try:
            signal = SignalGeneratedEvent.from_bytes(msg.data)
        except Exception as e:
            logger.error(f"🛡️  Signal parse error: {e}")
            return

        passed, failed = [], []

        # Kill switch
        if await self._state.is_kill_switch_active():
            failed.append("kill_switch_active")
        else:
            passed.append("kill_switch_ok")

        # Daily loss
        pnl = await self._state.get_total_pnl()
        net = pnl.get("net", 0)
        if net < -settings.max_daily_loss:
            failed.append(f"daily_loss:{net:.0f}")
        else:
            passed.append("daily_loss_ok")

        # Position size
        if signal.quantity > settings.max_position_size:
            failed.append(f"position_size:{signal.quantity}")
        else:
            passed.append("position_size_ok")

        # Exposure
        exposure = await self._state.get_exposure()
        strat_exp = exposure.get(signal.strategy_id, 0)
        price = await self._state.get_price_by_instrument(signal.instrument) or 0
        if strat_exp + price * signal.quantity > settings.max_exposure_per_strategy:
            failed.append("exposure_exceeded")
        else:
            passed.append("exposure_ok")

        if failed:
            rej = RiskRejectedEvent(
                signal_id=signal.signal_id,
                reason="; ".join(failed),
                checks_failed=failed,
            )
            await self._nc.publish(Subjects.RISK_REJECTED, rej.to_bytes())
            orders_total.labels(status="risk_rejected").inc()
            logger.warning(f"🛡️  REJECTED {signal.symbol}: {rej.reason}")
        else:
            app = RiskApprovedEvent(
                signal_id=signal.signal_id,
                signal=signal.model_dump(),
                approved=True,
                checks_passed=passed,
            )
            await self._nc.publish(Subjects.RISK_APPROVED, app.to_bytes())
            orders_total.labels(status="risk_approved").inc()
            logger.info(f"🛡️  APPROVED {signal.action} {signal.symbol}")

    async def _on_kill_switch(self, msg):
        logger.critical("🛡️  KILL SWITCH ACTIVATED")
        await self._state.activate_kill_switch()
