# SDV Parking Demo System — Root Makefile
#
# Orchestrates builds, tests, proto generation, linting, infrastructure,
# and container builds across all technology domains.

.PHONY: all build test proto lint clean \
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

proto: check-tools
	@echo "[make proto] Not yet implemented — will generate language bindings from .proto files."

# ─── Lint ────────────────────────────────────────────────────────────────────

lint:
	@echo "[make lint] Not yet implemented — will run linters for all components."

# ─── Clean ───────────────────────────────────────────────────────────────────

clean:
	@echo "[make clean] Not yet implemented — will remove all build artifacts."

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
