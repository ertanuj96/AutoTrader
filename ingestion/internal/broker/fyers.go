package broker

import (
	"encoding/json"
	"fmt"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

// FyersAdapter connects to Fyers WebSocket v3 market feed and emits RawTicks.
// Docs: https://myapi.fyers.in/docsv3
const fyersFeedURL = "wss://socket.fyers.in/trade/v3"

// FyersAdapter implements Adapter for the Fyers broker.
type FyersAdapter struct {
	accessToken string
	appID       string
	log         *zap.Logger
	conn        *websocket.Conn
	mu          sync.Mutex
	tickCh      chan RawTick
	errCh       chan error
	done        chan struct{}
}

// fyersQuote is the JSON market data message from Fyers.
type fyersQuote struct {
	T      string  `json:"T"`  // message type: "sf" for tick
	FY     string  `json:"fy"` // "NSE:NIFTY-INDEX"
	LTP    float64 `json:"lp"`
	Open   float64 `json:"op"`
	High   float64 `json:"h"`
	Low    float64 `json:"l"`
	Close  float64 `json:"c"` // previous close
	Volume uint64  `json:"v"`
	OI     uint64  `json:"oi"`
	Bid    float64 `json:"bp"`
	Ask    float64 `json:"sp"`
	BidQty uint64  `json:"bq"`
	AskQty uint64  `json:"sq"`
}

// NewFyersAdapter creates a Fyers market feed adapter.
func NewFyersAdapter(accessToken, appID string, log *zap.Logger) *FyersAdapter {
	return &FyersAdapter{
		accessToken: accessToken,
		appID:       appID,
		log:         log,
		tickCh:      make(chan RawTick, 1000),
		errCh:       make(chan error, 10),
		done:        make(chan struct{}),
	}
}

func (f *FyersAdapter) Name() string { return "fyers" }

func (f *FyersAdapter) Connect() error {
	conn, _, err := websocket.DefaultDialer.Dial(fyersFeedURL, nil)
	if err != nil {
		return fmt.Errorf("fyers: dial: %w", err)
	}
	f.mu.Lock()
	f.conn = conn
	f.mu.Unlock()

	authMsg := map[string]interface{}{
		"T": "conn",
		"A": fmt.Sprintf("%s:%s", f.appID, f.accessToken),
	}
	if err := conn.WriteJSON(authMsg); err != nil {
		return fmt.Errorf("fyers: auth: %w", err)
	}
	f.log.Info("Fyers WebSocket connected")
	go f.readLoop()
	return nil
}

func (f *FyersAdapter) Subscribe(instruments []string) error {
	msg := map[string]interface{}{
		"T":      "SUB_L2",
		"L2List": instruments,
	}
	f.mu.Lock()
	defer f.mu.Unlock()
	if f.conn == nil {
		return fmt.Errorf("fyers: not connected")
	}
	return f.conn.WriteJSON(msg)
}

func (f *FyersAdapter) Ticks() <-chan RawTick { return f.tickCh }
func (f *FyersAdapter) Errors() <-chan error  { return f.errCh }

func (f *FyersAdapter) Close() error {
	select {
	case <-f.done:
	default:
		close(f.done)
	}
	f.mu.Lock()
	defer f.mu.Unlock()
	if f.conn != nil {
		return f.conn.Close()
	}
	return nil
}

func (f *FyersAdapter) readLoop() {
	for {
		select {
		case <-f.done:
			return
		default:
		}
		f.mu.Lock()
		conn := f.conn
		f.mu.Unlock()
		if conn == nil {
			return
		}
		_, msg, err := conn.ReadMessage()
		if err != nil {
			select {
			case f.errCh <- err:
			default:
			}
			return
		}
		var q fyersQuote
		if err := json.Unmarshal(msg, &q); err != nil || q.LTP <= 0 {
			continue
		}
		exch, sym := parseInstrument(q.FY)
		tick := RawTick{
			Symbol:    sym,
			Exchange:  exch,
			LTP:       q.LTP,
			Open:      q.Open,
			High:      q.High,
			Low:       q.Low,
			PrevClose: q.Close,
			Volume:    q.Volume,
			OI:        q.OI,
			Bid:       q.Bid,
			Ask:       q.Ask,
			BidQty:    q.BidQty,
			AskQty:    q.AskQty,
			Timestamp: time.Now(),
		}
		select {
		case f.tickCh <- tick:
		default:
			f.log.Warn("fyers: tick channel full, dropping", zap.String("symbol", tick.Symbol))
		}
	}
}
