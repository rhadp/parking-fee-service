# SDV Parking Demo System — Root Makefile
#
# Orchestrates builds, tests, proto generation, linting, infrastructure,
# and container builds across all technology domains.

.PHONY: all build test test-e2e proto lint clean clean-rust clean-go clean-proto \
        check-tools \
        build-rust test-rust lint-rust \
        build-go test-go lint-go \
        infra-up infra-down infra-status \
        build-containers

# ─── Go Module Directories ────────────────────────────────────────────────
GO_BACKEND_DIRS := backend/parking-fee-service backend/cloud-gateway
GO_MOCK_DIRS    := mock/parking-app-cli mock/companion-app-cli
GO_ALL_DIRS     := $(GO_BACKEND_DIRS) $(GO_MOCK_DIRS)

# Default target
all: build

# ─── Tool Checks ────────────────────────────────────────────────────────────

check-tools:
	@./scripts/check-tools.sh

# ─── Build ───────────────────────────────────────────────────────────────────

build: check-tools build-rust build-go

build-rust:
	@echo "[make build] Building Rust workspace..."
	cd rhivos && cargo build --workspace
	@echo "[make build] Rust build complete."

build-go:
	@echo "[make build] Building Go services..."
	@for dir in $(GO_ALL_DIRS); do \
		echo "[make build]   $$dir"; \
		(cd $$dir && go build ./...) || exit 1; \
	done
	@echo "[make build] Go build complete."

# ─── Test ────────────────────────────────────────────────────────────────────

test: check-tools test-rust test-go

test-rust:
	@echo "[make test] Running Rust tests..."
	cd rhivos && cargo test --workspace
	@echo "[make test] Rust tests complete."

test-go:
	@echo "[make test] Running Go tests..."
	@for dir in $(GO_ALL_DIRS); do \
		echo "[make test]   $$dir"; \
		(cd $$dir && go test ./...) || exit 1; \
	done
	@echo "[make test] Go tests complete."

# ─── E2E / Integration Tests ────────────────────────────────────────────────

test-e2e:
	@echo "[make test-e2e] Running cloud connectivity E2E integration tests..."
	@echo "[make test-e2e] Requires 'make infra-up' (Kuksa + Mosquitto)"
	./tests/test_cloud_e2e.sh
	@echo "[make test-e2e] E2E tests complete."

# ─── Proto Generation ───────────────────────────────────────────────────────

PROTO_DIR     := proto
PROTO_FILES   := $(shell find $(PROTO_DIR)/common $(PROTO_DIR)/services -name '*.proto' 2>/dev/null)
GO_GEN_DIR    := $(PROTO_DIR)/gen/go
GO_MODULE     := github.com/rhadp/parking-fee-service/proto/gen/go

proto: check-tools
	@echo "[make proto] Generating Go bindings from .proto files..."
	@mkdir -p $(GO_GEN_DIR)
	protoc \
		--proto_path=$(PROTO_DIR) \
		--go_out=$(GO_GEN_DIR) --go_opt=module=$(GO_MODULE) \
		--go-grpc_out=$(GO_GEN_DIR) --go-grpc_opt=module=$(GO_MODULE) \
		$(PROTO_FILES)
	@echo "[make proto] Go bindings generated under $(GO_GEN_DIR)/"

# ─── Lint ────────────────────────────────────────────────────────────────────

lint: lint-rust lint-go

lint-rust:
	@echo "[make lint] Running Rust linter..."
	cd rhivos && cargo clippy --workspace -- -D warnings
	@echo "[make lint] Rust lint complete."

lint-go:
	@echo "[make lint] Running Go linter..."
	@for dir in $(GO_ALL_DIRS); do \
		echo "[make lint]   $$dir"; \
		(cd $$dir && go vet ./...) || exit 1; \
	done
	@echo "[make lint] Go lint complete."

# ─── Clean ───────────────────────────────────────────────────────────────────

