package normalize

import (
	"encoding/json"
	"testing"
)

// ── Unit tests ──

func TestTickEventSymbolRequired(t *testing.T) {
	tick := TickEvent{
		Symbol:   "NIFTY26JUN25000CE",
		Exchange: "NSE",
		LTP:      18500.0,
	}
	if tick.Symbol == "" {
		t.Fatal("Symbol must be non-empty")
	}
}

func TestTickEventJSONRoundTrip(t *testing.T) {
	original := TickEvent{
		Symbol:   "BANKNIFTY26JUN50000CE",
		Exchange: "NSE",
		LTP:      43200.75,
		Bid:      43200.0,
		Ask:      43201.0,
		BidQty:   100,
		AskQty:   200,
		Volume:   5000,
		OI:       150000,
	}

	data, err := json.Marshal(original)
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}

	var decoded TickEvent
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("unmarshal failed: %v", err)
	}

	if decoded.Symbol != original.Symbol {
		t.Errorf("Symbol: got %q want %q", decoded.Symbol, original.Symbol)
	}
	if decoded.LTP != original.LTP {
		t.Errorf("LTP: got %f want %f", decoded.LTP, original.LTP)
	}
	if decoded.OI != original.OI {
		t.Errorf("OI: got %d want %d", decoded.OI, original.OI)
	}
}

func TestTickEventOmitemptyFieldsAbsent(t *testing.T) {
	// Optional fields with zero values must be omitted from JSON (omitempty)
	tick := TickEvent{
		Symbol:   "NIFTY",
		Exchange: "NSE",
		LTP:      18000.0,
	}
	data, err := json.Marshal(tick)
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}
	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("unmarshal to map failed: %v", err)
	}
	for _, field := range []string{"bid", "ask", "bid_qty", "ask_qty", "volume", "oi"} {
		if _, exists := m[field]; exists {
			t.Errorf("optional zero field %q should be omitted, but was present", field)
		}
	}
}

// ── Table-driven tests ──

func TestTickEventTableDriven(t *testing.T) {
	cases := []struct {
		name     string
		tick     TickEvent
		wantJSON string // partial check
	}{
		{
			name:     "nse_option",
			tick:     TickEvent{Symbol: "NIFTY26JUN25000CE", Exchange: "NSE", LTP: 150.25},
			wantJSON: `"symbol":"NIFTY26JUN25000CE"`,
		},
		{
			name:     "mcx_crude",
			tick:     TickEvent{Symbol: "CRUDEOIL26JUNFUT", Exchange: "MCX", LTP: 6850.0},
			wantJSON: `"exchange":"MCX"`,
		},
		{
			name:     "with_oi",
			tick:     TickEvent{Symbol: "NIFTY", Exchange: "NSE", LTP: 18000.0, OI: 9999},
			wantJSON: `"oi":9999`,
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			data, err := json.Marshal(tc.tick)
			if err != nil {
				t.Fatalf("marshal failed: %v", err)
			}
			got := string(data)
			if !contains(got, tc.wantJSON) {
				t.Errorf("JSON %q does not contain %q", got, tc.wantJSON)
			}
		})
	}
}

func TestTickEventLTPNonNegative(t *testing.T) {
	ticks := []TickEvent{
		{Symbol: "NIFTY", Exchange: "NSE", LTP: 0.0},
		{Symbol: "NIFTY", Exchange: "NSE", LTP: 18000.0},
		{Symbol: "NIFTY", Exchange: "NSE", LTP: 0.01},
	}
	for _, tick := range ticks {
		if tick.LTP < 0 {
			t.Errorf("LTP must be non-negative, got %f", tick.LTP)
		}
	}
}

func TestTickEventExchangeField(t *testing.T) {
	validExchanges := []string{"NSE", "MCX", "BSE"}
	for _, ex := range validExchanges {
		tick := TickEvent{Symbol: "TEST", Exchange: ex, LTP: 100.0}
		if tick.Exchange != ex {
			t.Errorf("Exchange round-trip failed: got %q want %q", tick.Exchange, ex)
		}
	}
}

// ── Helpers ──

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr ||
		func() bool {
			for i := 0; i <= len(s)-len(substr); i++ {
				if s[i:i+len(substr)] == substr {
					return true
				}
			}
			return false
		}())
}
