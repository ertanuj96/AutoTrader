package broker

import (
	"encoding/json"
	"fmt"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

// Dhan MARKETFEED WebSocket adapter.
// Docs: https://dhanhq.co/docs/v2/marketfeed/
const dhanFeedURL = "wss://api-feed.dhan.co"

// DhanAdapter connects to Dhan's market feed WebSocket and emits RawTicks.
type DhanAdapter struct {
	accessToken string
	clientID    string
	log         *zap.Logger
	conn        *websocket.Conn
	mu          sync.Mutex
	tickCh      chan RawTick
	errCh       chan error
	done        chan struct{}
}

// dhanQuote is the JSON market data message received from Dhan.
type dhanQuote struct {
	MsgCode    int     `json:"MsgCode"`
	ExchSeg    string  `json:"ExchSeg"`
	SecurityID string  `json:"SecurityID"`
	LTP        float64 `json:"LTP"`
	Open       float64 `json:"Open"`
	High       float64 `json:"High"`
	Low        float64 `json:"Low"`
	Close      float64 `json:"Close"`
	Volume     uint64  `json:"Volume"`
	OI         uint64  `json:"OI"`
	BidPrice   float64 `json:"BidPrice"`
	AskPrice   float64 `json:"AskPrice"`
	BidQty     uint64  `json:"BidQty"`
	AskQty     uint64  `json:"AskQty"`
}

// NewDhanAdapter creates a Dhan market feed adapter.
func NewDhanAdapter(accessToken, clientID string, log *zap.Logger) *DhanAdapter {
	return &DhanAdapter{
		accessToken: accessToken,
		clientID:    clientID,
		log:         log,
		tickCh:      make(chan RawTick, 1000),
		errCh:       make(chan error, 10),
		done:        make(chan struct{}),
	}
}

func (d *DhanAdapter) Name() string { return "dhan" }

func (d *DhanAdapter) Connect() error {
	dialer := websocket.Dialer{HandshakeTimeout: 10 * time.Second}
	headers := http.Header{"Authorization": {"Bearer " + d.accessToken}}
	conn, _, err := dialer.Dial(dhanFeedURL, headers)
	if err != nil {
		return fmt.Errorf("dhan: dial: %w", err)
	}
	d.mu.Lock()
	d.conn = conn
	d.mu.Unlock()

	authMsg := map[string]interface{}{
		"LoginReq": map[string]interface{}{
			"MsgCode":  42,
			"ClientId": d.clientID,
			"Token":    d.accessToken,
		},
	}
	if err := conn.WriteJSON(authMsg); err != nil {
		return fmt.Errorf("dhan: auth: %w", err)
	}
	d.log.Info("Dhan WebSocket connected and authenticated")
	go d.readLoop()
	return nil
}

func (d *DhanAdapter) Subscribe(instruments []string) error {
	iList := make([]map[string]interface{}, 0, len(instruments))
	for _, inst := range instruments {
		exch, secID := parseInstrument(inst)
		iList = append(iList, map[string]interface{}{
			"ExchangeSegment": dhanExchangeSegment(exch),
			"SecurityId":      secID,
		})
	}
	msg := map[string]interface{}{
		"RequestCode":     21,
		"InstrumentCount": len(iList),
		"InstrumentList":  iList,
	}
	d.mu.Lock()
	defer d.mu.Unlock()
	if d.conn == nil {
		return fmt.Errorf("dhan: not connected")
	}
	return d.conn.WriteJSON(msg)
}

func (d *DhanAdapter) Ticks() <-chan RawTick  { return d.tickCh }
func (d *DhanAdapter) Errors() <-chan error   { return d.errCh }

func (d *DhanAdapter) Close() error {
	select {
	case <-d.done:
	default:
		close(d.done)
	}
	d.mu.Lock()
	defer d.mu.Unlock()
	if d.conn != nil {
		return d.conn.Close()
	}
	return nil
}

func (d *DhanAdapter) readLoop() {
	for {
		select {
		case <-d.done:
			return
		default:
		}
		d.mu.Lock()
		conn := d.conn
		d.mu.Unlock()
		if conn == nil {
			return
		}
		_, msg, err := conn.ReadMessage()
		if err != nil {
			select {
			case d.errCh <- err:
			default:
			}
			return
		}
		var q dhanQuote
		if err := json.Unmarshal(msg, &q); err != nil || q.LTP <= 0 {
			continue
		}
		tick := RawTick{
			Symbol:    q.SecurityID,
			Exchange:  dhanSegmentToExchange(q.ExchSeg),
			LTP:       q.LTP,
			Open:      q.Open,
			High:      q.High,
			Low:       q.Low,
			PrevClose: q.Close,
			Volume:    q.Volume,
			OI:        q.OI,
			Bid:       q.BidPrice,
			Ask:       q.AskPrice,
			BidQty:    q.BidQty,
			AskQty:    q.AskQty,
			Timestamp: time.Now(),
		}
		select {
		case d.tickCh <- tick:
		default:
			d.log.Warn("dhan: tick channel full, dropping", zap.String("symbol", tick.Symbol))
		}
	}
}

func dhanExchangeSegment(exch string) string {
	switch exch {
	case "NSE":
		return "NSE_FNO"
	case "BSE":
		return "BSE_EQ"
	case "MCX":
		return "MCX_COMM"
	default:
		return "NSE_FNO"
	}
}

func dhanSegmentToExchange(seg string) string {
	switch seg {
	case "NSE_EQ", "NSE_FNO":
		return "NSE"
	case "MCX_COMM":
		return "MCX"
	case "BSE_EQ":
		return "BSE"
	default:
		return seg
	}
}

// parseInstrument splits "NSE:41854" into ("NSE", "41854").
func parseInstrument(inst string) (exchange, secID string) {
	for i, c := range inst {
		if c == ':' {
			return inst[:i], inst[i+1:]
		}
	}
	return "NSE", inst
}
