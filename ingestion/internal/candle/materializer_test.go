package candle

import (
	"testing"
	"time"

	"github.com/ertanuj96/AutoTrader/ingestion/internal/normalize"
)

// IST offset for test helpers
var ist = time.FixedZone("IST", 5*3600+30*60)

func makeTick(symbol string, ltp float64, vol uint64) normalize.TickEvent {
	return normalize.TickEvent{Symbol: symbol, Exchange: "NSE", LTP: ltp, Volume: vol}
}

func TestBarAlignIST_1m(t *testing.T) {
	// 09:15:23 IST → 09:15:00 IST
	ts := time.Date(2024, 1, 15, 9, 15, 23, 0, ist)
	aligned := barAlignIST(ts, time.Minute)
	expected := time.Date(2024, 1, 15, 9, 15, 0, 0, ist).UTC()
	if !aligned.Equal(expected) {
		t.Errorf("1m align: got %v, want %v", aligned, expected)
	}
}

func TestBarAlignIST_5m(t *testing.T) {
	// 09:17:45 IST → 09:15:00 IST
	ts := time.Date(2024, 1, 15, 9, 17, 45, 0, ist)
	aligned := barAlignIST(ts, 5*time.Minute)
	expected := time.Date(2024, 1, 15, 9, 15, 0, 0, ist).UTC()
	if !aligned.Equal(expected) {
		t.Errorf("5m align: got %v, want %v", aligned, expected)
	}
}

func TestMaterializerEmitsOnBoundaryCross(t *testing.T) {
	mat := NewMaterializer([]string{"1m"}, 10)

	base := time.Date(2024, 1, 15, 9, 15, 0, 0, ist)
	tick := makeTick("NIFTY", 18000.0, 1000)

	// Feed 3 ticks in same 1m bar
	mat.Update(tick, base.Add(10*time.Second))
	mat.Update(tick, base.Add(30*time.Second))
	mat.Update(tick, base.Add(50*time.Second))

	// Feed a tick past the bar boundary
	tick2 := makeTick("NIFTY", 18050.0, 1100)
	mat.Update(tick2, base.Add(65*time.Second)) // crosses into next bar

	select {
	case bar := <-mat.Completed():
		if bar.Symbol != "NIFTY" {
			t.Errorf("expected NIFTY, got %s", bar.Symbol)
		}
		if bar.Timeframe != "1m" {
			t.Errorf("expected 1m, got %s", bar.Timeframe)
		}
		if bar.Close != 18000.0 {
			t.Errorf("expected close 18000, got %f", bar.Close)
		}
	default:
		t.Error("expected a completed bar but got none")
	}
}

func TestMaterializerHighLowUpdated(t *testing.T) {
	mat := NewMaterializer([]string{"5m"}, 10)
	base := time.Date(2024, 1, 15, 9, 15, 0, 0, ist)

	prices := []float64{18000, 18100, 17950, 18050}
	for i, p := range prices {
		tick := makeTick("BANKNIFTY", p, 500)
		mat.Update(tick, base.Add(time.Duration(i*30)*time.Second))
	}

	// Cross bar boundary to flush
	mat.Update(makeTick("BANKNIFTY", 18060, 600), base.Add(6*time.Minute))

	select {
	case bar := <-mat.Completed():
		if bar.High != 18100 {
			t.Errorf("expected high 18100, got %f", bar.High)
		}
		if bar.Low != 17950 {
			t.Errorf("expected low 17950, got %f", bar.Low)
		}
		if bar.Open != 18000 {
			t.Errorf("expected open 18000, got %f", bar.Open)
		}
	default:
		t.Error("expected a completed bar")
	}
}

func TestMultipleTimeframes(t *testing.T) {
	tfs := []string{"1m", "5m", "15m"}
	mat := NewMaterializer(tfs, 10)
	base := time.Date(2024, 1, 15, 9, 15, 0, 0, ist)
	tick := makeTick("NIFTY", 18000, 1000)

	mat.Update(tick, base.Add(5*time.Second))

	// All 3 timeframe keys should exist in bars map
	mat.mu.Lock()
	defer mat.mu.Unlock()
	for _, tf := range tfs {
		key := instrumentKey{symbol: "NIFTY", exchange: "NSE", timeframe: tf}
		if _, ok := mat.bars[key]; !ok {
			t.Errorf("missing bar for timeframe %s", tf)
		}
	}
}

func TestMaterializerFirstTickSetsOpen(t *testing.T) {
	mat := NewMaterializer([]string{"1m"}, 10)
	base := time.Date(2024, 1, 15, 9, 15, 5, 0, ist)
	mat.Update(makeTick("NIFTY", 18250.5, 200), base)

	mat.mu.Lock()
	defer mat.mu.Unlock()
	key := instrumentKey{symbol: "NIFTY", exchange: "NSE", timeframe: "1m"}
	bar, ok := mat.bars[key]
	if !ok {
		t.Fatal("bar not initialized")
	}
	if bar.open != 18250.5 {
		t.Errorf("expected open 18250.5, got %f", bar.open)
	}
}
