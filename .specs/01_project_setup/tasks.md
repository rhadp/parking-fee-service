# Implementation Plan: Project Setup

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan creates the foundational monorepo structure, skeleton implementations, build system, local infrastructure, and test runner configuration. Because this is a project setup spec (not application logic), the "failing tests" in task group 1 are shell-based verification scripts that check for directory existence, build success, and infrastructure availability. Subsequent groups create the artifacts that make those tests pass.

## Test Commands

- Spec tests: `bash tests/setup/run_all.sh`
- Rust tests: `cd rhivos && cargo test`
- Go backend tests: `cd backend/parking-fee-service && go test ./... && cd ../cloud-gateway && go test ./...`
- Go mock tests: `cd mock/parking-app-cli && go test ./... && cd ../companion-app-cli && go test ./... && cd ../parking-operator && go test ./...`
- All tests: `make test`
- Linter: `make lint`

## Tasks

- [ ] 1. Write failing spec tests
  - [ ] 1.1 Create test directory structure
    - Create `tests/setup/` directory at repo root
    - Create `tests/setup/run_all.sh` as the test runner entry point
    - _Test Spec: TS-01-1 through TS-01-26, TS-01-P1 through TS-01-P7, TS-01-E1 through TS-01-E6_

  - [ ] 1.2 Implement directory structure tests
    - Create `tests/setup/test_directories.sh`
    - Test for existence of all required top-level and nested directories (TS-01-1 through TS-01-4)
    - Test placeholder directories contain only README.md (TS-01-E1, TS-01-E2)
    - Test directory completeness property (TS-01-P1)
    - _Test Spec: TS-01-1, TS-01-2, TS-01-3, TS-01-4, TS-01-E1, TS-01-E2, TS-01-P1_

  - [ ] 1.3 Implement build and workspace tests
    - Create `tests/setup/test_build.sh`
    - Test Rust workspace Cargo.toml exists and lists members (TS-01-5)
    - Test Rust workspace builds and tests pass (TS-01-6, TS-01-7)
    - Test Go workspace files exist with correct modules (TS-01-8, TS-01-9)
    - Test Go modules build and tests pass (TS-01-10, TS-01-11)
    - Test skeleton binaries exit with code 0 (TS-01-12, TS-01-13)
    - Test each component has at least one passing test (TS-01-14)
    - Test build determinism property (TS-01-P2)
    - Test discoverability property (TS-01-P3)
    - Test skeleton exit behavior property (TS-01-P4)
    - Test skeleton binary without config (TS-01-E3)
    - Test make build failure reporting (TS-01-E4)
    - _Test Spec: TS-01-5 through TS-01-14, TS-01-P2, TS-01-P3, TS-01-P4, TS-01-E3, TS-01-E4_

  - [ ] 1.4 Implement Makefile and proto tests
    - Create `tests/setup/test_makefile.sh`
    - Test root Makefile has all required targets (TS-01-16)
    - Test make build/test/clean succeed (TS-01-17, TS-01-18, TS-01-19)
    - Test make test runs all component tests (TS-01-25)
    - Test proto directory contains valid proto3 file (TS-01-15)
    - Test proto validity property (TS-01-P6)
    - _Test Spec: TS-01-15, TS-01-16, TS-01-17, TS-01-18, TS-01-19, TS-01-25, TS-01-P6_

  - [ ] 1.5 Implement infrastructure and mock CLI tests
    - Create `tests/setup/test_infra.sh`
    - Test compose file defines NATS and Kuksa services (TS-01-20)
    - Test infrastructure starts and services are reachable (TS-01-21)
    - Test infrastructure stops cleanly (TS-01-22)
    - Test infrastructure lifecycle property (TS-01-P5)
    - Create `tests/setup/test_mock_cli.sh`
    - Test mock CLI apps build (TS-01-23)
    - Test mock CLI apps print usage (TS-01-24)
    - Test mock sensors crate builds (TS-01-26)
    - Test mock CLI usage output property (TS-01-P7)
    - Test unknown subcommand handling (TS-01-E5)
    - Test no-tests-graceful handling (TS-01-E6)
    - _Test Spec: TS-01-20 through TS-01-26, TS-01-P5, TS-01-P7, TS-01-E5, TS-01-E6_

  - [ ] 1.V Verify task group 1
    - [ ] All test scripts exist and are syntactically valid: `bash -n tests/setup/*.sh`
    - [ ] All spec tests FAIL (red) -- no implementation yet
    - [ ] Test runner entry point works: `bash tests/setup/run_all.sh` (expected: all fail)

