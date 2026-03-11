.PHONY: build test lint clean infra-up infra-down proto

# Go module directories
GO_BACKEND_MODULES := backend/parking-fee-service backend/cloud-gateway
GO_MOCK_MODULES := mock/parking-app-cli mock/companion-app-cli mock/parking-operator
GO_ALL_MODULES := $(GO_BACKEND_MODULES) $(GO_MOCK_MODULES)

# Build all Rust and Go components
build:
	cd rhivos && cargo build
	@for mod in $(GO_ALL_MODULES); do \
		echo "Building $$mod..."; \
		(cd $$mod && go build ./...) || exit 1; \
	done

# Run all unit tests across Rust and Go components
test:
	cd rhivos && cargo test
	@for mod in $(GO_ALL_MODULES); do \
		echo "Testing $$mod..."; \
		(cd $$mod && go test ./...) || exit 1; \
	done

# Run linters for all components
lint:
	cd rhivos && cargo clippy -- -D warnings
	@for mod in $(GO_ALL_MODULES); do \
		echo "Linting $$mod..."; \
		(cd $$mod && go vet ./...) || exit 1; \
	done

# Remove all build artifacts
clean:
	cd rhivos && cargo clean
	@for mod in $(GO_ALL_MODULES); do \
		(cd $$mod && go clean) || true; \
	done

# Start local infrastructure (NATS + Kuksa Databroker)
infra-up:
	@command -v podman >/dev/null 2>&1 || { echo "Error: podman is not installed or not in PATH"; exit 1; }
	podman compose -f deployments/compose.yml up -d
	@echo "Waiting for services to become healthy (up to 30s)..."
	@elapsed=0; \
	while [ $$elapsed -lt 30 ]; do \
		if nc -z localhost 4222 2>/dev/null && nc -z localhost 55556 2>/dev/null; then \
			echo "All services are reachable."; \
			exit 0; \
		fi; \
		sleep 1; \
		elapsed=$$((elapsed + 1)); \
	done; \
	echo "Error: services did not become reachable within 30 seconds"; exit 1

# Stop local infrastructure
infra-down:
	podman compose -f deployments/compose.yml down

# Validate proto files
proto:
	@find proto -name '*.proto' -exec grep -l 'syntax = "proto3"' {} \;
