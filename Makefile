# Root Makefile - Build orchestration for all components
# Provides uniform targets across Rust and Go toolchains

.PHONY: build test lint clean infra-up infra-down proto

# Go module directories
GO_BACKEND_MODULES := backend/parking-fee-service backend/cloud-gateway
GO_MOCK_MODULES := mock/parking-app-cli mock/companion-app-cli mock/parking-operator
GO_ALL_MODULES := $(GO_BACKEND_MODULES) $(GO_MOCK_MODULES)

# Compose file path
COMPOSE_FILE := deployments/docker-compose.yml

##@ Build

build: ## Compile all Rust and Go components
	@echo "=== Building Rust workspace ==="
	cd rhivos && cargo build
	@echo ""
	@for mod in $(GO_ALL_MODULES); do \
		echo "=== Building Go module: $$mod ==="; \
		(cd $$mod && go build ./...) || exit 1; \
	done
	@echo ""
	@echo "=== Build complete ==="

##@ Test

test: ## Run all unit tests across Rust and Go components
	@echo "=== Running Rust tests ==="
	cd rhivos && cargo test
	@echo ""
	@for mod in $(GO_ALL_MODULES); do \
		echo "=== Testing Go module: $$mod ==="; \
		(cd $$mod && go test ./...) || exit 1; \
	done
	@echo ""
	@echo "=== All tests complete ==="

##@ Lint

lint: ## Run all linters (clippy for Rust, go vet for Go)
	@echo "=== Linting Rust workspace ==="
	cd rhivos && cargo clippy -- -D warnings
	@echo ""
	@for mod in $(GO_ALL_MODULES); do \
		echo "=== Vetting Go module: $$mod ==="; \
		(cd $$mod && go vet ./...) || exit 1; \
	done
	@echo ""
	@echo "=== Lint complete ==="

##@ Clean

clean: ## Remove all build artifacts
	@echo "=== Cleaning Rust workspace ==="
	cd rhivos && cargo clean
	@for mod in $(GO_ALL_MODULES); do \
		echo "=== Cleaning Go module: $$mod ==="; \
		(cd $$mod && go clean ./...) || true; \
	done
	@echo ""
	@echo "=== Clean complete ==="

##@ Infrastructure

infra-up: ## Start local infrastructure (NATS, Kuksa Databroker)
	@if ! command -v docker >/dev/null 2>&1 && ! command -v podman >/dev/null 2>&1; then \
		echo "ERROR: Docker or Podman is not installed or not in PATH"; \
		echo "Please install Docker (https://docs.docker.com/get-docker/) or Podman."; \
		exit 1; \
	fi
	docker compose -f $(COMPOSE_FILE) up -d
	@echo "Waiting for services to be ready..."
	@echo "Infrastructure started. Kuksa Databroker on :55556"

infra-down: ## Stop and remove local infrastructure containers
	docker compose -f $(COMPOSE_FILE) down --remove-orphans

##@ Proto

proto: ## Validate proto files
	@echo "=== Validating proto files ==="
	@if command -v protoc >/dev/null 2>&1; then \
		find proto -name '*.proto' -exec protoc --proto_path=proto --descriptor_set_out=/dev/null {} + && \
		echo "All proto files are valid"; \
	elif command -v buf >/dev/null 2>&1; then \
		buf lint proto && echo "All proto files are valid"; \
	else \
		echo "WARNING: Neither protoc nor buf found. Skipping proto validation."; \
		echo "Install protoc or buf for proto file validation."; \
	fi
