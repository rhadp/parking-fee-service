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
	podman compose -f deployments/compose.yml up -d

# Stop local infrastructure
infra-down:
	podman compose -f deployments/compose.yml down

# Validate proto files
proto:
	@find proto -name '*.proto' -exec grep -l 'syntax = "proto3"' {} \;