- [ ] 2. Create repo structure and skeleton projects
  - [ ] 2.1 Create top-level directory structure
    - Create directories: `rhivos/`, `backend/`, `android/`, `mobile/`, `mock/`, `proto/`, `deployments/`
    - Create `android/README.md` with placeholder text
    - Create `mobile/README.md` with placeholder text
    - _Requirements: 01-REQ-1.1, 01-REQ-1.E1, 01-REQ-1.E2_

  - [ ] 2.2 Create Rust workspace and skeleton crates
    - Create `rhivos/Cargo.toml` with workspace members
    - Create skeleton `main.rs` for each binary crate: locking-service, cloud-gateway-client, update-service, parking-operator-adaptor
    - Create skeleton `lib.rs` for mock-sensors with modules: location, speed, door
    - Each crate gets its own `Cargo.toml` with appropriate dependencies
    - Each skeleton prints a startup message and includes one passing test
    - _Requirements: 01-REQ-1.2, 01-REQ-2.1, 01-REQ-2.2, 01-REQ-2.3, 01-REQ-4.1, 01-REQ-4.3, 01-REQ-10.1, 01-REQ-10.2_

  - [ ] 2.3 Create Go backend workspace and skeleton modules
    - Create `backend/go.work` listing both modules
    - Create `backend/parking-fee-service/` with go.mod and skeleton main.go
    - Create `backend/cloud-gateway/` with go.mod and skeleton main.go
    - Each skeleton prints a startup message and includes one passing test
    - _Requirements: 01-REQ-1.3, 01-REQ-3.1, 01-REQ-3.3, 01-REQ-3.4, 01-REQ-4.2, 01-REQ-4.3_

  - [ ] 2.4 Create mock CLI apps and Go workspace
    - Create `mock/go.work` listing all mock app modules
    - Create `mock/parking-app-cli/` with go.mod, main.go (prints usage, handles subcommands)
    - Create `mock/companion-app-cli/` with go.mod, main.go (prints usage, handles subcommands)
    - Create `mock/parking-operator/` with go.mod, main.go (prints usage, handles subcommands)
    - Each app prints usage when run without args, errors on unknown subcommands
    - _Requirements: 01-REQ-1.4, 01-REQ-3.2, 01-REQ-8.1, 01-REQ-8.2, 01-REQ-8.3, 01-REQ-8.4, 01-REQ-8.E1_

  - [ ] 2.5 Create shared proto definitions
    - Create `proto/parking/v1/common.proto` with proto3 syntax and package declaration
    - Include placeholder message types (Location, VehicleIdentifier)
    - _Requirements: 01-REQ-5.1, 01-REQ-5.2_

  - [ ] 2.V Verify task group 2
    - [ ] Directory structure tests pass: `bash tests/setup/test_directories.sh`
    - [ ] Rust workspace builds: `cd rhivos && cargo build`
    - [ ] Rust tests pass: `cd rhivos && cargo test`
    - [ ] Go backend builds: `cd backend/parking-fee-service && go build ./... && cd ../cloud-gateway && go build ./...`
    - [ ] Go backend tests pass: `cd backend/parking-fee-service && go test ./... && cd ../cloud-gateway && go test ./...`
    - [ ] Mock CLI apps build and run: `cd mock/parking-app-cli && go run .`
    - [ ] Proto file is valid proto3
    - [ ] All build tests pass: `bash tests/setup/test_build.sh`
    - [ ] Requirements 01-REQ-1.* through 01-REQ-5.*, 01-REQ-8.*, 01-REQ-10.* acceptance criteria met

- [ ] 3. Setup build system (root Makefile + per-component builds)
  - [ ] 3.1 Create root Makefile
    - Define targets: `build`, `test`, `lint`, `clean`, `infra-up`, `infra-down`, `proto`
    - `build` target: runs `cargo build` in rhivos/ + `go build ./...` in each Go module
    - `test` target: runs `cargo test` in rhivos/ + `go test ./...` in each Go module
    - `lint` target: runs `cargo clippy` in rhivos/ + `go vet ./...` in each Go module
    - `clean` target: runs `cargo clean` in rhivos/ + `go clean` in each Go module
    - `infra-up`/`infra-down`: delegates to docker compose
    - _Requirements: 01-REQ-6.1, 01-REQ-6.2, 01-REQ-6.3, 01-REQ-6.4, 01-REQ-6.E1_

  - [ ] 3.2 Verify Makefile targets
    - Run `make build` and confirm exit code 0
    - Run `make test` and confirm all tests pass
    - Run `make clean` and confirm build artifacts removed
    - Run `make lint` and confirm no warnings
    - _Requirements: 01-REQ-6.2, 01-REQ-6.3, 01-REQ-6.4, 01-REQ-9.3_

  - [ ] 3.V Verify task group 3
    - [ ] Makefile tests pass: `bash tests/setup/test_makefile.sh`
    - [ ] `make build` succeeds: `make build`
    - [ ] `make test` succeeds: `make test`
    - [ ] `make lint` succeeds: `make lint`
    - [ ] `make clean` removes artifacts: `make clean && test ! -d rhivos/target`
    - [ ] All existing tests still pass: `bash tests/setup/run_all.sh` (excluding infra tests)
    - [ ] Requirements 01-REQ-6.*, 01-REQ-9.3 acceptance criteria met

