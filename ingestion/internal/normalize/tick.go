// Package normalize converts broker-specific tick formats into canonical TickEvent.
package normalize

// TickEvent is the canonical normalized tick (matches Rust common::events::TickEvent).
type TickEvent struct {
	Symbol   string  `json:"symbol"`
	Exchange string  `json:"exchange"`
	LTP      float64 `json:"ltp"`
	Bid      float64 `json:"bid,omitempty"`
	Ask      float64 `json:"ask,omitempty"`
	BidQty   uint64  `json:"bid_qty,omitempty"`
	AskQty   uint64  `json:"ask_qty,omitempty"`
	Volume   uint64  `json:"volume,omitempty"`
	OI       uint64  `json:"oi,omitempty"`
}

