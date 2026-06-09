"""PTS Configuration — Pydantic Settings with .env support."""

from __future__ import annotations

from pydantic_settings import BaseSettings
from pydantic import Field


class Settings(BaseSettings):
    """Central configuration loaded from environment variables / .env file."""

    # ── Broker ──
    indstocks_api_token: str = Field(default="", description="INDstocks API bearer token")
    indstocks_base_url: str = Field(default="https://api.indstocks.com")
    indstocks_ws_url: str = Field(default="wss://ws-prices.indstocks.com/api/v1/ws/prices")

    # ── Trading Mode ──
    paper_trade: bool = Field(default=True, description="Paper trade mode (no real orders)")

    # ── Infrastructure ──
    nats_url: str = Field(default="nats://localhost:4222")
    redis_url: str = Field(default="redis://localhost:6379")
    clickhouse_host: str = Field(default="localhost")
    clickhouse_port: int = Field(default=8123)
    clickhouse_db: str = Field(default="pts")
    clickhouse_user: str = Field(default="pts")
    clickhouse_password: str = Field(default="pts_secret")

    # ── Risk Limits ──
    max_daily_loss: float = Field(default=10_000.0, description="Max daily loss in INR")
    max_position_size: int = Field(default=50, description="Max lots per position")
    max_exposure_per_strategy: float = Field(default=100_000.0, description="Max capital per strategy")
    max_drawdown_pct: float = Field(default=5.0, description="Max drawdown percentage")

    # ── Strategy Defaults ──
    default_instruments: str = Field(default="NSE:NIFTY,NSE:BANKNIFTY")
    ema_fast_period: int = Field(default=9)
    ema_slow_period: int = Field(default=21)

    # ── Metrics ──
    metrics_port: int = Field(default=8000)

    # ── Dashboard API ──
    dashboard_api_port: int = Field(default=8001)

    @property
    def instruments(self) -> list[str]:
        return [s.strip() for s in self.default_instruments.split(",") if s.strip()]

    model_config = {
        "env_file": ".env",
        "env_file_encoding": "utf-8",
        "case_sensitive": False,
    }


# Singleton
settings = Settings()