- [ ] 4. Setup local infrastructure (docker-compose)
  - [ ] 4.1 Create docker-compose.yml
    - Create `deployments/docker-compose.yml`
    - Define NATS service: image `nats:latest`, ports 4222 and 8222
    - Define Kuksa Databroker service: image `ghcr.io/eclipse-kuksa/kuksa-databroker:master`, port 55555
    - Add health check configurations for both services
    - _Requirements: 01-REQ-7.1, 01-REQ-7.2, 01-REQ-7.3_

  - [ ] 4.2 Wire Makefile infra targets
    - Ensure `make infra-up` runs `docker compose -f deployments/docker-compose.yml up -d`
    - Ensure `make infra-down` runs `docker compose -f deployments/docker-compose.yml down`
    - Add wait-for-healthy logic or timeout to infra-up
    - _Requirements: 01-REQ-7.2, 01-REQ-7.3, 01-REQ-7.E1, 01-REQ-7.E2_

  - [ ] 4.V Verify task group 4
    - [ ] Infrastructure tests pass: `bash tests/setup/test_infra.sh`
    - [ ] `make infra-up` starts containers: `make infra-up && docker compose -f deployments/docker-compose.yml ps`
    - [ ] NATS reachable on port 4222
    - [ ] Kuksa Databroker reachable on port 55555
    - [ ] `make infra-down` stops containers cleanly: `make infra-down`
    - [ ] All existing tests still pass: `bash tests/setup/run_all.sh`
    - [ ] Requirements 01-REQ-7.* acceptance criteria met

- [ ] 5. Configure test runners
  - [ ] 5.1 Verify Rust test runner configuration
    - Confirm `cargo test` in rhivos/ discovers and runs all workspace tests
    - Confirm test output format is usable (names, pass/fail counts)
    - _Requirements: 01-REQ-9.1_

  - [ ] 5.2 Verify Go test runner configuration
    - Confirm `go test ./...` in each Go module discovers and runs all tests
    - Confirm test output includes pass/fail status
    - _Requirements: 01-REQ-9.2_

  - [ ] 5.3 Verify make test integration
    - Confirm `make test` runs both Rust and Go tests
    - Confirm combined output shows results from all toolchains
    - _Requirements: 01-REQ-9.3_

  - [ ] 5.V Verify task group 5
    - [ ] Mock CLI tests pass: `bash tests/setup/test_mock_cli.sh`
    - [ ] All spec tests pass: `bash tests/setup/run_all.sh`
    - [ ] `make test` succeeds: `make test`
    - [ ] `make lint` succeeds: `make lint`
    - [ ] Requirements 01-REQ-9.* acceptance criteria met

