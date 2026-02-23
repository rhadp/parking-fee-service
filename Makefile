# Top-level Makefile for SDV Parking Demo System
# Orchestrates builds, tests, linting, proto generation, and local infrastructure.

.PHONY: build test lint clean proto check infra-up infra-down help \
        build-rust build-go

# ──────────────────────────────────────────────────────────────────────
# Toolchain detection helpers
# ──────────────────────────────────────────────────────────────────────

RUSTC := $(shell command -v rustc 2>/dev/null)
CARGO := $(shell command -v cargo 2>/dev/null)
GO    := $(shell command -v go 2>/dev/null)
PROTOC := $(shell command -v protoc 2>/dev/null)

# Container runtime detection: prefer podman, fall back to docker
CONTAINER_RUNTIME :=
COMPOSE_CMD :=
ifneq ($(shell command -v podman 2>/dev/null),)
  CONTAINER_RUNTIME := podman
  ifneq ($(shell command -v podman-compose 2>/dev/null),)
    COMPOSE_CMD := podman-compose
  else ifneq ($(shell command -v podman 2>/dev/null),)
    COMPOSE_CMD := podman compose
  endif
else ifneq ($(shell command -v docker 2>/dev/null),)
  CONTAINER_RUNTIME := docker
  COMPOSE_CMD := docker compose
endif

# Guard macros — fail early with a clear message naming the missing tool.
define require-rust
	@if [ -z "$(RUSTC)" ]; then \
		echo "Error: rustc not found. Please install the Rust toolchain (https://rustup.rs)." >&2; \
		exit 1; \
	fi
	@if [ -z "$(CARGO)" ]; then \
		echo "Error: cargo not found. Please install the Rust toolchain (https://rustup.rs)." >&2; \
		exit 1; \
	fi
endef

define require-go
	@if [ -z "$(GO)" ]; then \
		echo "Error: go not found. Please install Go (https://go.dev/dl/)." >&2; \
		exit 1; \
	fi
endef

define require-protoc
	@if [ -z "$(PROTOC)" ]; then \
		echo "Error: protoc not found. Please install Protocol Buffers compiler (https://grpc.io/docs/protoc-installation/)." >&2; \
		exit 1; \
	fi
endef

define require-container
	@if [ -z "$(CONTAINER_RUNTIME)" ]; then \
		echo "Error: podman or docker not found. Please install a container runtime." >&2; \
		echo "  Podman: https://podman.io/getting-started/installation" >&2; \
		echo "  Docker: https://docs.docker.com/get-docker/" >&2; \
		exit 1; \
	fi
	@if [ -z "$(COMPOSE_CMD)" ]; then \
		echo "Error: podman-compose or docker compose not found. Please install a compose tool." >&2; \
		exit 1; \
	fi
endef

# ──────────────────────────────────────────────────────────────────────
# Go modules
# ──────────────────────────────────────────────────────────────────────

GO_BACKEND_MODULES := backend/parking-fee-service backend/cloud-gateway
GO_MOCK_MODULES    := mock/parking-app-cli mock/companion-app-cli mock/parking-operator
GO_ALL_MODULES     := $(GO_BACKEND_MODULES) $(GO_MOCK_MODULES)

# ──────────────────────────────────────────────────────────────────────
# Default target
# ──────────────────────────────────────────────────────────────────────

.DEFAULT_GOAL := help

help: ## Show this help
	@echo "Usage: make <target>"
	@echo ""
	@echo "Targets:"
	@echo "  build       Build all Rust and Go components"
	@echo "  test        Run all unit tests (Rust + Go)"
	@echo "  lint        Run linters: cargo clippy, go vet"
	@echo "  clean       Remove all build artifacts"
	@echo "  proto       Regenerate Go code from .proto files"
	@echo "  check       Run build + test + lint in sequence"
	@echo "  infra-up    Start local infrastructure (Mosquitto, Kuksa)"
	@echo "  infra-down  Stop local infrastructure"
	@echo "  help        Show this help"

# ──────────────────────────────────────────────────────────────────────
# build — Build all components. Rust builds first, then each Go module.
#          Continues building remaining components if one fails.
# ──────────────────────────────────────────────────────────────────────

