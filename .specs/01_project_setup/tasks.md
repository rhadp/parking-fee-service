# Implementation Plan: Project Setup (Phase 1.2)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the project scaffolding for the SDV Parking Demo System.
The approach is test-first: task group 1 creates a standalone Go test module
(`tests/setup/`) that encodes all 60 test contracts from `test_spec.md` as
failing tests. Subsequent groups build the actual scaffolding (directories,
protos, Rust workspace, Go modules, Makefile, infrastructure) to make those
tests pass incrementally.

Ordering rationale:
1. Tests first (red) — establishes the verification baseline
2. Structure and protos — foundational layer everything else depends on
3. Rust scaffolding — depends on proto files
4. Go scaffolding — depends on proto files and gen/go
5. Build system — depends on all components being buildable
6. Infrastructure — depends on Makefile for infra-up/down targets

## Test Commands

- Spec tests: `cd tests/setup && go test -v -count=1 ./...`
- Spec tests (structural only): `cd tests/setup && go test -v -count=1 -run TestStructure ./...`
- Spec tests (build only): `cd tests/setup && go test -v -count=1 -run TestBuild ./...`
- Spec tests (property only): `cd tests/setup && go test -v -count=1 -run TestProperty ./...`
- Spec tests (edge only): `cd tests/setup && go test -v -count=1 -run TestEdge ./...`
- Rust unit tests: `cd rhivos && cargo test`
- Go unit tests (all): `go test ./backend/... ./mock/...` (from repo root with go.work)
- Linter (Rust): `cd rhivos && cargo clippy -- -D warnings`
- Linter (Go): `go vet ./backend/... ./mock/... ./gen/...`
- All tests: `make check`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Set up test module structure
    - Create `tests/setup/go.mod` as a standalone Go module
      (module path: `github.com/rhadp/parking-fee-service/tests/setup`)
    - Create a `helpers_test.go` with `repoRoot(t)` helper that locates the
      repo root by walking up from the test file directory until it finds `.git`
    - Add helpers: `assertDirExists`, `assertFileExists`, `assertFileContains`,
      `execCommand`, `waitForPort`, `portIsOpen`
    - _Test Spec: all (shared infrastructure)_

  - [x] 1.2 Write structural tests
    - Translate TS-01-1 through TS-01-6 (directory structure) into Go tests
    - Translate TS-01-34 through TS-01-36 (infra file content) into Go tests
    - Translate TS-01-42 (integration test directory) into Go test
    - Group under `TestStructure_*` naming convention
    - _Test Spec: TS-01-1 through TS-01-6, TS-01-34, TS-01-35, TS-01-36, TS-01-42_

  - [x] 1.3 Write proto and Rust source content tests
    - Translate TS-01-7 through TS-01-11 (proto definitions) into Go tests
    - Translate TS-01-12, TS-01-15, TS-01-16 (Rust workspace content) into Go tests
    - Group under `TestProto_*` and `TestRust_*` naming conventions
    - _Test Spec: TS-01-7 through TS-01-12, TS-01-15, TS-01-16_

  - [x] 1.4 Write Go structure and mock CLI tests
    - Translate TS-01-17, TS-01-20 (Go module existence, generated proto) into Go tests
    - Translate TS-01-23 through TS-01-27 (mock CLI existence, help, proto imports) into Go tests
    - Group under `TestGo_*` and `TestMockCLI_*` naming conventions
    - _Test Spec: TS-01-17, TS-01-20, TS-01-23 through TS-01-27_

  - [x] 1.5 Write build and runtime tests
    - Translate TS-01-13, TS-01-14 (Rust build/test) into Go tests
    - Translate TS-01-18, TS-01-19 (Go build/test) into Go tests
    - Translate TS-01-21, TS-01-22 (backend HTTP stubs) into Go tests
    - Translate TS-01-25, TS-01-26 (mock CLI build/help) into Go tests
    - Translate TS-01-28 through TS-01-33 (Makefile targets) into Go tests
    - Translate TS-01-37 through TS-01-41 (infra runtime, test isolation) into Go tests
    - Translate TS-01-39, TS-01-40 (placeholder tests) into Go tests
    - Group under `TestBuild_*`, `TestMake_*`, `TestInfra_*` naming conventions
    - _Test Spec: TS-01-13, TS-01-14, TS-01-18, TS-01-19, TS-01-21, TS-01-22, TS-01-25, TS-01-26, TS-01-28 through TS-01-33, TS-01-37 through TS-01-41_

  - [x] 1.6 Write edge case and property tests
    - Translate TS-01-E1 through TS-01-E11 into Go tests
    - Translate TS-01-P1 through TS-01-P7 into Go tests
    - Group under `TestEdge_*` and `TestProperty_*` naming conventions
    - _Test Spec: TS-01-E1 through TS-01-E11, TS-01-P1 through TS-01-P7_

  - [x] 1.V Verify task group 1
    - [x] All spec tests exist and are syntactically valid:
      `cd tests/setup && go vet ./...`
    - [x] All spec tests FAIL (red) — no implementation yet:
      `cd tests/setup && go test -count=1 ./... 2>&1 | grep -c FAIL`
    - [x] No linter warnings introduced:
      `cd tests/setup && go vet ./...`

