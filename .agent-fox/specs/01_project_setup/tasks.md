<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md
- Follow git-flow: feature branch from develop -> implement -> test -> merge to develop
- Update checkbox states as you go
-->

# Implementation Plan: Project Setup

## Overview

This task list implements the project setup specification through an incremental, test-first approach. Task Group 1 writes failing verification tests from the test spec, then subsequent groups build out the monorepo structure, skeleton binaries, proto definitions, infrastructure configuration, and Makefile targets until all tests pass. Each group ends with a verification step to confirm progress.

## Test Commands

| Command | Purpose |
|---------|---------|
| `make test` | Run all Rust and Go tests |
| `make test-setup` | Run setup verification tests |
| `make build` | Build all components |
| `make check` | Run lint + all tests |
| `cargo test --workspace` | Run Rust tests only (from rhivos/) |
| `go test ./...` | Run Go tests only (from repo root) |
| `make infra-up` | Start local infrastructure |
| `make infra-down` | Stop local infrastructure |
| `make proto` | Generate Go code from proto files |

## Tasks

- [x] 1. Write failing spec tests
  _Write the setup verification tests based on test_spec.md. These tests will initially fail because the project structure does not yet exist._

  - [x] 1.1 Create `tests/setup/` Go module with `go.mod`
    _Test Spec: TS-01-6_
    _Requirements: 01-REQ-1.8, 01-REQ-9.1_

  - [x] 1.2 Write directory structure tests: verify `rhivos/`, `backend/`, `android/`, `mobile/`, `mock/`, `proto/`, `deployments/` directories and their required subdirectories exist
    _Test Spec: TS-01-1, TS-01-2, TS-01-3, TS-01-4, TS-01-5_
    _Requirements: 01-REQ-1.1, 01-REQ-1.2, 01-REQ-1.3, 01-REQ-1.4, 01-REQ-1.5, 01-REQ-1.6, 01-REQ-1.7_

  - [x] 1.3 Write Rust workspace validation tests: verify `Cargo.toml` workspace configuration, crate structure, and mock-sensors binary targets
    _Test Spec: TS-01-7, TS-01-8_
    _Requirements: 01-REQ-2.1, 01-REQ-2.2, 01-REQ-2.3_

  - [x] 1.4 Write Go workspace validation tests: verify `go.work` references, `go.mod` files, and `main.go` presence
    _Test Spec: TS-01-10, TS-01-11_
    _Requirements: 01-REQ-3.1, 01-REQ-3.2, 01-REQ-3.3_

  - [x] 1.5 Write Makefile target tests: verify all required Make targets exist
    _Test Spec: TS-01-18_
    _Test Spec: TS-01-P4_
    _Requirements: 01-REQ-6.1_

  - [x] 1.6 Write infrastructure config tests: verify `compose.yml`, NATS config, and VSS overlay file contents
    _Test Spec: TS-01-23, TS-01-24, TS-01-25_
    _Test Spec: TS-01-P3_
    _Requirements: 01-REQ-7.1, 01-REQ-7.2, 01-REQ-7.3_

  - [x] 1.V Verify: run `go test -v ./...` in `tests/setup/` and confirm all tests fail (expected — structure does not exist yet)
    _Verification step for Task Group 1_

- [x] 2. Create monorepo directory structure
  _Create all directories, placeholder files, and workspace configurations._

  - [x] 2.1 Create `rhivos/` Cargo workspace: root `Cargo.toml` with workspace members, each member with `Cargo.toml` and `src/main.rs`
    _Test Spec: TS-01-1, TS-01-7_
    _Requirements: 01-REQ-1.1, 01-REQ-2.1, 01-REQ-2.2_

  - [x] 2.2 Create `rhivos/mock-sensors/` with three binary targets (`location-sensor`, `speed-sensor`, `door-sensor`) sharing common crate structure
    _Test Spec: TS-01-8_
    _Requirements: 01-REQ-2.3_

  - [x] 2.3 Create `backend/` Go modules: `parking-fee-service/` and `cloud-gateway/`, each with `go.mod`, `main.go`, and `main_test.go`
    _Test Spec: TS-01-2, TS-01-11, TS-01-27_
    _Requirements: 01-REQ-1.2, 01-REQ-3.2, 01-REQ-3.3, 01-REQ-8.2_

  - [x] 2.4 Create `mock/` Go modules: `parking-app-cli/`, `companion-app-cli/`, `parking-operator/`, each with `go.mod`, `main.go`, and `main_test.go`
    _Test Spec: TS-01-4, TS-01-11, TS-01-27_
    _Requirements: 01-REQ-1.5, 01-REQ-3.2, 01-REQ-3.3, 01-REQ-8.2_

  - [x] 2.5 Create `android/README.md` and `mobile/README.md` placeholder directories
    _Test Spec: TS-01-3_
    _Requirements: 01-REQ-1.3, 01-REQ-1.4_

  - [x] 2.6 Create `go.work` file referencing all Go modules
    _Test Spec: TS-01-10_
    _Requirements: 01-REQ-3.1_

  - [x] 2.V Verify: run `go test -v ./...` in `tests/setup/` and confirm directory structure and workspace tests pass
    _Verification step for Task Group 2_

