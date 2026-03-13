# Implementation Plan: Project Setup

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan sets up the SDV Parking Demo monorepo from scratch. Task group 1 writes the verification tests first (test-driven). Subsequent groups create the directory structure, Rust workspace, Go workspace, proto definitions, infrastructure, Makefile, and test infrastructure — each group making more tests pass.

The ordering ensures dependencies are built before dependents: directories first, then workspaces, then proto (which Go modules import), then infrastructure, then the root Makefile that ties everything together.

## Test Commands

- Spec tests: `cd tests/setup && go test -v ./...`
- Unit tests (Rust): `cd rhivos && cargo test`
- Unit tests (Go): `go test ./...`
- All tests: `make test`
- Linter (Rust): `cd rhivos && cargo clippy -- -D warnings`
- Linter (Go): `go vet ./...`
- All linters: `make lint`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Create tests/setup Go module
    - Create `tests/setup/go.mod` with module path `github.com/rhadp/parking-fee-service/tests/setup`
    - Create `tests/setup/structure_test.go` with tests for directory structure (TS-01-1 through TS-01-6)
    - Create `tests/setup/rust_workspace_test.go` with tests for Cargo workspace (TS-01-7 through TS-01-10)
    - _Test Spec: TS-01-1 through TS-01-10_

  - [x] 1.2 Write Go workspace and skeleton tests
    - Create `tests/setup/go_workspace_test.go` with tests for Go workspace (TS-01-11 through TS-01-13)
    - Create `tests/setup/skeleton_test.go` with tests for binary exit behavior (TS-01-14 through TS-01-17)
    - _Test Spec: TS-01-11 through TS-01-17_

  - [x] 1.3 Write proto and infrastructure tests
    - Create `tests/setup/proto_test.go` with tests for proto files and generation (TS-01-18 through TS-01-23)
    - Create `tests/setup/infra_test.go` with tests for compose file and infrastructure (TS-01-24 through TS-01-28)
    - _Test Spec: TS-01-18 through TS-01-28_

  - [x] 1.4 Write Makefile and test infrastructure tests
    - Create `tests/setup/makefile_test.go` with tests for Makefile targets (TS-01-29 through TS-01-34)
    - Create `tests/setup/test_infra_test.go` with tests for test runner discovery (TS-01-35 through TS-01-38)
    - _Test Spec: TS-01-29 through TS-01-38_

  - [x] 1.5 Write edge case and property tests
    - Create `tests/setup/edge_cases_test.go` with edge case tests (TS-01-E1 through TS-01-E9)
    - Create `tests/setup/property_test.go` with property tests (TS-01-P1 through TS-01-P8)
    - _Test Spec: TS-01-E1 through TS-01-E9, TS-01-P1 through TS-01-P8_

  - [x] 1.V Verify task group 1
    - [x] All spec tests exist and are syntactically valid: `cd tests/setup && go vet ./...`
    - [x] All spec tests FAIL (red) — no implementation yet: `cd tests/setup && go test -v ./... 2>&1 | grep FAIL`
    - [x] No linter warnings: `cd tests/setup && go vet ./...`

- [x] 2. Directory structure and placeholder directories
  - [x] 2.1 Create top-level directories
    - Create directories: `rhivos/`, `backend/`, `android/`, `mobile/`, `mock/`, `proto/`, `deployments/`, `tests/`
    - _Requirements: 01-REQ-1.1_

  - [x] 2.2 Create Rust component subdirectories
    - Create: `rhivos/locking-service/src/`, `rhivos/cloud-gateway-client/src/`, `rhivos/update-service/src/`, `rhivos/parking-operator-adaptor/src/`, `rhivos/mock-sensors/src/bin/`
    - _Requirements: 01-REQ-1.2_

  - [x] 2.3 Create Go and mock subdirectories
    - Create: `backend/parking-fee-service/`, `backend/cloud-gateway/`, `mock/parking-app-cli/`, `mock/companion-app-cli/`, `mock/parking-operator/`
    - _Requirements: 01-REQ-1.3, 01-REQ-1.4_

  - [x] 2.4 Create placeholder directories with READMEs
    - Create `android/README.md` with placeholder text for AAOS PARKING_APP
    - Create `mobile/README.md` with placeholder text for Flutter COMPANION_APP
    - _Requirements: 01-REQ-1.5, 01-REQ-1.6_

  - [x] 2.V Verify task group 2
    - [x] Directory structure tests pass: `cd tests/setup && go test -v -run 'TestTopLevel|TestRust.*Dir|TestGo.*Dir|TestMock.*Dir|TestPlaceholder' ./...`
    - [x] _Test Spec: TS-01-1 through TS-01-6_

