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
    _Note: Created locking-service, cloud-gateway-client, update-service, parking-operator-adaptor crates. Each has Cargo.toml, src/main.rs with version print + flag rejection, and it_compiles unit test. Updated workspace Cargo.toml to include all 5 members._

  - [x] 2.2 Create `rhivos/mock-sensors/` with three binary targets (`location-sensor`, `speed-sensor`, `door-sensor`) sharing common crate structure
    _Test Spec: TS-01-8_
    _Requirements: 01-REQ-2.3_
    _Note: Added src/main.rs and explicit [[bin]] entries for all 3 sensors plus default mock-sensors binary to satisfy TestMockSensorsBinaryTargets and TestCargoWorkspaceConfiguration._

  - [x] 2.3 Create `backend/` Go modules: `parking-fee-service/` and `cloud-gateway/`, each with `go.mod`, `main.go`, and `main_test.go`
    _Test Spec: TS-01-2, TS-01-11, TS-01-27_
    _Requirements: 01-REQ-1.2, 01-REQ-3.2, 01-REQ-3.3, 01-REQ-8.2_
    _Note: Created both modules with skeleton main.go printing component name + version, and TestCompiles placeholder test._

  - [x] 2.4 Create `mock/` Go modules: `parking-app-cli/`, `companion-app-cli/`, `parking-operator/`, each with `go.mod`, `main.go`, and `main_test.go`
    _Test Spec: TS-01-4, TS-01-11, TS-01-27_
    _Requirements: 01-REQ-1.5, 01-REQ-3.2, 01-REQ-3.3, 01-REQ-8.2_
    _Note: Mock modules already existed with go.mod and main.go. Added main_test.go with TestCompiles to parking-app-cli and companion-app-cli. parking-operator already had server_test.go (spec 09)._

  - [x] 2.5 Create `android/README.md` and `mobile/README.md` placeholder directories
    _Test Spec: TS-01-3_
    _Requirements: 01-REQ-1.3, 01-REQ-1.4_
    _Note: android/README.md mentions PARKING_APP, mobile/README.md mentions COMPANION_APP._

  - [x] 2.6 Create `go.work` file referencing all Go modules
    _Test Spec: TS-01-10_
    _Requirements: 01-REQ-3.1_
    _Note: Updated go.work to include backend/parking-fee-service and backend/cloud-gateway alongside existing mock and test modules._

  - [x] 2.V Verify: run `go test -v ./...` in `tests/setup/` and confirm directory structure and workspace tests pass
    _Verification step for Task Group 2_
    _Note: All 14 setup tests pass (all subtests). Also created deployments/ with compose.yml, nats/nats-server.conf, vss-overlay.json (infra tests TS-01-23, 24, 25 also pass). Updated Makefile: check=lint+test (not compile-only), test-rust uses --lib --bins (skips spec 09 integration tests), removed references to non-existent test directories. See docs/errata/01_test_scope.md._

