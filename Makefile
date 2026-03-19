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
#   e2e-build   Cross-compile binaries for Linux containers
#   e2e-images  Build E2E container images from pre-built binaries
#   e2e-up      Build images and start all services for E2E testing
#   e2e-down    Stop and remove E2E containers
#   e2e-test    Run E2E tests (starts stack, runs tests, tears down)
#   e2e-run-SERVICE  Start a single E2E service (+ its dependencies)
#                    e.g. make e2e-run-cloud-gateway-client

PROTO_DIR      := proto
GEN_GO_DIR     := gen/go
DEPLOYMENTS    := deployments
RHIVOS_DIR     := rhivos
BUILD_DIR      := build
GOARCH         := $(shell go env GOARCH)
E2E_PREFIX     := parking-e2e

RUST_E2E_BINARIES := update-service parking-operator-adaptor locking-service \
                      cloud-gateway-client location-sensor

E2E_SERVICES := nats databroker parking-fee-service cloud-gateway \
                mock-parking-operator update-service parking-operator-adaptor \
                locking-service cloud-gateway-client mock-sensors

.PHONY: proto build test lint check clean infra-up infra-down \
        e2e-build e2e-images e2e-up e2e-down e2e-test \
        $(addprefix e2e-run-,$(E2E_SERVICES))

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

check: proto build test lint

# ---------------------------------------------------------------------------
# Clean
# ---------------------------------------------------------------------------

clean:
	cd $(RHIVOS_DIR) && cargo clean
	go clean ./...
	rm -rf $(GEN_GO_DIR)
	rm -rf $(BUILD_DIR)

# ---------------------------------------------------------------------------
# Infrastructure (Podman Compose)
# ---------------------------------------------------------------------------

infra-up:
	@command -v podman >/dev/null 2>&1 || { \
		echo "Error: podman is required but not installed."; \
		echo "Install Podman from https://podman.io/getting-started/installation"; \
		exit 1; \
	}
	@mkdir -p /tmp/kuksa
	@podman machine ssh "mkdir -p /tmp/kuksa" 2>/dev/null || true
	podman compose -f $(DEPLOYMENTS)/compose.yml up -d

infra-down:
	@command -v podman >/dev/null 2>&1 || { \
		echo "Error: podman is required but not installed."; \
		exit 1; \
	}
	podman compose -f $(DEPLOYMENTS)/compose.yml down

# ---------------------------------------------------------------------------
# E2E: cross-compile binaries for Linux containers
# ---------------------------------------------------------------------------

e2e-build:
	@command -v go >/dev/null 2>&1 || { echo "Error: go (Go toolchain) is required but not installed."; exit 1; }
	@command -v podman >/dev/null 2>&1 || { echo "Error: podman is required but not installed."; exit 1; }
	@mkdir -p $(BUILD_DIR)
	@echo "==> Cross-compiling Go services for linux/$(GOARCH)..."
	CGO_ENABLED=0 GOOS=linux GOARCH=$(GOARCH) go build -o $(BUILD_DIR)/parking-fee-service ./backend/parking-fee-service/
	CGO_ENABLED=0 GOOS=linux GOARCH=$(GOARCH) go build -o $(BUILD_DIR)/cloud-gateway ./backend/cloud-gateway/
	CGO_ENABLED=0 GOOS=linux GOARCH=$(GOARCH) go build -o $(BUILD_DIR)/parking-operator ./mock/parking-operator/
	@echo "==> Building Rust services for linux (in container)..."
	podman build -f Containerfile.rust-builder -t parking-rust-builder .
	podman run --rm \
		-v $$(pwd)/$(RHIVOS_DIR):/src:Z \
		-v parking-rust-cargo:/usr/local/cargo/registry:Z \
		-v parking-rust-target:/tmp/target:Z \
		-v $$(pwd)/$(BUILD_DIR):/out:Z \
		parking-rust-builder \
		sh -c 'cd /src && cargo build --release --target-dir /tmp/target && \
			for bin in $(RUST_E2E_BINARIES); do cp /tmp/target/release/$$bin /out/$$bin; done'

# ---------------------------------------------------------------------------
# E2E: build container images from pre-built binaries
# ---------------------------------------------------------------------------

e2e-images: e2e-build
	@echo "==> Building E2E container images..."
	@for bin in $$(ls $(BUILD_DIR)/); do \
		echo "    $(E2E_PREFIX)/$$bin"; \
		podman build -q -f Containerfile.e2e --build-arg BINARY=$$bin -t $(E2E_PREFIX)/$$bin . ; \
	done

# ---------------------------------------------------------------------------
# End-to-end testing (Podman Compose)
# ---------------------------------------------------------------------------

e2e-up: e2e-images
	@mkdir -p /tmp/kuksa
	@podman machine ssh "mkdir -p /tmp/kuksa" 2>/dev/null || true
	podman compose -f $(DEPLOYMENTS)/compose.e2e.yml up -d

e2e-down:
	podman compose -f $(DEPLOYMENTS)/compose.e2e.yml down

e2e-test: e2e-up
	go test ./tests/e2e/ -v -timeout 120s
	$(MAKE) e2e-down

# ---------------------------------------------------------------------------
# E2E: start individual services (+ dependencies)
# ---------------------------------------------------------------------------
# Usage:
# make e2e-run-cloud-gateway-client   # starts nats + databroker + cloud-gateway-client
# make e2e-run-parking-fee-service    # starts only parking-fee-service (no deps)
# make e2e-run-parking-operator-adaptor  # starts databroker + mock-parking-operator + adaptor
#
# Compose automatically starts any depends_on services that aren't running.

$(addprefix e2e-run-,$(E2E_SERVICES)): e2e-run-%: e2e-images
	@mkdir -p /tmp/kuksa
	@podman machine ssh "mkdir -p /tmp/kuksa" 2>/dev/null || true
	podman compose -f $(DEPLOYMENTS)/compose.e2e.yml up -d $*
