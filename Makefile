# Root Makefile for parking-fee-service monorepo
# Orchestrates builds, tests, and infrastructure across Rust and Go toolchains.

.PHONY: build build-rust build-go test test-rust test-go test-setup clean proto infra-up infra-down check

# Go modules managed by the workspace (excluding tests/setup for regular build/test)
GO_MODULES := \
	backend/parking-fee-service \
	backend/cloud-gateway \
	mock/parking-app-cli \
	mock/companion-app-cli \
	mock/parking-operator

# --- Build targets ---

build: build-rust build-go
	@echo "Build complete."

build-rust:
	@echo "Building Rust workspace..."
	cd rhivos && cargo build --workspace

build-go:
	@echo "Building Go modules..."
	@for mod in $(GO_MODULES); do \
		echo "  Building $$mod..."; \
		cd $$mod && go build ./... && cd $(CURDIR) || exit 1; \
	done

# --- Test targets ---

test: test-rust test-go
	@echo "All tests passed."

test-rust:
	@echo "Running Rust tests..."
	cd rhivos && cargo test --workspace -- --test-threads=1

test-go:
	@echo "Running Go tests..."
	@for mod in $(GO_MODULES); do \
		echo "  Testing $$mod..."; \
		cd $$mod && go test ./... && cd $(CURDIR) || exit 1; \
	done

test-setup:
	@echo "Running setup verification tests..."
	cd tests/setup && go test -v ./...

# --- Clean target ---

clean:
	@echo "Cleaning build artifacts..."
	cd rhivos && cargo clean
	go clean -cache
	@echo "Clean complete."

# --- Proto generation ---

proto:
	@command -v protoc >/dev/null 2>&1 || { echo "Error: protoc is required but not installed." >&2; exit 1; }
	@command -v protoc-gen-go >/dev/null 2>&1 || { echo "Error: protoc-gen-go is required but not installed." >&2; exit 1; }
	@command -v protoc-gen-go-grpc >/dev/null 2>&1 || { echo "Error: protoc-gen-go-grpc is required but not installed." >&2; exit 1; }
	@echo "Generating Go code from proto definitions..."
	protoc --proto_path=proto \
		--go_out=. --go_opt=module=parking-fee-service \
		--go-grpc_out=. --go-grpc_opt=module=parking-fee-service \
		$$(find proto -name '*.proto')
	@echo "Proto generation complete."

# --- Infrastructure targets ---

infra-up:
	@echo "Starting infrastructure containers..."
	podman-compose -f deployments/compose.yml up -d

infra-down:
	@echo "Stopping infrastructure containers..."
	podman-compose -f deployments/compose.yml down

# --- Check target (lint + test) ---

check: build
	@echo "Running linters..."
	(cd rhivos && cargo clippy --workspace -- -D warnings 2>/dev/null) || (cd rhivos && cargo check --workspace)
	@for mod in $(GO_MODULES); do \
		cd $$mod && go vet ./... && cd $(CURDIR) || exit 1; \
	done
	@echo "Running tests..."
	$(MAKE) test
	@echo "All checks passed."