- [ ] 6. Checkpoint - Project Setup Complete
  - All spec tests pass: `bash tests/setup/run_all.sh`
  - All component tests pass: `make test`
  - All linters pass: `make lint`
  - Infrastructure lifecycle works: `make infra-up && make infra-down`
  - Ensure all tests pass, ask the user if questions arise.

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 01-REQ-1.1 | TS-01-1 | 2.1 | `tests/setup/test_directories.sh` |
| 01-REQ-1.2 | TS-01-2 | 2.2 | `tests/setup/test_directories.sh` |
| 01-REQ-1.3 | TS-01-3 | 2.3 | `tests/setup/test_directories.sh` |
| 01-REQ-1.4 | TS-01-4 | 2.4 | `tests/setup/test_directories.sh` |
| 01-REQ-1.E1 | TS-01-E1 | 2.1 | `tests/setup/test_directories.sh` |
| 01-REQ-1.E2 | TS-01-E2 | 2.1 | `tests/setup/test_directories.sh` |
| 01-REQ-2.1 | TS-01-5 | 2.2 | `tests/setup/test_build.sh` |
| 01-REQ-2.2 | TS-01-6 | 2.2 | `tests/setup/test_build.sh` |
| 01-REQ-2.3 | TS-01-7 | 2.2 | `tests/setup/test_build.sh` |
| 01-REQ-3.1 | TS-01-8 | 2.3 | `tests/setup/test_build.sh` |
| 01-REQ-3.2 | TS-01-9 | 2.4 | `tests/setup/test_build.sh` |
| 01-REQ-3.3 | TS-01-10 | 2.3, 2.4 | `tests/setup/test_build.sh` |
| 01-REQ-3.4 | TS-01-11 | 2.3, 2.4 | `tests/setup/test_build.sh` |
| 01-REQ-4.1 | TS-01-12 | 2.2 | `tests/setup/test_build.sh` |
| 01-REQ-4.2 | TS-01-13 | 2.3 | `tests/setup/test_build.sh` |
| 01-REQ-4.3 | TS-01-14 | 2.2, 2.3, 2.4 | `tests/setup/test_build.sh` |
| 01-REQ-4.E1 | TS-01-E3 | 2.2, 2.3 | `tests/setup/test_build.sh` |
| 01-REQ-5.1 | TS-01-15 | 2.5 | `tests/setup/test_makefile.sh` |
| 01-REQ-5.2 | TS-01-15 | 2.5 | `tests/setup/test_makefile.sh` |
| 01-REQ-6.1 | TS-01-16 | 3.1 | `tests/setup/test_makefile.sh` |
| 01-REQ-6.2 | TS-01-17 | 3.1 | `tests/setup/test_makefile.sh` |
| 01-REQ-6.3 | TS-01-18 | 3.1 | `tests/setup/test_makefile.sh` |
| 01-REQ-6.4 | TS-01-19 | 3.1 | `tests/setup/test_makefile.sh` |
| 01-REQ-6.E1 | TS-01-E4 | 3.1 | `tests/setup/test_build.sh` |
| 01-REQ-7.1 | TS-01-20 | 4.1 | `tests/setup/test_infra.sh` |
| 01-REQ-7.2 | TS-01-21 | 4.1, 4.2 | `tests/setup/test_infra.sh` |
| 01-REQ-7.3 | TS-01-22 | 4.1, 4.2 | `tests/setup/test_infra.sh` |
| 01-REQ-7.E1 | TS-01-20 | 4.1 | `tests/setup/test_infra.sh` |
| 01-REQ-7.E2 | TS-01-21 | 4.2 | `tests/setup/test_infra.sh` |
| 01-REQ-8.1 | TS-01-23 | 2.4 | `tests/setup/test_mock_cli.sh` |
| 01-REQ-8.2 | TS-01-23 | 2.4 | `tests/setup/test_mock_cli.sh` |
| 01-REQ-8.3 | TS-01-23 | 2.4 | `tests/setup/test_mock_cli.sh` |
| 01-REQ-8.4 | TS-01-24 | 2.4 | `tests/setup/test_mock_cli.sh` |
| 01-REQ-8.E1 | TS-01-E5 | 2.4 | `tests/setup/test_mock_cli.sh` |
| 01-REQ-9.1 | TS-01-7 | 5.1 | `tests/setup/test_build.sh` |
| 01-REQ-9.2 | TS-01-11 | 5.2 | `tests/setup/test_build.sh` |
| 01-REQ-9.3 | TS-01-25 | 5.3 | `tests/setup/test_makefile.sh` |
| 01-REQ-9.E1 | TS-01-E6 | 5.2 | `tests/setup/test_mock_cli.sh` |
| 01-REQ-10.1 | TS-01-26 | 2.2 | `tests/setup/test_mock_cli.sh` |
| 01-REQ-10.2 | TS-01-26 | 2.2 | `tests/setup/test_mock_cli.sh` |
| Property 1 | TS-01-P1 | 2.1, 2.2, 2.3, 2.4 | `tests/setup/test_directories.sh` |
| Property 2 | TS-01-P2 | 3.1 | `tests/setup/test_build.sh` |
| Property 3 | TS-01-P3 | 2.2, 2.3, 2.4 | `tests/setup/test_build.sh` |
| Property 4 | TS-01-P4 | 2.2, 2.3 | `tests/setup/test_build.sh` |
| Property 5 | TS-01-P5 | 4.1, 4.2 | `tests/setup/test_infra.sh` |
| Property 6 | TS-01-P6 | 2.5 | `tests/setup/test_makefile.sh` |
| Property 7 | TS-01-P7 | 2.4 | `tests/setup/test_mock_cli.sh` |

## Notes

- This spec is infrastructure-heavy: tests are shell scripts rather than unit tests in a framework.
- Infrastructure tests (task group 4) require Docker or Podman to be installed and running.
- Proto validation tests require `protoc` to be installed.
- The `android/` and `mobile/` directories are intentionally minimal placeholders -- they will be populated by separate specs.
- Mock CLI apps use Go's `flag` or `cobra` package for subcommand handling. The specific library choice is an implementation decision.
- All skeleton binaries should be stateless -- no config files, databases, or network connections needed for basic execution.