- [x] 3. Rust workspace and skeletons
  - [x] 3.1 Create Cargo workspace root
    - Create `rhivos/Cargo.toml` with `[workspace]` defining members: `locking-service`, `cloud-gateway-client`, `update-service`, `parking-operator-adaptor`, `mock-sensors`
    - _Requirements: 01-REQ-2.1_

  - [x] 3.2 Create Rust skeleton crates
    - Create `Cargo.toml` and `src/main.rs` for: locking-service, cloud-gateway-client, update-service, parking-operator-adaptor
    - Each `main.rs` prints usage with component name and exits 0
    - Each crate includes a `#[test] fn it_compiles()` test
    - _Requirements: 01-REQ-4.1, 01-REQ-4.3, 01-REQ-4.E1_

  - [x] 3.3 Create mock-sensors crate with binary targets
    - Create `rhivos/mock-sensors/Cargo.toml` with `[[bin]]` entries for location-sensor, speed-sensor, door-sensor
    - Create `src/bin/location-sensor.rs`, `src/bin/speed-sensor.rs`, `src/bin/door-sensor.rs`
    - Create `src/lib.rs` with shared stub code
    - Each binary prints usage and exits 0
    - _Requirements: 01-REQ-2.4, 01-REQ-4.1, 01-REQ-4.3_

  - [x] 3.4 Verify Rust workspace builds and tests pass
    - Run `cd rhivos && cargo build` — must exit 0
    - Run `cd rhivos && cargo test` — must discover and pass tests for all crates
    - Run `cd rhivos && cargo clippy -- -D warnings` — must pass
    - _Requirements: 01-REQ-2.2, 01-REQ-2.3, 01-REQ-8.1_

  - [x] 3.V Verify task group 3
    - [x] Rust workspace tests pass: `cd tests/setup && go test -v -run 'TestCargo|TestRustSkeleton|TestRustBinary' ./...`
    - [x] All existing tests still pass: `cd tests/setup && go test -v ./...`
    - [x] No linter warnings: `cd rhivos && cargo clippy -- -D warnings`
    - [x] _Test Spec: TS-01-7 through TS-01-10, TS-01-14, TS-01-16_

- [ ] 4. Go workspace and skeletons
  - [ ] 4.1 Create Go modules
    - Create `backend/go.mod` (module `github.com/rhadp/parking-fee-service/backend`)
    - Create `mock/go.mod` (module `github.com/rhadp/parking-fee-service/mock`)
    - Create `go.work` at repo root with `use` directives for `./backend`, `./mock`, `./tests/setup`
    - _Requirements: 01-REQ-3.1_

  - [ ] 4.2 Create Go skeleton binaries
    - Create `main.go` for: `backend/parking-fee-service`, `backend/cloud-gateway`, `mock/parking-app-cli`, `mock/companion-app-cli`, `mock/parking-operator`
    - Each prints usage with component name and exits 0
    - _Requirements: 01-REQ-4.2, 01-REQ-4.4, 01-REQ-4.E1_

  - [ ] 4.3 Create Go placeholder tests
    - Create `main_test.go` for each Go binary with a trivial `TestMain` test
    - _Requirements: 01-REQ-8.2_

  - [ ] 4.4 Verify Go workspace builds and tests pass
    - Run `go build ./...` — must exit 0
    - Run `go test ./...` — must discover and pass tests
    - Run `go vet ./...` — must pass
    - _Requirements: 01-REQ-3.2, 01-REQ-3.3, 01-REQ-8.2_

  - [ ] 4.V Verify task group 4
    - [ ] Go workspace tests pass: `cd tests/setup && go test -v -run 'TestGoWork|TestGoSkeleton|TestGoBinary' ./...`
    - [ ] All existing tests still pass: `cd tests/setup && go test -v ./...`
    - [ ] No linter warnings: `go vet ./...`
    - [ ] _Test Spec: TS-01-11 through TS-01-13, TS-01-15, TS-01-17_

- [ ] 5. Checkpoint - Workspaces Complete
  - Ensure `cargo build`, `cargo test`, `go build ./...`, `go test ./...` all pass
  - All skeleton binaries exit 0 with usage messages
  - Ask the user if questions arise

