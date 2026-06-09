// Package main — AutoTrader Data Ingestion Service.
// High-performance tick ingestion from broker WebSockets,
// candle materialization, option chain polling, and EOD data fetching.
//
// Architecture:
//   Broker WS → Normalizer → NATS (pts.market.tick)
//                          → ClickHouse (batch insert)
//                          → Candle Materializer → NATS (pts.market.candle)
package main

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"
)

func main() {
	fmt.Println("🚀 AutoTrader Ingestion Service v0.1.0")
	fmt.Println("TODO: Wire up broker WebSocket consumers, NATS publisher, ClickHouse writer")

	// Wait for shutdown signal
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	<-sigCh
	fmt.Println("👋 Ingestion service stopped")
}

