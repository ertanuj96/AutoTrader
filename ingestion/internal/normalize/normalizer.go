package normalize

import (
	"strings"
	"time"

	"github.com/ertanuj96/AutoTrader/ingestion/internal/broker"
)

// CandleBar is a completed OHLCV candle published to downstream systems.
type CandleBar struct {
	Symbol    string    `json:"symbol"`
	Exchange  string    `json:"exchange"`
	Timeframe string    `json:"timeframe"`
	Open      float64   `json:"open"`
	High      float64   `json:"high"`
	Low       float64   `json:"low"`
	Close     float64   `json:"close"`
	Volume    uint64    `json:"volume"`
	OI        uint64    `json:"oi"`
	BarStart  time.Time `json:"bar_start"`
	BarEnd    time.Time `json:"bar_end"`
}

// FromRawTick converts a broker RawTick to the canonical TickEvent format.
func FromRawTick(raw broker.RawTick) TickEvent {
	return TickEvent{
		Symbol:   canonicalSymbol(raw.Symbol),
		Exchange: raw.Exchange,
		LTP:      raw.LTP,
		Bid:      raw.Bid,
		Ask:      raw.Ask,
		BidQty:   raw.BidQty,
		AskQty:   raw.AskQty,
		Volume:   raw.Volume,
		OI:       raw.OI,
	}
}

// canonicalSymbol normalizes symbol strings to upper-case trimmed form.
func canonicalSymbol(symbol string) string {
	s := strings.TrimSpace(symbol)
	if s == "" {
		return "UNKNOWN"
	}
	return strings.ToUpper(s)
}