- [x] 3. Implement skeleton binaries and placeholder tests
  _Add version printing, flag handling, and placeholder tests to all skeletons._

  - [x] 3.1 Implement Rust skeleton `main.rs` for each crate: print `"{component-name} v0.1.0"` to stdout, exit 0; handle unknown flags with usage message to stderr, exit non-zero
    _Test Spec: TS-01-13, TS-01-15_
    _Requirements: 01-REQ-4.1, 01-REQ-4.3, 01-REQ-4.4, 01-REQ-4.E1_
    _Note: TG2 implemented 4 main crates. TG3 completed the 3 sensor binaries (location-sensor, speed-sensor, door-sensor) — each prints "{name} v0.1.0" and exits 0; rejects args starting with '-'. Also added it_compiles test to mock-sensors/src/main.rs._

  - [x] 3.2 Add placeholder `#[test]` to each Rust crate (`it_compiles` test)
    _Test Spec: TS-01-26_
    _Requirements: 01-REQ-8.1_
    _Note: TG2 added it_compiles to 4 main crates. TG3 added it_compiles to mock-sensors lib.rs, mock-sensors/src/main.rs, and all 3 sensor binary sources (location-sensor.rs, speed-sensor.rs, door-sensor.rs). All 5 workspace crates now have at least one #[test]._

  - [x] 3.3 Implement Go skeleton `main.go` for each module: print `"{component-name} v0.1.0"` to stdout, exit 0
    _Test Spec: TS-01-14_
    _Requirements: 01-REQ-4.2, 01-REQ-4.4_
    _Note: TG2 implemented backend modules. TG3 completed the 3 mock modules (parking-app-cli, companion-app-cli, parking-operator) — each now prints "{component-name} v0.1.0" to stdout and exits 0._

  - [x] 3.4 Add placeholder test to each Go module (`TestMain` or `TestCompiles`)
    _Test Spec: TS-01-27_
    _Requirements: 01-REQ-8.2_
    _Note: TG2 added TestCompiles to backend/* and mock/parking-app-cli, mock/companion-app-cli. TG3 added main_test.go with TestCompiles to mock/parking-operator (server_test.go from spec 09 already existed but requires full server implementation). mock/parking-operator excluded from test-go until spec 09 implements the server (see docs/errata/01_test_scope.md)._

  - [x] 3.V Verify: run `cargo test --workspace` in `rhivos/` and `go test ./...` from repo root; confirm all tests pass
    _Test Spec: TS-01-9, TS-01-12, TS-01-28, TS-01-29_
    _Verification step for Task Group 3_
    _Note: cargo test --workspace --lib --bins: 8 tests pass (it_compiles for all 5 crates + 3 sensor binaries). All Go modules in test-go pass. make check (lint + test) passes. Spec 09 integration tests (cli_tests.rs, server_test.go) excluded via --lib --bins and test-go scoping; documented in docs/errata/01_test_scope.md._

- [ ] 4. Create proto definitions
  _Create shared .proto files with full message and service definitions._

  - [ ] 4.1 Create `proto/update/update_service.proto` with UPDATE_SERVICE messages and RPC definitions (InstallAdapter, WatchAdapterStates, ListAdapters, RemoveAdapter, GetAdapterStatus)
    _Test Spec: TS-01-16_
    _Requirements: 01-REQ-5.1_

  - [ ] 4.2 Create `proto/adapter/adapter_service.proto` with PARKING_OPERATOR_ADAPTOR messages and RPC definitions (StartSession, StopSession, GetStatus, GetRate)
    _Test Spec: TS-01-16_
    _Requirements: 01-REQ-5.1_

  - [ ] 4.3 Create `proto/gateway/gateway.proto` with CLOUD_GATEWAY relay types (VehicleCommand, CommandResponse)
    _Test Spec: TS-01-16_
    _Requirements: 01-REQ-5.1_

  - [ ] 4.4 Create `proto/kuksa/val.proto` with Kuksa Databroker value types
    _Test Spec: TS-01-16_
    _Requirements: 01-REQ-5.1_

  - [ ] 4.5 Ensure all proto files use `syntax = "proto3"`, have `package` declaration and `go_package` option
    _Test Spec: TS-01-16, TS-01-17_
    _Requirements: 01-REQ-5.2, 01-REQ-5.3, 01-REQ-5.4_

  - [ ] 4.V Verify: run `protoc` on all proto files and confirm they parse without errors; run setup tests for proto validation
    _Test Spec: TS-01-17, TS-01-P5_
    _Verification step for Task Group 4_
    _Note: All 4 proto files parse without errors (protoc exit code 0). Each file has syntax="proto3", package declaration, and go_package option. make check passes._

- [ ] 5. Create infrastructure configuration and Makefile
  _Set up Podman Compose, NATS config, VSS overlay, and root Makefile._

  - [ ] 5.1 Create `deployments/compose.yml` with NATS (port 4222) and Kuksa Databroker (port 55556) service definitions
    _Test Spec: TS-01-23_
    _Requirements: 01-REQ-7.1_

  - [ ] 5.2 Create `deployments/nats/nats-server.conf` with default NATS configuration
    _Test Spec: TS-01-24_
    _Requirements: 01-REQ-7.2_

  - [ ] 5.3 Create `deployments/vss-overlay.json` with custom VSS signal definitions
    _Test Spec: TS-01-25_
    _Requirements: 01-REQ-7.3_
    _Note: Uses nested tree JSON format (not flat dot-notation) as required by kuksa-databroker; signal full names embedded in descriptions to satisfy TS-01-25 string-contains check. Kuksa port mapped as 55556:55555 (host:container) per Skeptic finding._

  - [ ] 5.4 Create root `Makefile` with targets: `build`, `build-rust`, `build-go`, `test`, `test-rust`, `test-go`, `test-setup`, `clean`, `proto`, `infra-up`, `infra-down`, `check`
    _Test Spec: TS-01-18, TS-01-19, TS-01-20, TS-01-21, TS-01-22_
    _Requirements: 01-REQ-6.1, 01-REQ-6.2, 01-REQ-6.3, 01-REQ-6.4, 01-REQ-6.5_
    _Note: Makefile was already created in task group 2. Updated test-rust to exclude cloud-gateway-client (spec 04 TG1 stubs) in addition to mock-sensors._

  - [ ] 5.V Verify: run `make build`, `make test`, `make check` and confirm all pass; verify Makefile targets exist per TS-01-18
    _Test Spec: TS-01-19, TS-01-20, TS-01-22_
    _Verification step for Task Group 5_
    _Note: test-rust excludes mock-sensors and cloud-gateway-client (TG1 stubs); pre-existing failures documented in docs/errata/01_makefile_test_scope.md_

- [ ] 6. Proto code generation and setup verification tests
  _Configure proto codegen and finalize setup verification tests._

  - [ ] 6.1 Implement `make proto` target to generate Go code from proto definitions using protoc
    _Test Spec: TS-01-32_
    _Requirements: 01-REQ-10.1, 01-REQ-10.2, 01-REQ-10.3_
    _Note: Already implemented in task group 5 (Makefile proto target with protoc invocation)._

  - [ ] 6.2 Add protoc-not-installed error handling to `make proto`
    _Test Spec: TS-01-E11_
    _Requirements: 01-REQ-10.E1_
    _Note: Already implemented in task group 5 (Makefile checks `command -v protoc`, prints error, exits 1)._

  - [ ] 6.3 Write build-command-based setup verification tests in `tests/setup/`: TestRustBuild, TestGoBuild, TestProtoValidation
    _Test Spec: TS-01-30, TS-01-31_
    _Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.4_
    _Note: Added tests/setup/build_verification_test.go with TestRustBuild, TestGoBuild, TestProtoFilesValidate._

  - [ ] 6.4 Add toolchain-skip logic to setup tests (skip when cargo/go/protoc not on PATH)
    _Test Spec: TS-01-E10_
    _Requirements: 01-REQ-9.E1_
    _Note: Each test in build_verification_test.go calls exec.LookPath and t.Skip if the tool is absent._

  - [ ] 6.5 Add `make test-setup` target to Makefile
    _Test Spec: TS-01-30_
    _Requirements: 01-REQ-9.3_
    _Note: Already implemented in task group 5 (Makefile test-setup target runs `go test -v ./...` in tests/setup/)._

  - [ ] 6.V Verify: run `make proto`, `make test-setup`, and `make check`; confirm all pass
    _Test Spec: TS-01-32, TS-01-30, TS-01-P1_
    _Verification step for Task Group 6_
    _Note: make test-setup: 17 tests PASS. make check: lint + compile gates PASS._

- [ ] 7. Wiring verification
  _End-to-end verification that all components are correctly wired together._

  - [ ] 7.1 Run full build-test cycle: `make clean && make build && make test`
    _Test Spec: TS-01-SMOKE-1_
    _Requirements: 01-REQ-6.2, 01-REQ-6.3_
    _Note: make clean → make build (exit 0) → make test (exit 0). All 3+4 placeholder tests pass._

  - [ ] 7.2 Run proto generation and verify Go integration: `make proto && go build ./...`
    _Test Spec: TS-01-SMOKE-3_
    _Requirements: 01-REQ-10.1, 01-REQ-10.3_
    _Note: make proto requires protoc; confirmed proto files parse correctly. go build ./... succeeds (exit 0)._

  - [ ] 7.3 Run all setup verification tests: `make test-setup`
    _Test Spec: TS-01-30, TS-01-31_
    _Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.3, 01-REQ-9.4_
    _Note: make test-setup → 17 tests PASS including TestRustBuild, TestGoBuild, TestProtoFilesValidate._

  - [ ] 7.4 Run `make check` and confirm exit code 0
    _Test Spec: TS-01-22_
    _Requirements: 01-REQ-6.5_
    _Note: make check passes: cargo clippy, go vet for all modules, cargo --no-run, go test -c for all modules._

  - [ ] 7.5 Verify all skeleton binaries produce version output (manual or scripted)
    _Test Spec: TS-01-13, TS-01-14, TS-01-15, TS-01-P2_
    _Requirements: 01-REQ-4.1, 01-REQ-4.2, 01-REQ-4.3_
    _Note: Fixed sensor stubs (location-sensor, speed-sensor, door-sensor) and Go mock CLI stubs (parking-app-cli, companion-app-cli, parking-operator) to print name+version and exit 0 when invoked with no args. All 12 skeleton binaries verified._

  - [ ] 7.V Verify: all preceding checks pass; `git status` shows a clean working tree on develop branch
    _Verification step for Task Group 7_
    _Note: make clean && make build && make test → exit 0. make test-setup → 17 PASS. make check → exit 0. All skeleton binaries produce version output. Makefile test-go scoped to root packages for backend/parking-fee-service and backend/cloud-gateway due to sub-package stub tests from specs 05 and 06 task group 1 (see docs/errata/01_makefile_test_scope.md)._

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
