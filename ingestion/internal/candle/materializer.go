package candle

import (
	"sync"
	"time"

	"github.com/ertanuj96/AutoTrader/ingestion/internal/normalize"
)

// Supported timeframe durations.
var timeframeDurations = map[string]time.Duration{
	"1m":  1 * time.Minute,
	"5m":  5 * time.Minute,
	"15m": 15 * time.Minute,
	"1h":  1 * time.Hour,
	"4h":  4 * time.Hour,
}

type instrumentKey struct {
	symbol    string
	exchange  string
	timeframe string
}

type activeBar struct {
	open, high, low, close float64
	volume                 uint64
	oi                     uint64
	barStart, barEnd       time.Time
}

// Materializer converts a stream of TickEvents into completed OHLCV candles
// aligned to IST (UTC+5:30) bar boundaries.
type Materializer struct {
	timeframes []string
	bars       map[instrumentKey]*activeBar
	mu         sync.Mutex
	out        chan normalize.CandleBar
}

// NewMaterializer creates a Materializer for the given timeframes.
// bufSize controls the size of the completed-candle output channel.
func NewMaterializer(timeframes []string, bufSize int) *Materializer {
	return &Materializer{
		timeframes: timeframes,
		bars:       make(map[instrumentKey]*activeBar),
		out:        make(chan normalize.CandleBar, bufSize),
	}
}

// Completed returns the channel that emits completed candles.
func (m *Materializer) Completed() <-chan normalize.CandleBar { return m.out }

// Update feeds a tick into the materializer at the given wall-clock timestamp.
// Completed candles are sent to Completed() when a bar boundary is crossed.
func (m *Materializer) Update(tick normalize.TickEvent, ts time.Time) {
	m.mu.Lock()
	defer m.mu.Unlock()

	for _, tf := range m.timeframes {
		d, ok := timeframeDurations[tf]
		if !ok {
			continue
		}
		barStart := barAlignIST(ts, d)
		barEnd := barStart.Add(d)
		key := instrumentKey{symbol: tick.Symbol, exchange: tick.Exchange, timeframe: tf}

		bar, exists := m.bars[key]
		if !exists {
			m.bars[key] = &activeBar{
				open: tick.LTP, high: tick.LTP, low: tick.LTP, close: tick.LTP,
				volume: tick.Volume, oi: tick.OI,
				barStart: barStart, barEnd: barEnd,
			}
			continue
		}

		// Boundary crossed → emit completed bar, start new
		if !ts.Before(bar.barEnd) {
			select {
			case m.out <- normalize.CandleBar{
				Symbol:    tick.Symbol,
				Exchange:  tick.Exchange,
				Timeframe: tf,
				Open:      bar.open,
				High:      bar.high,
				Low:       bar.low,
				Close:     bar.close,
				Volume:    bar.volume,
				OI:        bar.oi,
				BarStart:  bar.barStart,
				BarEnd:    bar.barEnd,
			}:
			default:
				// Consumer is slow; drop rather than block
			}
			m.bars[key] = &activeBar{
				open: tick.LTP, high: tick.LTP, low: tick.LTP, close: tick.LTP,
				volume: tick.Volume, oi: tick.OI,
				barStart: barStart, barEnd: barEnd,
			}
			continue
		}

		// Update the active bar
		if tick.LTP > bar.high {
			bar.high = tick.LTP
		}
		if tick.LTP < bar.low {
			bar.low = tick.LTP
		}
		bar.close = tick.LTP
		bar.volume = tick.Volume
		bar.oi = tick.OI
	}
}

// barAlignIST aligns a UTC timestamp to the floor of d in IST (UTC+5:30).
func barAlignIST(t time.Time, d time.Duration) time.Time {
	ist := time.FixedZone("IST", 5*3600+30*60)
	local := t.In(ist)
	// Unix seconds since epoch in IST timezone context
	totalSec := local.Unix()
	dSec := int64(d.Seconds())
	alignedSec := totalSec - (totalSec % dSec)
	return time.Unix(alignedSec, 0).UTC()
}
