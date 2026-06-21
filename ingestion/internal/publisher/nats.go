package publisher

import (
	"encoding/json"
	"fmt"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"

	"github.com/ertanuj96/AutoTrader/ingestion/internal/normalize"
)

// NATS subjects — must match the PTS event bus specification.
const (
	SubjectTick   = "pts.market.tick"
	SubjectCandle = "pts.market.candle"
)

// NatsPublisher publishes TickEvents and CandleBars to NATS.
type NatsPublisher struct {
	nc  *nats.Conn
	log *zap.Logger
}

// NewNatsPublisher connects to NATS and returns a publisher.
func NewNatsPublisher(url string, log *zap.Logger) (*NatsPublisher, error) {
	opts := []nats.Option{
		nats.Name("autotrader-ingestion"),
		nats.RetryOnFailedConnect(true),
		nats.MaxReconnects(-1),
		nats.ReconnectWait(2 * time.Second),
		nats.DisconnectErrHandler(func(_ *nats.Conn, err error) {
			log.Warn("NATS disconnected", zap.Error(err))
		}),
		nats.ReconnectHandler(func(nc *nats.Conn) {
			log.Info("NATS reconnected", zap.String("url", nc.ConnectedUrl()))
		}),
	}
	nc, err := nats.Connect(url, opts...)
	if err != nil {
		return nil, fmt.Errorf("nats connect %s: %w", url, err)
	}
	log.Info("NATS publisher connected", zap.String("url", url))
	return &NatsPublisher{nc: nc, log: log}, nil
}

// PublishTick serializes a TickEvent to JSON and publishes to pts.market.tick.
func (p *NatsPublisher) PublishTick(tick normalize.TickEvent) error {
	data, err := json.Marshal(tick)
	if err != nil {
		return fmt.Errorf("marshal tick: %w", err)
	}
	return p.nc.Publish(SubjectTick, data)
}

// PublishCandle serializes a CandleBar to JSON and publishes to pts.market.candle.
func (p *NatsPublisher) PublishCandle(bar normalize.CandleBar) error {
	data, err := json.Marshal(bar)
	if err != nil {
		return fmt.Errorf("marshal candle: %w", err)
	}
	return p.nc.Publish(SubjectCandle, data)
}

// Close drains pending messages and disconnects.
func (p *NatsPublisher) Close() {
	_ = p.nc.Drain()
}