- [ ] 6. Protocol Buffer definitions and code generation
  - [ ] 6.1 Create proto files
    - Create `proto/common.proto` with AdapterState, AdapterInfo, ErrorDetails (as specified in design.md)
    - Create `proto/update_service.proto` with UpdateService and all 5 RPCs
    - Create `proto/parking_adaptor.proto` with ParkingAdaptor and all 4 RPCs
    - _Requirements: 01-REQ-5.1, 01-REQ-5.2, 01-REQ-5.3, 01-REQ-5.4_

  - [ ] 6.2 Create proto generation script
    - Add `proto` target to Makefile (or create a separate script) that runs `protoc` with Go plugins
    - Generate code into `gen/go/commonpb/`, `gen/go/updateservicepb/`, `gen/go/parkingadaptorpb/`
    - Include `protoc` availability check with error message
    - _Requirements: 01-REQ-5.5, 01-REQ-5.E1_

  - [ ] 6.3 Generate and verify Go code
    - Run `make proto` and verify generated code compiles
    - Add `gen/go/` module or ensure it's importable by backend and mock modules
    - _Requirements: 01-REQ-5.5, 01-REQ-5.6_

  - [ ] 6.V Verify task group 6
    - [ ] Proto tests pass: `cd tests/setup && go test -v -run 'TestProto' ./...`
    - [ ] All existing tests still pass: `cd tests/setup && go test -v ./...`
    - [ ] No linter warnings: `go vet ./...`
    - [ ] _Test Spec: TS-01-18 through TS-01-23_

- [ ] 7. Local infrastructure
  - [ ] 7.1 Create Podman Compose configuration
    - Create `deployments/compose.yml` with NATS (:4222) and Kuksa Databroker (:55556)
    - Create `deployments/nats/nats-server.conf` with port 4222 configuration
    - _Requirements: 01-REQ-6.1, 01-REQ-6.4_

  - [ ] 7.2 Create VSS overlay file
    - Create `deployments/vss-overlay.json` with custom signals: Vehicle.Parking.SessionActive, Vehicle.Command.Door.Lock, Vehicle.Command.Door.Response
    - _Requirements: 01-REQ-6.5_

  - [ ] 7.3 Add infra-up and infra-down to Makefile
    - Add `infra-up` target: `podman compose -f deployments/compose.yml up -d`
    - Add `infra-down` target: `podman compose -f deployments/compose.yml down`
    - Include Podman availability check
    - _Requirements: 01-REQ-6.2, 01-REQ-6.3, 01-REQ-6.E1_

  - [ ] 7.V Verify task group 7
    - [ ] Infrastructure tests pass: `cd tests/setup && go test -v -run 'TestCompose|TestNATS|TestVSS|TestInfra' ./...`
    - [ ] All existing tests still pass: `cd tests/setup && go test -v ./...`
    - [ ] _Test Spec: TS-01-24 through TS-01-28_

- [ ] 8. Root Makefile and final integration
  - [ ] 8.1 Complete root Makefile
    - Add `build` target (cargo build + go build)
    - Add `test` target (cargo test + go test)
    - Add `lint` target (cargo clippy + go vet)
    - Add `check` target (build + test + lint)
    - Add `clean` target (cargo clean + go clean + rm artifacts)
    - Include toolchain availability checks
    - _Requirements: 01-REQ-7.1, 01-REQ-7.2, 01-REQ-7.3, 01-REQ-7.4, 01-REQ-7.5, 01-REQ-7.6, 01-REQ-7.E1_

  - [ ] 8.2 Verify all Makefile targets
    - Run `make build` — must exit 0
    - Run `make test` — must exit 0, output both Rust and Go test results
    - Run `make lint` — must exit 0
    - Run `make check` — must exit 0
    - Run `make clean` — must remove build artifacts
    - _Requirements: 01-REQ-7.1 through 01-REQ-7.6, 01-REQ-8.4_

  - [ ] 8.V Verify task group 8
    - [ ] Makefile tests pass: `cd tests/setup && go test -v -run 'TestMakefile|TestMakeTarget' ./...`
    - [ ] All existing tests still pass: `cd tests/setup && go test -v ./...`
    - [ ] No linter warnings: `make lint`
    - [ ] _Test Spec: TS-01-29 through TS-01-34, TS-01-38_

