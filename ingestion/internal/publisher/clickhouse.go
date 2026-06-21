package publisher

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/ClickHouse/clickhouse-go/v2"
	"github.com/ClickHouse/clickhouse-go/v2/lib/driver"
	"go.uber.org/zap"

	"github.com/ertanuj96/AutoTrader/ingestion/internal/normalize"
)

// CHWriter batches TickEvents and CandleBars and flushes them to ClickHouse.
type CHWriter struct {
	conn          driver.Conn
	log           *zap.Logger
	tickBatch     []normalize.TickEvent
	candleBatch   []normalize.CandleBar
	mu            sync.Mutex
	batchSize     int
	flushInterval time.Duration
	stopCh        chan struct{}
}

// NewCHWriter opens a ClickHouse connection and returns a writer.
func NewCHWriter(
	host string, port int, db, user, pass string,
	batchSize int, flushInterval time.Duration,
	log *zap.Logger,
) (*CHWriter, error) {
	conn, err := clickhouse.Open(&clickhouse.Options{
		Addr: []string{fmt.Sprintf("%s:%d", host, port)},
		Auth: clickhouse.Auth{Database: db, Username: user, Password: pass},
		Settings: clickhouse.Settings{
			"max_execution_time": 30,
		},
		DialTimeout:      5 * time.Second,
		MaxOpenConns:     5,
		MaxIdleConns:     5,
		ConnMaxLifetime:  1 * time.Hour,
		ConnOpenStrategy: clickhouse.ConnOpenInOrder,
	})
	if err != nil {
		return nil, fmt.Errorf("clickhouse open: %w", err)
	}
	if err := conn.Ping(context.Background()); err != nil {
		return nil, fmt.Errorf("clickhouse ping: %w", err)
	}
	log.Info("ClickHouse connected", zap.String("host", host), zap.Int("port", port))
	return &CHWriter{
		conn:          conn,
		log:           log,
		batchSize:     batchSize,
		flushInterval: flushInterval,
		stopCh:        make(chan struct{}),
	}, nil
}

// AddTick enqueues a tick; flushes immediately if the batch is full.
func (w *CHWriter) AddTick(tick normalize.TickEvent) {
	w.mu.Lock()
	defer w.mu.Unlock()
	w.tickBatch = append(w.tickBatch, tick)
	if len(w.tickBatch) >= w.batchSize {
		w.flushTicksLocked()
	}
}

// AddCandle enqueues a candle; flushes immediately if the batch is full.
func (w *CHWriter) AddCandle(bar normalize.CandleBar) {
	w.mu.Lock()
	defer w.mu.Unlock()
	w.candleBatch = append(w.candleBatch, bar)
	if len(w.candleBatch) >= w.batchSize {
		w.flushCandlesLocked()
	}
}

// StartFlushLoop starts a background goroutine that flushes on a timer.
func (w *CHWriter) StartFlushLoop(ctx context.Context) {
	ticker := time.NewTicker(w.flushInterval)
	go func() {
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				w.Flush()
			case <-ctx.Done():
				w.Flush()
				return
			case <-w.stopCh:
				w.Flush()
				return
			}
		}
	}()
}

// Flush writes all pending batches to ClickHouse synchronously.
func (w *CHWriter) Flush() {
	w.mu.Lock()
	defer w.mu.Unlock()
	w.flushTicksLocked()
	w.flushCandlesLocked()
}

func (w *CHWriter) flushTicksLocked() {
	if len(w.tickBatch) == 0 {
		return
	}
	ctx := context.Background()
	batch, err := w.conn.PrepareBatch(ctx, "INSERT INTO pts_market.tick_data")
	if err != nil {
		w.log.Error("tick batch prepare", zap.Error(err))
		w.tickBatch = w.tickBatch[:0]
		return
	}
	for _, t := range w.tickBatch {
		if err := batch.Append(
			time.Now(), t.Symbol, t.Exchange, t.LTP,
			t.Bid, t.Ask, t.BidQty, t.AskQty, t.Volume, t.OI,
		); err != nil {
			w.log.Error("tick batch append", zap.Error(err))
		}
	}
	if err := batch.Send(); err != nil {
		w.log.Error("tick batch send", zap.Error(err))
	} else {
		w.log.Debug("flushed ticks", zap.Int("count", len(w.tickBatch)))
	}
	w.tickBatch = w.tickBatch[:0]
}

func (w *CHWriter) flushCandlesLocked() {
	if len(w.candleBatch) == 0 {
		return
	}
	ctx := context.Background()
	batch, err := w.conn.PrepareBatch(ctx, "INSERT INTO pts_market.candle_data")
	if err != nil {
		w.log.Error("candle batch prepare", zap.Error(err))
		w.candleBatch = w.candleBatch[:0]
		return
	}
	for _, c := range w.candleBatch {
		if err := batch.Append(
			c.BarStart, c.Symbol, c.Exchange, c.Timeframe,
			c.Open, c.High, c.Low, c.Close, c.Volume, c.OI,
		); err != nil {
			w.log.Error("candle batch append", zap.Error(err))
		}
	}
	if err := batch.Send(); err != nil {
		w.log.Error("candle batch send", zap.Error(err))
	} else {
		w.log.Debug("flushed candles", zap.Int("count", len(w.candleBatch)))
	}
	w.candleBatch = w.candleBatch[:0]
}

// Stop signals the flush loop to stop.
func (w *CHWriter) Stop() {
	select {
	case <-w.stopCh:
	default:
		close(w.stopCh)
	}
}
