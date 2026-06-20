package broker

import "time"

// RawTick is the normalized intermediate representation before canonical conversion.
type RawTick struct {
	Symbol    string
	Exchange  string
	LTP       float64
	Bid       float64
	Ask       float64
	BidQty    uint64
	AskQty    uint64
	Volume    uint64
	OI        uint64
	High      float64
	Low       float64
	Open      float64
	PrevClose float64
	Timestamp time.Time
}

// Adapter is the interface each broker WebSocket client must implement.
type Adapter interface {
	// Connect establishes the WebSocket connection and authenticates.
	Connect() error
	// Subscribe sends subscription requests for the given instrument tokens.
	Subscribe(instruments []string) error
	// Ticks returns a read-only channel that delivers RawTick events.
	Ticks() <-chan RawTick
	// Errors returns a channel for connection-level errors.
	Errors() <-chan error
	// Close gracefully disconnects.
	Close() error
	// Name returns the broker identifier string.
	Name() string
}
