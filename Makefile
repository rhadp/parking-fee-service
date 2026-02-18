# SDV Parking Demo System — Root Makefile
#
# Orchestrates builds, tests, proto generation, linting, infrastructure,
# and container builds across all technology domains.

.PHONY: all build test proto lint clean clean-proto \
        check-tools \
        infra-up infra-down infra-status \
        build-containers

# Default target
all: build

# ─── Tool Checks ────────────────────────────────────────────────────────────

check-tools:
	@./scripts/check-tools.sh

# ─── Build ───────────────────────────────────────────────────────────────────

build: check-tools
	@echo "[make build] Not yet implemented — will build all Rust and Go components."

# ─── Test ────────────────────────────────────────────────────────────────────

test: check-tools
	@echo "[make test] Not yet implemented — will run all unit tests."

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

lint:
	@echo "[make lint] Not yet implemented — will run linters for all components."

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
