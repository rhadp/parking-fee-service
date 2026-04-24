.PHONY: build build-rust build-go test test-rust test-go test-setup check clean lint proto infra-up infra-down

# Go modules to build, test, and lint
GO_MODULES = \
	backend/parking-fee-service \
	backend/cloud-gateway \
	mock/parking-app-cli \
	mock/companion-app-cli \
	mock/parking-operator

# Build all components
build: build-rust build-go

build-rust:
	cd rhivos && cargo build --workspace

build-go:
	@for mod in $(GO_MODULES); do \
		echo "Building $$mod..."; \
		cd $$mod && go build ./... && cd $(CURDIR) || exit 1; \
	done

# Lint all code
lint:
	cd rhivos && cargo clippy --workspace -- -D warnings
	@for mod in $(GO_MODULES); do \
		echo "Vetting $$mod..."; \
		cd $$mod && go vet ./... && cd $(CURDIR) || exit 1; \
	done
	cd tests/setup && go vet ./...

# Run all tests
test: test-rust test-go

test-rust:
	cd rhivos && cargo test --workspace

test-go:
	@for mod in $(GO_MODULES); do \
		echo "Testing $$mod..."; \
		cd $$mod && go test ./... && cd $(CURDIR) || exit 1; \
	done

# Run setup verification tests
test-setup:
	cd tests/setup && go test -v ./...

# Quality gate: lint + all tests
check: lint test

# Remove build artifacts
clean:
	cd rhivos && cargo clean
	@for mod in $(GO_MODULES); do \
		cd $$mod && go clean ./... && cd $(CURDIR) || true; \
	done

# Generate Go code from proto definitions
proto:
	@command -v protoc >/dev/null 2>&1 || { echo "Error: protoc is required but not installed. Install protoc and protoc-gen-go." >&2; exit 1; }
	@mkdir -p gen
	protoc --proto_path=proto \
		--go_out=gen --go_opt=module=github.com/rhadp/parking-fee-service/gen \
		--go-grpc_out=gen --go-grpc_opt=module=github.com/rhadp/parking-fee-service/gen \
		$$(find proto -name '*.proto')

# Start local infrastructure (NATS + Kuksa Databroker)
infra-up:
	podman-compose -f deployments/compose.yml up -d

# Stop local infrastructure
infra-down:
	podman-compose -f deployments/compose.yml down
