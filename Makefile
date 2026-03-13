# Makefile — SDV Parking Demo monorepo build orchestration
#
# Targets:
#   proto       Generate Go code from .proto files
#   build       Compile all Rust and Go components
#   test        Run all unit tests (Rust + Go)
#   lint        Run linters (cargo clippy + go vet)
#   check       Run build, test, lint in sequence
#   clean       Remove all build artifacts
#   infra-up    Start local infrastructure (NATS + Kuksa Databroker)
#   infra-down  Stop and remove local infrastructure containers

PROTO_DIR      := proto
GEN_GO_DIR     := gen/go
DEPLOYMENTS    := deployments
RHIVOS_DIR     := rhivos

.PHONY: proto build test lint check clean infra-up infra-down

# ---------------------------------------------------------------------------
# Proto generation
# ---------------------------------------------------------------------------

proto:
	@command -v protoc >/dev/null 2>&1 || { \
		echo "Error: protoc is required but not installed."; \
		echo "Install protoc (Protocol Buffer compiler) from https://grpc.io/docs/protoc-installation/"; \
		exit 1; \
	}
	@mkdir -p $(GEN_GO_DIR)/commonpb $(GEN_GO_DIR)/updateservicepb $(GEN_GO_DIR)/parkingadaptorpb
	protoc --proto_path=$(PROTO_DIR) \
		--go_out=$(GEN_GO_DIR)/commonpb --go_opt=paths=source_relative \
		$(PROTO_DIR)/common.proto
	protoc --proto_path=$(PROTO_DIR) \
		--go_out=$(GEN_GO_DIR)/updateservicepb --go_opt=paths=source_relative \
		--go-grpc_out=$(GEN_GO_DIR)/updateservicepb --go-grpc_opt=paths=source_relative \
		$(PROTO_DIR)/update_service.proto
	protoc --proto_path=$(PROTO_DIR) \
		--go_out=$(GEN_GO_DIR)/parkingadaptorpb --go_opt=paths=source_relative \
		--go-grpc_out=$(GEN_GO_DIR)/parkingadaptorpb --go-grpc_opt=paths=source_relative \
		$(PROTO_DIR)/parking_adaptor.proto

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

build:
	@command -v cargo >/dev/null 2>&1 || { echo "Error: cargo (Rust toolchain) is required but not installed."; exit 1; }
	@command -v go >/dev/null 2>&1 || { echo "Error: go (Go toolchain) is required but not installed."; exit 1; }
	cd $(RHIVOS_DIR) && cargo build
	go build ./...

# ---------------------------------------------------------------------------
# Test
# ---------------------------------------------------------------------------

test:
	@command -v cargo >/dev/null 2>&1 || { echo "Error: cargo (Rust toolchain) is required but not installed."; exit 1; }
	@command -v go >/dev/null 2>&1 || { echo "Error: go (Go toolchain) is required but not installed."; exit 1; }
	cd $(RHIVOS_DIR) && cargo test
	go test ./...

# ---------------------------------------------------------------------------
# Lint
# ---------------------------------------------------------------------------

lint:
	@command -v cargo >/dev/null 2>&1 || { echo "Error: cargo (Rust toolchain) is required but not installed."; exit 1; }
	@command -v go >/dev/null 2>&1 || { echo "Error: go (Go toolchain) is required but not installed."; exit 1; }
	cd $(RHIVOS_DIR) && cargo clippy -- -D warnings
	go vet ./...

# ---------------------------------------------------------------------------
# Check (build + test + lint)
# ---------------------------------------------------------------------------

check: build test lint

# ---------------------------------------------------------------------------
# Clean
# ---------------------------------------------------------------------------

clean:
	cd $(RHIVOS_DIR) && cargo clean
	go clean ./...
	rm -rf $(GEN_GO_DIR)

# ---------------------------------------------------------------------------
# Infrastructure (Podman Compose)
# ---------------------------------------------------------------------------

infra-up:
	@command -v podman >/dev/null 2>&1 || { \
		echo "Error: podman is required but not installed."; \
		echo "Install Podman from https://podman.io/getting-started/installation"; \
		exit 1; \
	}
	podman compose -f $(DEPLOYMENTS)/compose.yml up -d

infra-down:
	@command -v podman >/dev/null 2>&1 || { \
		echo "Error: podman is required but not installed."; \
		exit 1; \
	}
	podman compose -f $(DEPLOYMENTS)/compose.yml down
