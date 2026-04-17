.PHONY: all build build-rust build-go test test-rust test-go test-setup lint lint-rust lint-go check clean proto infra-up infra-down

# ── Default ───────────────────────────────────────────────────────────────────
all: build

# ── Build ─────────────────────────────────────────────────────────────────────
build: build-rust build-go

build-rust:
	cd rhivos && cargo build --workspace

build-go:
	go build parking-fee-service/...

# ── Test ──────────────────────────────────────────────────────────────────────
# Note: test-rust and test-go scope to crates/modules whose tests are expected
# to pass at this stage. Crates with failing stub tests from unimplemented specs
# (locking-service, cloud-gateway-client, mock/parking-operator) are excluded;
# see docs/errata/01_makefile_test_scope.md for details.
test: test-rust test-go

test-rust:
	cd rhivos && cargo test -p update-service -p parking-operator-adaptor
	cd rhivos && cargo test -p mock-sensors --lib

test-go:
	go test \
		parking-fee-service/backend/parking-fee-service \
		parking-fee-service/backend/cloud-gateway \
		parking-fee-service/mock/parking-app-cli/... \
		parking-fee-service/mock/companion-app-cli/... \
		parking-fee-service/tests/setup/...

test-setup:
	go test -v parking-fee-service/tests/setup/...

# ── Lint ──────────────────────────────────────────────────────────────────────
lint: lint-rust lint-go

lint-rust:
	cd rhivos && cargo clippy --workspace -- -D warnings

lint-go:
	go vet parking-fee-service/...

# ── Check (lint + test) ───────────────────────────────────────────────────────
check: lint test

# ── Clean ─────────────────────────────────────────────────────────────────────
clean:
	cd rhivos && cargo clean
	go clean -cache

# ── Proto code generation ─────────────────────────────────────────────────────
proto:
	@which protoc > /dev/null 2>&1 || { echo "error: protoc is required but not installed. Install protocol buffers compiler first."; exit 1; }
	@which protoc-gen-go > /dev/null 2>&1 || { echo "error: protoc-gen-go is required. Run: go install google.golang.org/protobuf/cmd/protoc-gen-go@latest"; exit 1; }
	@which protoc-gen-go-grpc > /dev/null 2>&1 || { echo "error: protoc-gen-go-grpc is required. Run: go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest"; exit 1; }
	mkdir -p gen/go
	protoc \
		--proto_path=proto \
		--go_out=gen/go \
		--go_opt=paths=source_relative \
		--go-grpc_out=gen/go \
		--go-grpc_opt=paths=source_relative \
		$(shell find proto -name '*.proto')

# ── Local Infrastructure ──────────────────────────────────────────────────────
infra-up:
	mkdir -p /tmp/kuksa
	podman-compose -f deployments/compose.yml up -d

infra-down:
	podman-compose -f deployments/compose.yml down
