// Package main — AutoTrader Data Ingestion Service.
//
// Architecture:
//
//	Broker WS → Normalizer → NATS (pts.market.tick)
//	                       → ClickHouse (batch insert)
//	                       → Candle Materializer → NATS (pts.market.candle)
package main

import (
	"context"
	"os"
	"os/signal"
	"syscall"
	"time"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"

	"github.com/ertanuj96/AutoTrader/ingestion/internal/broker"
	"github.com/ertanuj96/AutoTrader/ingestion/internal/candle"
	"github.com/ertanuj96/AutoTrader/ingestion/internal/config"
	"github.com/ertanuj96/AutoTrader/ingestion/internal/normalize"
	"github.com/ertanuj96/AutoTrader/ingestion/internal/publisher"
)

func main() {
	log := buildLogger()
	defer log.Sync() //nolint:errcheck

	cfg := config.Load()
	log.Info("AutoTrader Ingestion Service starting",
		zap.String("nats", cfg.NatsURL),
		zap.String("ch_host", cfg.ClickHouseHost),
		zap.Strings("instruments", cfg.Instruments),
		zap.Strings("timeframes", cfg.Timeframes),
	)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// ── NATS publisher ──────────────────────────────────────────────────────────
	natsPub, err := publisher.NewNatsPublisher(cfg.NatsURL, log)
	if err != nil {
		log.Fatal("NATS init failed", zap.Error(err))
	}
	defer natsPub.Close()

	// ── ClickHouse writer (optional — service runs without it) ─────────────────
	var chWriter *publisher.CHWriter
	chWriter, err = publisher.NewCHWriter(
		cfg.ClickHouseHost, cfg.ClickHousePort,
		cfg.ClickHouseDB, cfg.ClickHouseUser, cfg.ClickHousePass,
		cfg.CHBatchSize, cfg.CHFlushInterval, log,
	)
	if err != nil {
		log.Warn("ClickHouse unavailable — ticks will only go to NATS", zap.Error(err))
	} else {
		chWriter.StartFlushLoop(ctx)
		defer chWriter.Stop()
	}

	// ── Candle materializer ─────────────────────────────────────────────────────
	mat := candle.NewMaterializer(cfg.Timeframes, 500)

	// Consumer: publish completed candles to NATS and ClickHouse
	go func() {
		for bar := range mat.Completed() {
			if err := natsPub.PublishCandle(bar); err != nil {
				log.Error("publish candle", zap.Error(err))
			}
			if chWriter != nil {
				chWriter.AddCandle(bar)
			}
		}
	}()

	// ── Broker adapters ─────────────────────────────────────────────────────────
	adapters := buildAdapters(cfg, log)
	if len(adapters) == 0 {
		log.Fatal("No broker adapters configured — set DHAN_ACCESS_TOKEN or FYERS_ACCESS_TOKEN")
	}
	for _, a := range adapters {
		go runAdapter(ctx, a, cfg.Instruments, cfg.ReconnectDelay, mat, natsPub, chWriter, log)
	}

	// ── Graceful shutdown ───────────────────────────────────────────────────────
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)
	<-sig
	log.Info("Shutdown signal received")
	cancel()
	time.Sleep(500 * time.Millisecond) // allow final flush
	log.Info("AutoTrader Ingestion Service stopped")
}

// runAdapter manages the lifecycle of a single broker adapter with reconnection.
func runAdapter(
	ctx context.Context,
	a broker.Adapter,
	instruments []string,
	reconnectDelay time.Duration,
	mat *candle.Materializer,
	natsPub *publisher.NatsPublisher,
	chWriter *publisher.CHWriter,
	log *zap.Logger,
) {
	for {
		select {
		case <-ctx.Done():
			_ = a.Close()
			return
		default:
		}

		log.Info("Connecting broker", zap.String("broker", a.Name()))
		if err := a.Connect(); err != nil {
			log.Error("broker connect failed", zap.String("broker", a.Name()), zap.Error(err))
			select {
			case <-ctx.Done():
				return
			case <-time.After(reconnectDelay):
				continue
			}
		}

		if err := a.Subscribe(instruments); err != nil {
			log.Error("broker subscribe failed", zap.String("broker", a.Name()), zap.Error(err))
			_ = a.Close()
			select {
			case <-ctx.Done():
				return
			case <-time.After(reconnectDelay):
				continue
			}
		}

		log.Info("Broker connected and subscribed",
			zap.String("broker", a.Name()),
			zap.Int("instruments", len(instruments)),
		)

		disconnected := false
		for !disconnected {
			select {
			case <-ctx.Done():
				_ = a.Close()
				return
			case raw := <-a.Ticks():
				tick := normalize.FromRawTick(raw)
				mat.Update(tick, raw.Timestamp)
				if err := natsPub.PublishTick(tick); err != nil {
					log.Error("publish tick", zap.Error(err))
				}
				if chWriter != nil {
					chWriter.AddTick(tick)
				}
			case err := <-a.Errors():
				log.Error("broker error", zap.String("broker", a.Name()), zap.Error(err))
				disconnected = true
			}
		}

		_ = a.Close()
		log.Info("Reconnecting broker", zap.String("broker", a.Name()), zap.Duration("delay", reconnectDelay))
		select {
		case <-ctx.Done():
			return
		case <-time.After(reconnectDelay):
		}
	}
}

func buildAdapters(cfg *config.Config, log *zap.Logger) []broker.Adapter {
	var adapters []broker.Adapter
	if cfg.DhanAccessToken != "" {
		adapters = append(adapters, broker.NewDhanAdapter(cfg.DhanAccessToken, cfg.DhanClientID, log))
		log.Info("Dhan adapter configured")
	}
	if cfg.FyersAccessToken != "" {
		adapters = append(adapters, broker.NewFyersAdapter(cfg.FyersAccessToken, cfg.FyersAppID, log))
		log.Info("Fyers adapter configured")
	}
	return adapters
}

func buildLogger() *zap.Logger {
	enc := zap.NewProductionEncoderConfig()
	enc.TimeKey = "ts"
	enc.EncodeTime = zapcore.ISO8601TimeEncoder
	cfg := zap.NewProductionConfig()
	cfg.EncoderConfig = enc
	l, _ := cfg.Build()
	return l
}