- [x] 2. Repository structure, protos, and config files
  - [x] 2.1 Create directory layout
    - Create all directories: `proto/`, `gen/go/`, `rhivos/`, `backend/parking-fee-service/`,
      `backend/cloud-gateway/`, `mock/parking-app-cli/`, `mock/companion-app-cli/`,
      `infra/mosquitto/`, `tests/integration/`
    - Create placeholder READMEs in `aaos/parking-app/`, `android/companion-app/`,
      `tests/integration/`
    - _Requirements: 01-REQ-1.1 through 01-REQ-1.6_

  - [x] 2.2 Write proto files
    - Create `proto/common.proto` with AdapterState enum, AdapterInfo, ErrorDetails
    - Create `proto/update_service.proto` with UpdateService RPCs
    - Create `proto/parking_adaptor.proto` with ParkingAdaptor RPCs
    - All files use proto3 syntax with correct go_package options
    - Verify: `protoc --proto_path=proto/ --descriptor_set_out=/dev/null proto/*.proto`
    - _Requirements: 01-REQ-2.1 through 01-REQ-2.5_

  - [x] 2.3 Create infrastructure config files
    - Create `infra/docker-compose.yml` with Mosquitto (:1883) and Kuksa (:55556)
    - Create `infra/mosquitto/mosquitto.conf` with anonymous access on port 1883
    - _Requirements: 01-REQ-7.1, 01-REQ-7.2, 01-REQ-7.3_

  - [x] 2.V Verify task group 2
    - [x] Structural spec tests pass:
      `cd tests/setup && go test -v -count=1 -run "TestStructure|TestProto_Definition|TestProto_Compile|TestProto_Syntax|TestInfra_ComposeFile|TestInfra_Mosquitto|TestInfra_Kuksa" ./...`
    - [x] Proto files compile:
      `protoc --proto_path=proto/ --descriptor_set_out=/dev/null proto/*.proto`
    - [x] No linter warnings introduced

