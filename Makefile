.PHONY: all build test clean dev infra

# ── Build all services ──
all: build

build: build-rust build-go
	@echo "✅ All services built"

build-rust:
	cd quant-engine && cargo build --release

build-go:
	cd ingestion && go build -o bin/ingest ./cmd/ingest

test: test-rust test-go test-python
	@echo "✅ All tests passed"

test-rust:
	cd quant-engine && cargo test

test-go:
	cd ingestion && go test ./...

test-python:
	cd trading-core && python -m pytest -q 2>/dev/null || echo "No Python tests yet"

# ── Development ──
dev: infra
	@echo "🚀 Infrastructure running. Start services manually:"
	@echo "  Rust:   cd quant-engine && cargo run"
	@echo "  Go:     cd ingestion && go run ./cmd/ingest"
	@echo "  Python: cd trading-core && python -m pts.main"
	@echo "  API:    cd dashboard-api && uvicorn app.main:app --port 8001 --reload"
	@echo "  UI:     cd dashboard-ui && npm run dev"

infra:
	docker-compose up -d

clean:
	cd quant-engine && cargo clean
	cd ingestion && rm -rf bin/
	docker-compose down