clean: clean-rust clean-go clean-proto
	@echo "[make clean] All build artifacts removed."

clean-rust:
	@echo "[make clean] Cleaning Rust build artifacts..."
	cd rhivos && cargo clean
	@echo "[make clean] Rust clean complete."

clean-go:
	@echo "[make clean] Cleaning Go build artifacts..."
	@for dir in $(GO_ALL_DIRS); do \
		echo "[make clean]   $$dir"; \
		(cd $$dir && go clean ./...) || true; \
	done
	@echo "[make clean] Go clean complete."

clean-proto:
	@echo "[make clean] Removing generated Go proto files..."
	find $(GO_GEN_DIR) -name '*.pb.go' -delete 2>/dev/null || true
	@echo "[make clean] Generated Go proto files removed."

# ─── Infrastructure ─────────────────────────────────────────────────────────

# Detect container runtime: prefer podman, fall back to docker
CONTAINER_RUNTIME := $(shell command -v podman 2>/dev/null || command -v docker 2>/dev/null)
COMPOSE_CMD := $(CONTAINER_RUNTIME) compose
COMPOSE_FILE := infra/compose.yaml

infra-up:
ifndef CONTAINER_RUNTIME
	$(error Container runtime not found. Install podman or docker to manage local infrastructure)
endif
	@echo "[make infra-up] Starting local development infrastructure..."
	$(COMPOSE_CMD) -f $(COMPOSE_FILE) up -d
	@echo "[make infra-up] Infrastructure started."
	@echo "[make infra-up]   Kuksa Databroker: localhost:55555 (gRPC)"
	@echo "[make infra-up]   Mosquitto:        localhost:1883  (MQTT)"

infra-down:
ifndef CONTAINER_RUNTIME
	$(error Container runtime not found. Install podman or docker to manage local infrastructure)
endif
	@echo "[make infra-down] Stopping local development infrastructure..."
	$(COMPOSE_CMD) -f $(COMPOSE_FILE) down
	@echo "[make infra-down] Infrastructure stopped and containers removed."

infra-status:
ifndef CONTAINER_RUNTIME
	$(error Container runtime not found. Install podman or docker to manage local infrastructure)
endif
	@echo "[make infra-status] Infrastructure status:"
	$(COMPOSE_CMD) -f $(COMPOSE_FILE) ps

# ─── Container Builds ───────────────────────────────────────────────────────

# Containerfile definitions: service-name=path/to/Containerfile
CONTAINER_RHIVOS := \
	locking-service=containers/rhivos/locking-service.Containerfile \
	cloud-gateway-client=containers/rhivos/cloud-gateway-client.Containerfile \
	update-service=containers/rhivos/update-service.Containerfile \
	parking-operator-adaptor=containers/rhivos/parking-operator-adaptor.Containerfile

CONTAINER_BACKEND := \
	parking-fee-service=containers/backend/parking-fee-service.Containerfile \
	cloud-gateway=containers/backend/cloud-gateway.Containerfile

CONTAINER_MOCK := \
	mock-sensors=containers/mock/sensors.Containerfile \
	parking-app-cli=containers/mock/parking-app-cli.Containerfile \
	companion-app-cli=containers/mock/companion-app-cli.Containerfile

CONTAINER_ALL := $(CONTAINER_RHIVOS) $(CONTAINER_BACKEND) $(CONTAINER_MOCK)

build-containers:
ifndef CONTAINER_RUNTIME
	$(error Container runtime not found. Install podman or docker to build container images)
endif
	@echo "[make build-containers] Building all container images..."
	@for entry in $(CONTAINER_ALL); do \
		name=$${entry%%=*}; \
		file=$${entry##*=}; \
		echo "[make build-containers]   Building $${name}:latest from $${file}"; \
		$(CONTAINER_RUNTIME) build -t "$${name}:latest" -f "$${file}" . || exit 1; \
	done
	@echo "[make build-containers] All container images built successfully."