- [x] 3. Rust workspace scaffolding
  - [x] 3.1 Create Cargo workspace
    - Create `rhivos/Cargo.toml` with workspace members and shared dependencies
      (tonic, prost, tokio, tonic-build)
    - Set resolver = "2", edition 2021
    - _Requirements: 01-REQ-3.1_

  - [x] 3.2 Create locking-service and cloud-gateway-client crates
    - Create `Cargo.toml`, `src/main.rs`, `src/lib.rs` for each
    - `main.rs`: tokio runtime, prints "not implemented" and exits
    - `lib.rs`: placeholder module (no proto dependency for these crates —
      they interact with DATA_BROKER via Kuksa's proto, not our custom protos)
    - Add one placeholder `#[test]` per crate
    - _Requirements: 01-REQ-3.1, 01-REQ-3.5, 01-REQ-8.1_

  - [x] 3.3 Create update-service and parking-operator-adaptor crates
    - Create `Cargo.toml`, `build.rs`, `src/main.rs`, `src/lib.rs` for each
    - `build.rs`: uses tonic-build to compile the crate's proto file from
      `../../proto/`
    - `lib.rs`: re-exports generated types, implements gRPC service trait
      with all methods returning `Status::unimplemented("not yet implemented")`
    - `main.rs`: tokio runtime, prints "not implemented" and exits
    - Add one placeholder `#[test]` per crate
    - _Requirements: 01-REQ-3.1, 01-REQ-3.4, 01-REQ-3.5, 01-REQ-8.1_

  - [x] 3.4 Verify Rust workspace builds and tests pass
    - `cd rhivos && cargo build`
    - `cd rhivos && cargo test`
    - `cd rhivos && cargo clippy -- -D warnings`
    - _Requirements: 01-REQ-3.2, 01-REQ-3.3_

  - [x] 3.V Verify task group 3
    - [x] Rust spec tests pass:
      `cd tests/setup && go test -v -count=1 -run "TestRust" ./...`
    - [x] All existing tests still pass:
      `cd tests/setup && go test -v -count=1 -run "TestStructure|TestProto|TestRust" ./...`
    - [x] No linter warnings: `cd rhivos && cargo clippy -- -D warnings`
    - [x] Requirements 01-REQ-3.1 through 01-REQ-3.5, 01-REQ-8.1 met

- [x] 4. Go scaffolding: generated code, backend services, mock CLIs
  - [x] 4.1 Generate Go proto code
    - Create `gen/go/go.mod`
      (module: `github.com/rhadp/parking-fee-service/gen/go`)
    - Run protoc with `protoc-gen-go` and `protoc-gen-go-grpc` to generate
      code into `gen/go/commonpb/`, `gen/go/updateservicepb/`,
      `gen/go/parkingadaptorpb/`
    - Verify generated code compiles: `cd gen/go && go build ./...`
    - _Requirements: 01-REQ-4.4_

  - [x] 4.2 Create parking-fee-service skeleton
    - Create `backend/parking-fee-service/go.mod` and `main.go`
    - HTTP server on `:8080` (configurable via PORT env var)
    - `GET /health` returns 200 `{"status": "ok"}`
    - `GET /operators` returns 501
    - `GET /operators/{id}/adapter` returns 501
    - Add one placeholder test in `main_test.go`
    - _Requirements: 01-REQ-4.1, 01-REQ-4.2, 01-REQ-4.5, 01-REQ-8.2_

  - [x] 4.3 Create cloud-gateway skeleton
    - Create `backend/cloud-gateway/go.mod` and `main.go`
    - HTTP server on `:8081` (configurable via PORT env var)
    - `GET /health` returns 200 `{"status": "ok"}`
    - `POST /vehicles/{vin}/commands` returns 501
    - Print "MQTT client: not connected" on startup
    - Add one placeholder test in `main_test.go`
    - _Requirements: 01-REQ-4.1, 01-REQ-4.2, 01-REQ-4.6, 01-REQ-8.2_

  - [x] 4.4 Create mock parking-app-cli
    - Create `mock/parking-app-cli/go.mod` and `main.go`
    - Use cobra for CLI: root command with help, subcommands (lookup, install,
      watch, list, status, start-session, stop-session, get-status, get-rate)
    - Global flags: `--pfs-url`, `--update-addr`, `--adaptor-addr`
    - All subcommands print "not implemented: <command>" to stderr, exit 1
    - Import generated proto packages from `gen/go/`
    - Add one placeholder test in `main_test.go`
    - _Requirements: 01-REQ-5.1, 01-REQ-5.3, 01-REQ-5.4, 01-REQ-5.5, 01-REQ-8.2_

  - [x] 4.5 Create mock companion-app-cli
    - Create `mock/companion-app-cli/go.mod` and `main.go`
    - Use cobra for CLI: root command with help, subcommands (lock, unlock,
      status)
    - Global flags: `--gateway-url`, `--vin`, `--token`
    - All subcommands print "not implemented: <command>" to stderr, exit 1
    - Import generated proto packages from `gen/go/`
    - Add one placeholder test in `main_test.go`
    - _Requirements: 01-REQ-5.2, 01-REQ-5.3, 01-REQ-5.4, 01-REQ-5.5, 01-REQ-8.2_

  - [x] 4.6 Create Go workspace and verify
    - Create `go.work` at repo root linking all Go modules
    - Verify all Go modules build: `go build ./backend/... ./mock/...`
    - Verify all Go tests pass: `go test ./backend/... ./mock/...`
    - Verify Go vet: `go vet ./backend/... ./mock/... ./gen/...`
    - _Requirements: 01-REQ-4.2, 01-REQ-4.3, 01-REQ-8.2_

  - [x] 4.V Verify task group 4
    - [x] Go and mock CLI spec tests pass:
      `cd tests/setup && go test -v -count=1 -run "TestGo|TestMockCLI" ./...`
    - [x] All existing tests still pass:
      `cd tests/setup && go test -v -count=1 -run "TestStructure|TestProto|TestRust|TestGo|TestMockCLI" ./...`
    - [x] No linter warnings: `go vet ./backend/... ./mock/... ./gen/...`
    - [x] Requirements 01-REQ-4.1 through 01-REQ-4.6, 01-REQ-5.1 through
      01-REQ-5.5, 01-REQ-8.2 met

- [ ] 5. Checkpoint — component scaffolds complete
  - All component directories, proto files, Rust crates, Go modules, and
    mock CLI apps exist and compile.
  - Verify end-to-end:
    - `cd rhivos && cargo build && cargo test && cargo clippy -- -D warnings`
    - `go build ./backend/... ./mock/...`
    - `go test ./backend/... ./mock/...`
  - Ask the user if questions arise before proceeding to build system.

- [ ] 6. Build system and local infrastructure
  - [ ] 6.1 Create Makefile with toolchain detection
    - Check for `rustc`/`cargo`, `go`, `protoc`, `podman`/`docker` at the
      start of relevant targets
    - Print "Error: <tool> not found. Please install <tool>." and exit 1
      if missing
    - _Requirements: 01-REQ-6.1, 01-REQ-6.E1_

  - [ ] 6.2 Add build, test, lint, clean targets
    - `build`: `cd rhivos && cargo build` + `go build` for each Go module
    - `test`: `cd rhivos && cargo test` + `go test` for each Go module
    - `lint`: `cd rhivos && cargo clippy -- -D warnings` + `go vet` for each
      Go module
    - `clean`: `cd rhivos && cargo clean` + remove Go binaries
    - `build` should continue building other components if one fails (use
      `-k` flag or per-component error tracking)
    - _Requirements: 01-REQ-6.2, 01-REQ-6.3, 01-REQ-6.4, 01-REQ-6.6,
      01-REQ-6.E2_

  - [ ] 6.3 Add proto and check targets
    - `proto`: run protoc for Go code generation into `gen/go/`
    - `check`: run `build` + `test` + `lint` in sequence
    - _Requirements: 01-REQ-6.5_

  - [ ] 6.4 Add infra-up and infra-down targets
    - `infra-up`: `cd infra && podman-compose up -d` (or `docker compose up -d`)
    - `infra-down`: `cd infra && podman-compose down` (or `docker compose down`)
    - Auto-detect podman-compose vs docker compose
    - _Requirements: 01-REQ-7.4, 01-REQ-7.5_

  - [ ] 6.V Verify task group 6
    - [ ] Build system spec tests pass:
      `cd tests/setup && go test -v -count=1 -run "TestMake" ./...`
    - [ ] Infrastructure spec tests pass (requires container runtime):
      `cd tests/setup && go test -v -count=1 -run "TestInfra_Up|TestInfra_Down" ./...`
    - [ ] All existing tests still pass:
      `cd tests/setup && go test -v -count=1 ./...`
    - [ ] No linter warnings: `make lint`
    - [ ] Requirements 01-REQ-6.1 through 01-REQ-6.6, 01-REQ-7.4, 01-REQ-7.5 met

- [ ] 7. Final verification and documentation
  - [ ] 7.1 Run all spec tests and fix failures
    - Run full spec test suite:
      `cd tests/setup && go test -v -count=1 ./...`
    - Run edge case tests:
      `cd tests/setup && go test -v -count=1 -run TestEdge ./...`
    - Run property tests:
      `cd tests/setup && go test -v -count=1 -run TestProperty ./...`
    - Fix any remaining failures
    - _Test Spec: TS-01-E1 through TS-01-E11, TS-01-P1 through TS-01-P7_

  - [ ] 7.2 Update README.md
    - Project overview (what this repo is)
    - Prerequisites (Rust, Go, protoc, Podman/Docker)
    - Quick start: `make build`, `make test`, `make infra-up`
    - Repository structure overview
    - Link to `.specs/prd.md` for full product requirements

  - [ ] 7.3 Run full quality gate
    - `make check` (build + test + lint)
    - `cd tests/setup && go test -v -count=1 ./...`
    - `git status` shows clean working tree

  - [ ] 7.V Verify task group 7
    - [ ] All 60 spec tests pass (42 acceptance + 11 edge + 7 property)
    - [ ] `make check` exits 0
    - [ ] No linter warnings
    - [ ] README.md is updated
    - [ ] All changes committed and pushed

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 01-REQ-1.1 | TS-01-1 | 2.1 | `TestStructure_RustDir` |
| 01-REQ-1.2 | TS-01-2 | 2.1 | `TestStructure_GoBackendDir` |
| 01-REQ-1.3 | TS-01-3 | 2.1 | `TestStructure_ProtoDir` |
| 01-REQ-1.4 | TS-01-4 | 2.1 | `TestStructure_MockDir` |
| 01-REQ-1.5 | TS-01-5 | 2.1 | `TestStructure_PlaceholderDirs` |
| 01-REQ-1.6 | TS-01-6 | 2.3 | `TestStructure_InfraDir` |
| 01-REQ-1.E1 | TS-01-E1 | 6.1 | `TestEdge_MissingDirectory` |
| 01-REQ-2.1 | TS-01-7 | 2.2 | `TestProto_UpdateService` |
| 01-REQ-2.2 | TS-01-8 | 2.2 | `TestProto_ParkingAdaptor` |
| 01-REQ-2.3 | TS-01-9 | 2.2 | `TestProto_CommonTypes` |
| 01-REQ-2.4 | TS-01-10 | 2.2 | `TestProto_Compile` |
| 01-REQ-2.5 | TS-01-11 | 2.2 | `TestProto_Syntax` |
| 01-REQ-2.E1 | TS-01-E2 | 2.2 | `TestEdge_ProtoImportPaths` |
| 01-REQ-3.1 | TS-01-12 | 3.1, 3.2, 3.3 | `TestRust_WorkspaceMembers` |
| 01-REQ-3.2 | TS-01-13 | 3.4 | `TestBuild_RustWorkspace` |
| 01-REQ-3.3 | TS-01-14 | 3.4 | `TestBuild_RustTests` |
| 01-REQ-3.4 | TS-01-15 | 3.3 | `TestRust_ProtoGeneration` |
| 01-REQ-3.5 | TS-01-16 | 3.2, 3.3 | `TestRust_StubsUnimplemented` |
| 01-REQ-3.E1 | TS-01-E3 | 3.3 | `TestEdge_MissingProtoRust` |
| 01-REQ-4.1 | TS-01-17 | 4.2, 4.3 | `TestGo_ModulesExist` |
| 01-REQ-4.2 | TS-01-18 | 4.6 | `TestBuild_GoBackend` |
| 01-REQ-4.3 | TS-01-19 | 4.6 | `TestBuild_GoTests` |
| 01-REQ-4.4 | TS-01-20 | 4.1 | `TestGo_GeneratedProto` |
| 01-REQ-4.5 | TS-01-21 | 4.2 | `TestBuild_ParkingFeeServiceHealth` |
| 01-REQ-4.6 | TS-01-22 | 4.3 | `TestBuild_CloudGatewayStub` |
| 01-REQ-4.E1 | TS-01-E4 | 4.1 | `TestEdge_MalformedProtoGo` |
| 01-REQ-5.1 | TS-01-23 | 4.4 | `TestMockCLI_ParkingAppExists` |
| 01-REQ-5.2 | TS-01-24 | 4.5 | `TestMockCLI_CompanionAppExists` |
| 01-REQ-5.3 | TS-01-25 | 4.4, 4.5 | `TestMockCLI_BuildBinary` |
| 01-REQ-5.4 | TS-01-26 | 4.4, 4.5 | `TestMockCLI_ShowHelp` |
| 01-REQ-5.5 | TS-01-27 | 4.4, 4.5 | `TestMockCLI_ShareProtoImports` |
| 01-REQ-5.E1 | TS-01-E5 | 4.4, 4.5 | `TestEdge_UnknownCLICommand` |
| 01-REQ-6.1 | TS-01-28 | 6.1 | `TestMake_MakefileExists` |
| 01-REQ-6.2 | TS-01-29 | 6.2 | `TestMake_Build` |
| 01-REQ-6.3 | TS-01-30 | 6.2 | `TestMake_Test` |
| 01-REQ-6.4 | TS-01-31 | 6.2 | `TestMake_Lint` |
| 01-REQ-6.5 | TS-01-32 | 6.3 | `TestMake_Proto` |
| 01-REQ-6.6 | TS-01-33 | 6.2 | `TestMake_Clean` |
| 01-REQ-6.E1 | TS-01-E6, E7 | 6.1 | `TestEdge_MissingRustToolchain`, `TestEdge_MissingGoToolchain` |
| 01-REQ-6.E2 | TS-01-E8 | 6.2 | `TestEdge_PartialBuildFailure` |
| 01-REQ-7.1 | TS-01-34 | 2.3 | `TestInfra_ComposeFileExists` |
| 01-REQ-7.2 | TS-01-35 | 2.3 | `TestInfra_MosquittoConfig` |
| 01-REQ-7.3 | TS-01-36 | 2.3 | `TestInfra_KuksaConfig` |
| 01-REQ-7.4 | TS-01-37 | 6.4 | `TestInfra_Up` |
| 01-REQ-7.5 | TS-01-38 | 6.4 | `TestInfra_Down` |
| 01-REQ-7.E1 | TS-01-E9 | 6.4 | `TestEdge_PortConflict` |
| 01-REQ-7.E2 | TS-01-E10 | 6.1 | `TestEdge_MissingContainerRuntime` |
| 01-REQ-8.1 | TS-01-39 | 3.2, 3.3 | `TestBuild_RustPlaceholderTests` |
| 01-REQ-8.2 | TS-01-40 | 4.2, 4.3, 4.4, 4.5 | `TestBuild_GoPlaceholderTests` |
| 01-REQ-8.3 | TS-01-41 | 6.2 | `TestBuild_TestsWithoutInfra` |
| 01-REQ-8.4 | TS-01-42 | 2.1 | `TestStructure_IntegrationTestDir` |
| 01-REQ-8.E1 | TS-01-E11 | 4.6 | `TestEdge_EmptyTestFile` |
| Property 1 | TS-01-P1 | 3, 4 | `TestProperty_BuildCompleteness` |
| Property 2 | TS-01-P2 | 2.2, 3.3, 4.1 | `TestProperty_ProtoConsistency` |
| Property 3 | TS-01-P3 | 6.2 | `TestProperty_TestIsolation` |
| Property 4 | TS-01-P4 | 4.4, 4.5 | `TestProperty_MockCLIUsability` |
| Property 5 | TS-01-P5 | 6.4 | `TestProperty_InfraIdempotency` |
| Property 6 | TS-01-P6 | 6.2 | `TestProperty_CleanBuildReproducibility` |
| Property 7 | TS-01-P7 | 6.1 | `TestProperty_ToolchainDetection` |

## Notes

- **Test implementation language:** All spec tests are Go tests in
  `tests/setup/`. This module is standalone (not part of `go.work`) and uses
  only standard library packages (`os`, `os/exec`, `path/filepath`, `strings`,
  `testing`, `net`, `net/http`, `time`).
- **Infrastructure tests require container runtime:** Tests tagged with
  `TestInfra_Up`, `TestInfra_Down`, and `TestProperty_InfraIdempotency`
  require Podman or Docker. They should be skipped in CI environments without
  a container runtime using `t.Skip()`.
- **Edge case tests are destructive:** Tests like `TestEdge_MissingDirectory`
  temporarily rename files/directories. They must use `t.Cleanup()` to restore
  state, and should not run in parallel with other tests.
- **Proto generation prerequisites:** `protoc`, `protoc-gen-go`, and
  `protoc-gen-go-grpc` must be installed. The Makefile `proto` target
  documents installation instructions.
- **Session sizing:** Each task group is scoped for one coding session.
  Groups 2 and 3 are the smallest; group 4 (Go scaffolding) is the largest
  but consists of repetitive module creation.
