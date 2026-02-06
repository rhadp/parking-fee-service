# Root Makefile for SDV Parking Demo System
# This Makefile orchestrates builds across all technology stacks

.PHONY: all proto build test clean clean-all
.PHONY: proto-rust proto-kotlin proto-dart proto-go
.PHONY: build-rhivos build-android build-backend build-containers
.PHONY: build-parking-fee-service build-cli build-companion-cli build-parking-cli
.PHONY: test-rhivos test-android test-backend
.PHONY: test-parking-fee-service
.PHONY: infra-up infra-down
.PHONY: dev-up dev-down dev-status dev-test dev-logs
.PHONY: certs certs-clean
.PHONY: git-clean-branches

# Default target
all: proto build

#------------------------------------------------------------------------------
# Protocol Buffer Generation Targets
#------------------------------------------------------------------------------

# Generate all language bindings from Protocol Buffer definitions
proto: proto-rust proto-kotlin proto-dart proto-go
	@echo "All Protocol Buffer bindings generated successfully"

# Generate Rust bindings
proto-rust:
	@echo "Generating Rust Protocol Buffer bindings..."
	@./scripts/generate-proto.sh rust

# Generate Kotlin bindings
proto-kotlin:
	@echo "Generating Kotlin Protocol Buffer bindings..."
	@./scripts/generate-proto.sh kotlin

# Generate Dart bindings
proto-dart:
	@echo "Generating Dart Protocol Buffer bindings..."
	@./scripts/generate-proto.sh dart

# Generate Go bindings
proto-go:
	@echo "Generating Go Protocol Buffer bindings..."
	@./scripts/generate-proto.sh go

#------------------------------------------------------------------------------
# Build Targets
#------------------------------------------------------------------------------

# Build all components (generates proto stubs first)
build: proto build-rhivos build-android build-backend
	@echo "All components built successfully"

# Build Rust services for RHIVOS
build-rhivos:
	@echo "Building RHIVOS Rust services..."
	cd rhivos && cargo build --workspace

# Build Android applications
build-android:
	@echo "Building Android applications..."
	cd android/parking-app && gradle build
	cd android/companion_app && flutter build apk

# Build Go backend services
build-backend:
	@echo "Building Go backend services..."
	cd backend && go build ./...

# Build parking-fee-service specifically
build-parking-fee-service:
	@echo "Building parking-fee-service..."
	cd backend && go build -o bin/parking-fee-service ./parking-fee-service/cmd/server

# Build CLI simulators
build-cli: build-companion-cli build-parking-cli
	@echo "CLI simulators built successfully"

# Build companion-cli
build-companion-cli:
	@echo "Building companion-cli..."
	cd backend && go build -o bin/companion-cli ./companion-cli/cmd/companion-cli

# Build parking-cli
build-parking-cli:
	@echo "Building parking-cli..."
	cd backend && go build -o bin/parking-cli ./parking-cli/cmd/parking-cli

# Build all container images
build-containers:
	@echo "Building container images..."
	@GIT_COMMIT=$$(git rev-parse --short HEAD 2>/dev/null || echo "unknown") && \
	GIT_TAG=$$(git describe --tags --always 2>/dev/null || echo "dev") && \
	echo "Tagging images with: $${GIT_TAG}-$${GIT_COMMIT}" && \
	podman build -t sdv-locking-service:$${GIT_TAG}-$${GIT_COMMIT} -f containers/rhivos/Containerfile.locking-service . && \
	podman build -t sdv-update-service:$${GIT_TAG}-$${GIT_COMMIT} -f containers/rhivos/Containerfile.update-service . && \
	podman build -t sdv-parking-operator-adaptor:$${GIT_TAG}-$${GIT_COMMIT} -f containers/rhivos/Containerfile.parking-operator-adaptor . && \
	podman build -t sdv-cloud-gateway-client:$${GIT_TAG}-$${GIT_COMMIT} -f containers/rhivos/Containerfile.cloud-gateway-client . && \
	podman build -t sdv-parking-fee-service:$${GIT_TAG}-$${GIT_COMMIT} -f containers/backend/Containerfile.parking-fee-service . && \
	podman build -t sdv-cloud-gateway:$${GIT_TAG}-$${GIT_COMMIT} -f containers/backend/Containerfile.cloud-gateway .

#------------------------------------------------------------------------------
# Test Targets
#------------------------------------------------------------------------------

# Run all tests
test: test-rhivos test-android test-backend
	@echo "All tests completed"

# Run Rust tests
test-rhivos:
	@echo "Running RHIVOS Rust tests..."
	cd rhivos && cargo test --workspace

# Run Android tests
test-android:
	@echo "Running Android tests..."
	@if [ -n "$$ANDROID_HOME" ] || [ -f android/parking-app/local.properties ]; then \
		cd android/parking-app && gradle test; \
	else \
		echo "Skipping parking-app tests: ANDROID_HOME not set and no local.properties found"; \
	fi
	@if command -v flutter >/dev/null 2>&1 && [ -d android/companion_app/test ]; then \
		cd android/companion_app && flutter test; \
	else \
		echo "Skipping companion_app tests: Flutter not available or no test directory"; \
	fi

# Run Go backend tests
test-backend:
	@echo "Running Go backend tests..."
	cd backend && go test ./...

# Run parking-fee-service tests specifically
test-parking-fee-service:
	@echo "Running parking-fee-service tests..."
	cd backend && go test -v ./parking-fee-service/...

#------------------------------------------------------------------------------
# TLS Certificate Targets
#------------------------------------------------------------------------------

# Generate development TLS certificates
certs:
	@echo "Generating development TLS certificates..."
	@./scripts/generate-certs.sh