- [x] 3. Implement skeleton binaries and placeholder tests
  _Add version printing, flag handling, and placeholder tests to all skeletons._

  - [x] 3.1 Implement Rust skeleton `main.rs` for each crate: print `"{component-name} v0.1.0"` to stdout, exit 0; handle unknown flags with usage message to stderr, exit non-zero
    _Test Spec: TS-01-13, TS-01-15_
    _Requirements: 01-REQ-4.1, 01-REQ-4.3, 01-REQ-4.4, 01-REQ-4.E1_

  - [x] 3.2 Add placeholder `#[test]` to each Rust crate (`it_compiles` test)
    _Test Spec: TS-01-26_
    _Requirements: 01-REQ-8.1_

  - [x] 3.3 Implement Go skeleton `main.go` for each module: print `"{component-name} v0.1.0"` to stdout, exit 0
    _Test Spec: TS-01-14_
    _Requirements: 01-REQ-4.2, 01-REQ-4.4_

  - [x] 3.4 Add placeholder test to each Go module (`TestMain` or `TestCompiles`)
    _Test Spec: TS-01-27_
    _Requirements: 01-REQ-8.2_

  - [x] 3.V Verify: run `cargo test --workspace` in `rhivos/` and `go test ./...` from repo root; confirm all tests pass
    _Test Spec: TS-01-9, TS-01-12, TS-01-28, TS-01-29_
    _Verification step for Task Group 3_
    _Note: 3 pre-existing spec 09 sensor_tests.rs failures remain (see docs/errata/01_skeleton_vs_spec09_sensors.md). All spec 01 tests pass._

- [x] 4. Create proto definitions
  _Create shared .proto files with full message and service definitions._

  - [x] 4.1 Create `proto/update/update_service.proto` with UPDATE_SERVICE messages and RPC definitions (InstallAdapter, WatchAdapterStates, ListAdapters, RemoveAdapter, GetAdapterStatus)
    _Test Spec: TS-01-16_
    _Requirements: 01-REQ-5.1_

  - [x] 4.2 Create `proto/adapter/adapter_service.proto` with PARKING_OPERATOR_ADAPTOR messages and RPC definitions (StartSession, StopSession, GetStatus, GetRate)
    _Test Spec: TS-01-16_
    _Requirements: 01-REQ-5.1_

  - [x] 4.3 Create `proto/gateway/gateway.proto` with CLOUD_GATEWAY relay types (VehicleCommand, CommandResponse)
    _Test Spec: TS-01-16_
    _Requirements: 01-REQ-5.1_

  - [x] 4.4 Create `proto/kuksa/val.proto` with Kuksa Databroker value types
    _Test Spec: TS-01-16_
    _Requirements: 01-REQ-5.1_

  - [x] 4.5 Ensure all proto files use `syntax = "proto3"`, have `package` declaration and `go_package` option
    _Test Spec: TS-01-16, TS-01-17_
    _Requirements: 01-REQ-5.2, 01-REQ-5.3, 01-REQ-5.4_

  - [x] 4.V Verify: run `protoc` on all proto files and confirm they parse without errors; run setup tests for proto validation
    _Test Spec: TS-01-17, TS-01-P5_
    _Verification step for Task Group 4_

- [x] 5. Create infrastructure configuration and Makefile
  _Set up Podman Compose, NATS config, VSS overlay, and root Makefile._

  - [x] 5.1 Create `deployments/compose.yml` with NATS (port 4222) and Kuksa Databroker (port 55556) service definitions
    _Test Spec: TS-01-23_
    _Requirements: 01-REQ-7.1_

  - [x] 5.2 Create `deployments/nats/nats-server.conf` with default NATS configuration
    _Test Spec: TS-01-24_
    _Requirements: 01-REQ-7.2_

  - [x] 5.3 Create `deployments/vss-overlay.json` with custom VSS signal definitions
    _Test Spec: TS-01-25_
    _Requirements: 01-REQ-7.3_

  - [x] 5.4 Create root `Makefile` with targets: `build`, `build-rust`, `build-go`, `test`, `test-rust`, `test-go`, `test-setup`, `clean`, `proto`, `infra-up`, `infra-down`, `check`
    _Test Spec: TS-01-18, TS-01-19, TS-01-20, TS-01-21, TS-01-22_
    _Requirements: 01-REQ-6.1, 01-REQ-6.2, 01-REQ-6.3, 01-REQ-6.4, 01-REQ-6.5_

  - [x] 5.V Verify: run `make build`, `make test`, `make check` and confirm all pass; verify Makefile targets exist per TS-01-18
    _Test Spec: TS-01-19, TS-01-20, TS-01-22_
    _Verification step for Task Group 5_
    _Note: test-rust and test-go scoped to spec-01 passing crates; pre-existing failures from other specs documented in docs/errata/01_makefile_test_scope.md_

