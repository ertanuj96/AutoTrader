package config

import (
	"os"
	"strconv"
	"strings"
	"time"
)

// Config holds all runtime configuration loaded from environment variables.
type Config struct {
	NatsURL          string
	ClickHouseHost   string
	ClickHousePort   int
	ClickHouseDB     string
	ClickHouseUser   string
	ClickHousePass   string
	DhanAccessToken  string
	DhanClientID     string
	FyersAccessToken string
	FyersAppID       string
	Instruments      []string      // e.g. ["NSE:41854", "NSE:35001"]
	Timeframes       []string      // e.g. ["1m","5m","15m","1h","4h"]
	CHBatchSize      int
	CHFlushInterval  time.Duration
	ReconnectDelay   time.Duration
}

func Load() *Config {
	return &Config{
		NatsURL:          getEnv("NATS_URL", "nats://localhost:4222"),
		ClickHouseHost:   getEnv("CLICKHOUSE_HOST", "localhost"),
		ClickHousePort:   getEnvInt("CLICKHOUSE_PORT", 9000),
		ClickHouseDB:     getEnv("CLICKHOUSE_DB", "pts_market"),
		ClickHouseUser:   getEnv("CLICKHOUSE_USER", "default"),
		ClickHousePass:   getEnv("CLICKHOUSE_PASS", ""),
		DhanAccessToken:  getEnv("DHAN_ACCESS_TOKEN", ""),
		DhanClientID:     getEnv("DHAN_CLIENT_ID", ""),
		FyersAccessToken: getEnv("FYERS_ACCESS_TOKEN", ""),
		FyersAppID:       getEnv("FYERS_APP_ID", ""),
		Instruments:      getEnvCSV("INSTRUMENTS", []string{}),
		Timeframes:       getEnvCSV("TIMEFRAMES", []string{"1m", "5m", "15m", "1h", "4h"}),
		CHBatchSize:      getEnvInt("CH_BATCH_SIZE", 500),
		CHFlushInterval:  getEnvDuration("CH_FLUSH_INTERVAL", 5*time.Second),
		ReconnectDelay:   getEnvDuration("RECONNECT_DELAY", 3*time.Second),
	}
}

func getEnv(key, def string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return def
}

func getEnvInt(key string, def int) int {
	if v := os.Getenv(key); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			return n
		}
	}
	return def
}

func getEnvDuration(key string, def time.Duration) time.Duration {
	if v := os.Getenv(key); v != "" {
		if d, err := time.ParseDuration(v); err == nil {
			return d
		}
	}
	return def
}

func getEnvCSV(key string, def []string) []string {
	v := os.Getenv(key)
	if v == "" {
		return def
	}
	parts := strings.Split(v, ",")
	result := make([]string, 0, len(parts))
	for _, p := range parts {
		if s := strings.TrimSpace(p); s != "" {
			result = append(result, s)
		}
	}
	return result
}