# Clean TLS certificates
certs-clean:
	@echo "Cleaning TLS certificates..."
	@./scripts/generate-certs.sh clean

#------------------------------------------------------------------------------
# Local Infrastructure Targets
#------------------------------------------------------------------------------

# Start local development infrastructure
infra-up:
	@echo "Starting local development infrastructure..."
	cd infra/compose && podman-compose up -d
	@echo "Waiting for services to be healthy..."
	@sleep 5
	@echo "Local infrastructure is running"
	@echo "  - MQTT Broker: localhost:1883 (TLS: 8883)"
	@echo "  - Kuksa Databroker: localhost:55556"
	@echo "  - Mock Parking Operator: localhost:8080"

# Stop local development infrastructure
infra-down:
	@echo "Stopping local development infrastructure..."
	cd infra/compose && podman-compose down
	@echo "Local infrastructure stopped"

#------------------------------------------------------------------------------
# Local Development Environment Targets
#------------------------------------------------------------------------------

# Start complete local development environment (infrastructure + RHIVOS services)
dev-up:
	@./scripts/local-dev.sh start

# Stop complete local development environment
dev-down:
	@./scripts/local-dev.sh stop

# Check health status of all services
dev-status:
	@./scripts/local-dev.sh status

# Run integration tests against local environment
dev-test:
	@./scripts/local-dev.sh test

# Tail logs from all services
dev-logs:
	@./scripts/local-dev.sh logs

#------------------------------------------------------------------------------
# Clean Targets
#------------------------------------------------------------------------------

# Clean all build artifacts (preserves generated proto stubs)
clean:
	@echo "Cleaning all build artifacts..."
	@# Clean Rust artifacts
	cd rhivos && cargo clean 2>/dev/null || true
	@# Clean Android artifacts
	cd android/parking-app && gradle clean 2>/dev/null || true
	cd android/companion_app && flutter clean 2>/dev/null || true
	@# Clean Go artifacts
	cd backend && go clean ./... 2>/dev/null || true
	rm -rf backend/bin 2>/dev/null || true
	@echo "Clean complete"

# Clean everything including generated proto stubs
clean-all: clean
	@echo "Cleaning generated proto stubs..."
	rm -rf rhivos/shared/src/proto 2>/dev/null || true
	rm -rf android/parking-app/app/src/main/java/sdv/proto 2>/dev/null || true
	rm -rf android/companion_app/lib/generated 2>/dev/null || true
	rm -rf android/companion_app/lib/proto 2>/dev/null || true
	rm -rf backend/gen 2>/dev/null || true
	@echo "Full clean complete"

#------------------------------------------------------------------------------
# Git Maintenance Targets
#------------------------------------------------------------------------------

# Delete all local branches except main, develop, and release
git-clean-branches:
	@echo "Cleaning up local feature branches..."
	@git checkout develop 2>/dev/null || git checkout main
	@git branch | grep -vE '^\*|^\s*(main|develop|release)$$' | xargs -r git branch -D
	@echo "Branch cleanup complete. Remaining branches:"
	@git branch

#------------------------------------------------------------------------------
# Help Target
#------------------------------------------------------------------------------

help:
	@echo "SDV Parking Demo System - Build Targets"
	@echo ""
	@echo "Main targets:"
	@echo "  all              - Generate protos and build all components (default)"
	@echo "  proto            - Generate all Protocol Buffer bindings"
	@echo "  build            - Generate protos and build all components"
	@echo "  test             - Run all tests"
	@echo "  clean            - Clean build artifacts (preserves proto stubs)"
	@echo "  clean-all        - Clean everything including generated proto stubs"
	@echo ""
	@echo "Proto generation targets:"
	@echo "  proto-rust       - Generate Rust bindings"
	@echo "  proto-kotlin     - Generate Kotlin bindings"
	@echo "  proto-dart       - Generate Dart bindings"
	@echo "  proto-go         - Generate Go bindings"
	@echo ""
	@echo "Build targets:"
	@echo "  build-rhivos             - Build Rust services for RHIVOS"
	@echo "  build-android            - Build Android applications"
	@echo "  build-backend            - Build Go backend services"
	@echo "  build-parking-fee-service - Build parking-fee-service specifically"
	@echo "  build-cli                - Build CLI simulators (companion-cli, parking-cli)"
	@echo "  build-companion-cli      - Build companion-cli for remote vehicle control"
	@echo "  build-parking-cli        - Build parking-cli for parking session management"
	@echo "  build-containers         - Build all container images"
	@echo ""
	@echo "Test targets:"
	@echo "  test-rhivos              - Run Rust tests"
	@echo "  test-android             - Run Android tests"
	@echo "  test-backend             - Run Go backend tests"
	@echo "  test-parking-fee-service - Run parking-fee-service tests"
	@echo ""
	@echo "Certificate targets:"
	@echo "  certs            - Generate development TLS certificates"
	@echo "  certs-clean      - Clean TLS certificates"
	@echo ""
	@echo "Infrastructure targets:"
	@echo "  infra-up         - Start local development infrastructure"
	@echo "  infra-down       - Stop local development infrastructure"
	@echo ""
	@echo "Local development environment:"
	@echo "  dev-up           - Start complete local dev environment (infrastructure + RHIVOS)"
	@echo "  dev-down         - Stop complete local dev environment"
	@echo "  dev-status       - Check health status of all services"
	@echo "  dev-test         - Run integration tests against local environment"
	@echo "  dev-logs         - Tail logs from all services"
	@echo ""
	@echo "Git maintenance targets:"
	@echo "  git-clean-branches - Delete all local branches except main, develop, release"