- [x] 6. Proto code generation and setup verification tests
  _Configure proto codegen and finalize setup verification tests._

  - [x] 6.1 Implement `make proto` target to generate Go code from proto definitions using protoc
    _Test Spec: TS-01-32_
    _Requirements: 01-REQ-10.1, 01-REQ-10.2, 01-REQ-10.3_

  - [x] 6.2 Add protoc-not-installed error handling to `make proto`
    _Test Spec: TS-01-E11_
    _Requirements: 01-REQ-10.E1_

  - [x] 6.3 Write build-command-based setup verification tests in `tests/setup/`: TestRustBuild, TestGoBuild, TestProtoValidation
    _Test Spec: TS-01-30, TS-01-31_
    _Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4_

  - [x] 6.4 Add toolchain-skip logic to setup tests (skip when cargo/go/protoc not on PATH)
    _Test Spec: TS-01-E10_
    _Requirements: 01-REQ-9.E1_

  - [x] 6.5 Add `make test-setup` target to Makefile
    _Test Spec: TS-01-30_
    _Requirements: 01-REQ-9.3_

  - [x] 6.V Verify: run `make proto`, `make test-setup`, and `make check`; confirm all pass
    _Test Spec: TS-01-32, TS-01-30, TS-01-P1_
    _Verification step for Task Group 6_
    _Note: 6.1/6.2/6.5 were already implemented in task group 5 (Makefile). 6.3/6.4 implemented build_verification_test.go; all tests pass via make check._