build: ## Build all Rust and Go components
	$(require-rust)
	$(require-go)
	@echo "==> Building Rust workspace..."
	@BUILD_FAILED=0; \
	cd rhivos && cargo build 2>&1 || BUILD_FAILED=1; \
	cd ..; \
	echo "==> Building Go modules..."; \
	for mod in $(GO_ALL_MODULES); do \
		echo "  -> $$mod"; \
		(cd $$mod && go build ./...) || BUILD_FAILED=1; \
	done; \
	if [ $$BUILD_FAILED -ne 0 ]; then \
		echo "Error: one or more components failed to build." >&2; \
		exit 1; \
	fi; \
	echo "==> Build complete."

# ──────────────────────────────────────────────────────────────────────
# test — Run all unit tests (Rust + Go)
# ──────────────────────────────────────────────────────────────────────

test: ## Run all unit tests (Rust + Go)
	$(require-rust)
	$(require-go)
	@echo "==> Running Rust tests..."
	@cd rhivos && cargo test
	@echo "==> Running Go tests..."
	@for mod in $(GO_ALL_MODULES); do \
		echo "  -> $$mod"; \
		(cd $$mod && go test ./...) || exit 1; \
	done
	@echo "==> All tests passed."

# ──────────────────────────────────────────────────────────────────────
# lint — Run linters for all components
# ──────────────────────────────────────────────────────────────────────

lint: ## Run linters: cargo clippy, go vet
	$(require-rust)
	$(require-go)
	@echo "==> Running cargo clippy..."
	@cd rhivos && cargo clippy -- -D warnings
	@echo "==> Running go vet..."
	@for mod in $(GO_ALL_MODULES) gen/go; do \
		echo "  -> $$mod"; \
		(cd $$mod && go vet ./...) || exit 1; \
	done
	@echo "==> Lint passed."

# ──────────────────────────────────────────────────────────────────────
# clean — Remove all build artifacts
# ──────────────────────────────────────────────────────────────────────

clean: ## Remove all build artifacts
	@echo "==> Cleaning Rust build artifacts..."
	@if [ -d rhivos/target ]; then rm -rf rhivos/target; fi
	@echo "==> Cleaning Go binaries..."
	@for app in parking-app-cli companion-app-cli; do \
		rm -f mock/$$app/$$app; \
	done
	@rm -f backend/parking-fee-service/parking-fee-service
	@rm -f backend/cloud-gateway/cloud-gateway
	@echo "==> Clean complete."

# ──────────────────────────────────────────────────────────────────────
# proto — Regenerate Go code from .proto definitions
# ──────────────────────────────────────────────────────────────────────

proto: ## Regenerate Go code from .proto files
	$(require-protoc)
	@echo "==> Generating Go protobuf code..."
	@protoc \
		--proto_path=proto/ \
		--go_out=gen/go/ \
		--go_opt=module=github.com/rhadp/parking-fee-service/gen/go \
		--go-grpc_out=gen/go/ \
		--go-grpc_opt=module=github.com/rhadp/parking-fee-service/gen/go \
		proto/*.proto
	@echo "==> Proto generation complete."

# ──────────────────────────────────────────────────────────────────────
# check — Run build + test + lint in sequence
# ──────────────────────────────────────────────────────────────────────

check: build test lint ## Run build + test + lint in sequence
	@echo "==> All checks passed."

# ──────────────────────────────────────────────────────────────────────
# infra-up / infra-down — Local infrastructure lifecycle
# ──────────────────────────────────────────────────────────────────────

infra-up: ## Start local infrastructure (Mosquitto, Kuksa)
	$(require-container)
	@echo "==> Starting infrastructure..."
	@cd infra && $(COMPOSE_CMD) up -d
	@echo "==> Infrastructure started."
	@echo "  Mosquitto MQTT : localhost:1883"
	@echo "  Kuksa Databroker: localhost:55556"

infra-down: ## Stop local infrastructure
	$(require-container)
	@echo "==> Stopping infrastructure..."
	@cd infra && $(COMPOSE_CMD) down
	@echo "==> Infrastructure stopped."
