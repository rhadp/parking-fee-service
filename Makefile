.PHONY: all build test lint check clean

# ── Default ───────────────────────────────────────────────────────────────────
all: build

# ── Build ────────────────────────────────────────────────────────────────────
build:
	cd rhivos && cargo build -p mock-sensors
	cd mock && go build ./...

# ── Test ─────────────────────────────────────────────────────────────────────
test: test-rust test-go test-integration

test-rust:
	cd rhivos && cargo test -p mock-sensors

test-go:
	cd mock && go test -v ./...

test-integration:
	cd tests/mock-apps && go test -v ./...

# ── Lint ─────────────────────────────────────────────────────────────────────
lint: lint-rust lint-go

lint-rust:
	cd rhivos && cargo clippy -p mock-sensors -- -D warnings

lint-go:
	go vet github.com/sdv-demo/mock/... github.com/sdv-demo/tests/mock-apps/...

# ── Check (lint + test) ───────────────────────────────────────────────────────
check: lint test

# ── Clean ────────────────────────────────────────────────────────────────────
clean:
	cd rhivos && cargo clean -p mock-sensors
	go clean ./...