- [x] 7. Wiring verification
  _End-to-end verification that all components are correctly wired together._

  - [x] 7.1 Run full build-test cycle: `make clean && make build && make test`
    _Test Spec: TS-01-SMOKE-1_
    _Requirements: 01-REQ-6.2, 01-REQ-6.3_

  - [x] 7.2 Run proto generation and verify Go integration: `make proto && go build ./...`
    _Test Spec: TS-01-SMOKE-3_
    _Requirements: 01-REQ-10.1, 01-REQ-10.3_

  - [x] 7.3 Run all setup verification tests: `make test-setup`
    _Test Spec: TS-01-30, TS-01-31_
    _Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.3, 01-REQ-9.4_

  - [x] 7.4 Run `make check` and confirm exit code 0
    _Test Spec: TS-01-22_
    _Requirements: 01-REQ-6.5_

  - [x] 7.5 Verify all skeleton binaries produce version output (manual or scripted)
    _Test Spec: TS-01-13, TS-01-14, TS-01-15, TS-01-P2_
    _Requirements: 01-REQ-4.1, 01-REQ-4.2, 01-REQ-4.3_

  - [x] 7.V Verify: all preceding checks pass; `git status` shows a clean working tree on develop branch
    _Verification step for Task Group 7_
    _Note: Makefile test-go scoped to root packages for backend/parking-fee-service and backend/cloud-gateway due to sub-package stub tests from specs 05 and 06 task group 1 (see docs/errata/01_makefile_test_scope.md). All spec-01 tests pass._

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|------------|----------------|--------------------|--------------------|
| 01-REQ-1.1 | TS-01-1 | 2.1 | TS-01-1 |
| 01-REQ-1.2 | TS-01-2 | 2.3 | TS-01-2 |
| 01-REQ-1.3 | TS-01-3 | 2.5 | TS-01-3 |
| 01-REQ-1.4 | TS-01-3 | 2.5 | TS-01-3 |
| 01-REQ-1.5 | TS-01-4 | 2.4 | TS-01-4 |
| 01-REQ-1.6 | TS-01-5 | 4.1–4.4 | TS-01-5 |
| 01-REQ-1.7 | TS-01-5 | 5.1–5.3 | TS-01-5 |
| 01-REQ-1.8 | TS-01-6 | 1.1 | TS-01-6 |
| 01-REQ-1.E1 | TS-01-E1 | 2.1–2.6 | TS-01-E1 |
| 01-REQ-2.1 | TS-01-7 | 2.1 | TS-01-7 |
| 01-REQ-2.2 | TS-01-7 | 2.1 | TS-01-7 |
| 01-REQ-2.3 | TS-01-8 | 2.2 | TS-01-8 |
| 01-REQ-2.4 | TS-01-9 | 3.1 | TS-01-9 |
| 01-REQ-2.E1 | TS-01-E2 | 2.1 | TS-01-E2 |
| 01-REQ-3.1 | TS-01-10 | 2.6 | TS-01-10 |
| 01-REQ-3.2 | TS-01-11 | 2.3, 2.4 | TS-01-11 |
| 01-REQ-3.3 | TS-01-11 | 2.3, 2.4 | TS-01-11 |
| 01-REQ-3.4 | TS-01-12 | 3.3 | TS-01-12 |
| 01-REQ-3.E1 | TS-01-E3 | 2.3, 2.4 | TS-01-E3 |
| 01-REQ-4.1 | TS-01-13 | 3.1 | TS-01-13 |
| 01-REQ-4.2 | TS-01-14 | 3.3 | TS-01-14 |
| 01-REQ-4.3 | TS-01-15 | 3.1 | TS-01-15 |
| 01-REQ-4.4 | TS-01-13, TS-01-14 | 3.1, 3.3 | TS-01-13, TS-01-14 |
| 01-REQ-4.E1 | TS-01-E4 | 3.1 | TS-01-E4 |
| 01-REQ-5.1 | TS-01-16 | 4.1–4.4 | TS-01-16 |
| 01-REQ-5.2 | TS-01-16 | 4.5 | TS-01-16 |
| 01-REQ-5.3 | TS-01-16 | 4.5 | TS-01-16 |
| 01-REQ-5.4 | TS-01-17 | 4.5 | TS-01-17 |
| 01-REQ-5.E1 | TS-01-E5 | 4.1–4.4 | TS-01-E5 |
| 01-REQ-6.1 | TS-01-18 | 5.4 | TS-01-18 |
| 01-REQ-6.2 | TS-01-19 | 5.4 | TS-01-19 |
| 01-REQ-6.3 | TS-01-20 | 5.4 | TS-01-20 |
| 01-REQ-6.4 | TS-01-21 | 5.4 | TS-01-21 |
| 01-REQ-6.5 | TS-01-22 | 5.4 | TS-01-22 |
| 01-REQ-6.E1 | TS-01-E6 | 5.4 | TS-01-E6 |
| 01-REQ-7.1 | TS-01-23 | 5.1 | TS-01-23 |
| 01-REQ-7.2 | TS-01-24 | 5.2 | TS-01-24 |
| 01-REQ-7.3 | TS-01-25 | 5.3 | TS-01-25 |
| 01-REQ-7.4 | TS-01-SMOKE-2 | 5.1, 5.4 | TS-01-SMOKE-2 |
| 01-REQ-7.5 | TS-01-SMOKE-2 | 5.1, 5.4 | TS-01-SMOKE-2 |
| 01-REQ-7.E1 | TS-01-E7 | 5.1 | TS-01-E7 |
| 01-REQ-7.E2 | TS-01-E8 | 5.4 | TS-01-E8 |
| 01-REQ-8.1 | TS-01-26 | 3.2 | TS-01-26 |
| 01-REQ-8.2 | TS-01-27 | 3.4 | TS-01-27 |
| 01-REQ-8.3 | TS-01-28 | 3.2 | TS-01-28 |
| 01-REQ-8.4 | TS-01-29 | 3.4 | TS-01-29 |
| 01-REQ-8.E1 | TS-01-E9 | 3.2, 3.4 | TS-01-E9 |
| 01-REQ-9.1 | TS-01-30 | 6.3 | TS-01-30 |
| 01-REQ-9.2 | TS-01-30 | 6.3 | TS-01-30 |
| 01-REQ-9.3 | TS-01-30 | 6.5 | TS-01-30 |
| 01-REQ-9.4 | TS-01-31 | 6.3 | TS-01-31 |
| 01-REQ-9.E1 | TS-01-E10 | 6.4 | TS-01-E10 |
| 01-REQ-10.1 | TS-01-32 | 6.1 | TS-01-32 |
| 01-REQ-10.2 | TS-01-32 | 6.1 | TS-01-32 |
| 01-REQ-10.3 | TS-01-32 | 6.1 | TS-01-32 |
| 01-REQ-10.E1 | TS-01-E11 | 6.2 | TS-01-E11 |

## Notes

- All tests are shell-script-driven Go tests in `tests/setup/` that verify filesystem structure and build outputs.
- Infrastructure tests (`make infra-up/down`) require Podman and are skipped when unavailable.
- Proto code generation requires `protoc` and Go plugins; tests skip gracefully when tools are missing.