- [ ] 9. Edge cases and property verification
  - [ ] 9.1 Verify edge case tests
    - Run edge case tests: `cd tests/setup && go test -v -run 'TestEdge' ./...`
    - Fix any failures in the implementation
    - _Test Spec: TS-01-E1 through TS-01-E9_

  - [ ] 9.2 Verify property tests
    - Run property tests: `cd tests/setup && go test -v -run 'TestProperty' ./...`
    - Fix any failures in the implementation
    - _Test Spec: TS-01-P1 through TS-01-P8_

  - [ ] 9.3 Update README.md
    - Update README.md to match the PRD-authoritative directory structure
    - Ensure directory names match: `deployments/`, `android/`, `mobile/`
    - Ensure documented Makefile targets match actual implementation

  - [ ] 9.V Verify task group 9
    - [ ] All edge case tests pass: `cd tests/setup && go test -v -run 'TestEdge' ./...`
    - [ ] All property tests pass: `cd tests/setup && go test -v -run 'TestProperty' ./...`
    - [ ] All tests pass: `make test && cd tests/setup && go test -v ./...`
    - [ ] No linter warnings: `make lint`
    - [ ] README accurately reflects implementation

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
| 01-REQ-1.1 | TS-01-1 | 2.1 | tests/setup/structure_test.go::TestTopLevelDirectories |
| 01-REQ-1.2 | TS-01-2 | 2.2 | tests/setup/structure_test.go::TestRustComponentDirs |
| 01-REQ-1.3 | TS-01-3 | 2.3 | tests/setup/structure_test.go::TestGoBackendDirs |
| 01-REQ-1.4 | TS-01-4 | 2.3 | tests/setup/structure_test.go::TestMockCliDirs |
| 01-REQ-1.5 | TS-01-5 | 2.4 | tests/setup/structure_test.go::TestAaosPlaceholder |
| 01-REQ-1.6 | TS-01-6 | 2.4 | tests/setup/structure_test.go::TestFlutterPlaceholder |
| 01-REQ-1.E1 | TS-01-E1 | 2.1 | tests/setup/edge_cases_test.go::TestEdgeMissingDir |
| 01-REQ-2.1 | TS-01-7 | 3.1 | tests/setup/rust_workspace_test.go::TestCargoWorkspaceMembers |
| 01-REQ-2.2 | TS-01-8 | 3.4 | tests/setup/rust_workspace_test.go::TestCargoBuild |
| 01-REQ-2.3 | TS-01-9 | 3.4 | tests/setup/rust_workspace_test.go::TestCargoTest |
| 01-REQ-2.4 | TS-01-10 | 3.3 | tests/setup/rust_workspace_test.go::TestMockSensorsBinaries |
| 01-REQ-2.E1 | TS-01-E2 | 3.1 | tests/setup/edge_cases_test.go::TestEdgeMissingCargoMember |
| 01-REQ-3.1 | TS-01-11 | 4.1 | tests/setup/go_workspace_test.go::TestGoWorkFile |
| 01-REQ-3.2 | TS-01-12 | 4.4 | tests/setup/go_workspace_test.go::TestGoBuild |
| 01-REQ-3.3 | TS-01-13 | 4.4 | tests/setup/go_workspace_test.go::TestGoTest |
| 01-REQ-3.E1 | TS-01-E3 | 4.1 | tests/setup/edge_cases_test.go::TestEdgeMissingGoMod |
| 01-REQ-4.1 | TS-01-14 | 3.2, 3.3 | tests/setup/skeleton_test.go::TestRustSkeletonExit |
| 01-REQ-4.2 | TS-01-15 | 4.2 | tests/setup/skeleton_test.go::TestGoSkeletonExit |
| 01-REQ-4.3 | TS-01-16 | 3.2, 3.3 | tests/setup/skeleton_test.go::TestRustBinaryList |
| 01-REQ-4.4 | TS-01-17 | 4.2 | tests/setup/skeleton_test.go::TestGoBinaryList |
| 01-REQ-4.E1 | TS-01-E4 | 3.2, 4.2 | tests/setup/edge_cases_test.go::TestEdgeUnrecognizedFlag |
| 01-REQ-5.1 | TS-01-18 | 6.1 | tests/setup/proto_test.go::TestProtoFilesExist |
| 01-REQ-5.2 | TS-01-19 | 6.1 | tests/setup/proto_test.go::TestCommonProtoTypes |
| 01-REQ-5.3 | TS-01-20 | 6.1 | tests/setup/proto_test.go::TestUpdateServiceProtoRPCs |
| 01-REQ-5.4 | TS-01-21 | 6.1 | tests/setup/proto_test.go::TestParkingAdaptorProtoRPCs |
| 01-REQ-5.5 | TS-01-22 | 6.2, 6.3 | tests/setup/proto_test.go::TestProtoGeneration |
| 01-REQ-5.6 | TS-01-23 | 6.3 | tests/setup/proto_test.go::TestGeneratedGoCompiles |
| 01-REQ-5.E1 | TS-01-E5 | 6.2 | tests/setup/edge_cases_test.go::TestEdgeMissingProtoc |
| 01-REQ-6.1 | TS-01-24 | 7.1 | tests/setup/infra_test.go::TestComposeFileServices |
| 01-REQ-6.2 | TS-01-25 | 7.3 | tests/setup/infra_test.go::TestInfraUp |
| 01-REQ-6.3 | TS-01-26 | 7.3 | tests/setup/infra_test.go::TestInfraDown |
| 01-REQ-6.4 | TS-01-27 | 7.1 | tests/setup/infra_test.go::TestNatsConfigExists |
| 01-REQ-6.5 | TS-01-28 | 7.2 | tests/setup/infra_test.go::TestVssOverlayExists |
| 01-REQ-6.E1 | TS-01-E6 | 7.3 | tests/setup/edge_cases_test.go::TestEdgeMissingPodman |
| 01-REQ-6.E2 | TS-01-E7 | 7.3 | tests/setup/edge_cases_test.go::TestEdgeIdempotentInfraUp |
| 01-REQ-7.1 | TS-01-29 | 8.1 | tests/setup/makefile_test.go::TestMakeBuild |
| 01-REQ-7.2 | TS-01-30 | 8.1 | tests/setup/makefile_test.go::TestMakeTest |
| 01-REQ-7.3 | TS-01-31 | 8.1 | tests/setup/makefile_test.go::TestMakeLint |
| 01-REQ-7.4 | TS-01-32 | 8.1 | tests/setup/makefile_test.go::TestMakeCheck |
| 01-REQ-7.5 | TS-01-33 | 8.1 | tests/setup/makefile_test.go::TestMakeClean |
| 01-REQ-7.6 | TS-01-34 | 8.1 | tests/setup/makefile_test.go::TestMakefileTargetsDefined |
| 01-REQ-7.E1 | TS-01-E8 | 8.1 | tests/setup/edge_cases_test.go::TestEdgeMissingToolchain |
| 01-REQ-8.1 | TS-01-35 | 3.2, 3.3 | tests/setup/test_infra_test.go::TestCargoTestDiscovery |
| 01-REQ-8.2 | TS-01-36 | 4.3 | tests/setup/test_infra_test.go::TestGoTestDiscovery |
| 01-REQ-8.3 | TS-01-37 | 1.1 | tests/setup/test_infra_test.go::TestSetupTestsModule |
| 01-REQ-8.4 | TS-01-38 | 8.1 | tests/setup/test_infra_test.go::TestMakeTestRunsAll |
| Property 1 | TS-01-P1 | 3.1-3.4 | tests/setup/property_test.go::TestPropertyRustCompleteness |
| Property 2 | TS-01-P2 | 4.1-4.4 | tests/setup/property_test.go::TestPropertyGoCompleteness |
| Property 3 | TS-01-P3 | 3.2, 4.2 | tests/setup/property_test.go::TestPropertySkeletonExit |
| Property 4 | TS-01-P4 | 6.2, 6.3 | tests/setup/property_test.go::TestPropertyProtoIdempotency |
| Property 5 | TS-01-P5 | 7.3 | tests/setup/property_test.go::TestPropertyInfraIdempotency |
| Property 6 | TS-01-P6 | 2.1-2.4 | tests/setup/property_test.go::TestPropertyDirCompleteness |
| Property 7 | TS-01-P7 | 3.2, 4.3 | tests/setup/property_test.go::TestPropertyTestDiscovery |
| Property 8 | TS-01-P8 | 6.1 | tests/setup/property_test.go::TestPropertyProtoServiceCompleteness |

## Notes

- All tests live in `tests/setup/` as a standalone Go module. They use `os/exec` to run shell commands and verify structural invariants.
- Edge case tests that modify the filesystem (rename/remove files) must restore the original state in cleanup, regardless of test pass/fail. Use `t.Cleanup()`.
- Infrastructure tests (TS-01-25, TS-01-26, TS-01-E7, TS-01-P5) require Podman to be running. Mark these with a build tag or skip condition (`testing.Short()`).
- Proto generation tests require `protoc`, `protoc-gen-go`, and `protoc-gen-go-grpc`. Skip if unavailable.
- The `gen/go/` directory is not checked into git — it's regenerated by `make proto`. The Go workspace should handle this via a `go.mod` in `gen/go/` or by vendoring.
