# SDV Parking Demo System — Root Makefile
#
# Orchestrates builds, tests, proto generation, linting, infrastructure,
# and container builds across all technology domains.

.PHONY: all build test proto lint clean clean-proto \
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

clean:
	@echo "[make clean] Not yet implemented — will remove all build artifacts."
	@echo "[make clean] Note: 'make clean-proto' is available to remove generated Go proto files."

clean-proto:
	@echo "[make clean-proto] Removing generated Go proto files..."
	find $(GO_GEN_DIR) -name '*.pb.go' -delete
	@echo "[make clean-proto] Done."

# ─── Infrastructure ─────────────────────────────────────────────────────────

infra-up:
	@echo "[make infra-up] Not yet implemented — will start local development infrastructure."

infra-down:
	@echo "[make infra-down] Not yet implemented — will stop local development infrastructure."

infra-status:
	@echo "[make infra-status] Not yet implemented — will show infrastructure status."

# ─── Container Builds ───────────────────────────────────────────────────────

build-containers:
	@echo "[make build-containers] Not yet implemented — will build all container images."
