# Implementation Plan: Repository Setup

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
- Proto files are the source of truth — write them first, then generate bindings
- Rust workspace must compile before skeleton services are added
- Go proto generation must succeed before Go modules reference generated packages
-->

## Overview

This plan establishes the project foundation in dependency order:

1. Directory structure and build scaffolding first (everything else depends on
   paths existing).
2. Proto definitions next (all services depend on shared contracts).
3. Rust workspace and proto crate (Rust services depend on generated bindings).
4. Rust skeleton services (depend on proto crate).
5. Go proto generation and backend skeletons (depend on proto files).
6. Mock CLI tools (depend on both Rust workspace and Go proto generation).
7. Local infrastructure (independent but needed for integration).
8. Container definitions and final end-to-end verification.

## Test Commands

- Rust unit tests: `cd rhivos && cargo test --workspace`
- Go unit tests (per module): `cd backend/parking-fee-service && go test ./...`
- All tests: `make test`
- Rust linter: `cd rhivos && cargo clippy --workspace -- -D warnings`
- Go linter: `cd backend/parking-fee-service && go vet ./...`
- All linters: `make lint`
- Build all: `make build`
- Proto generation: `make proto`
- Infrastructure: `make infra-up` / `make infra-down`
- Containers: `make build-containers`

## Tasks

- [ ] 1. Project Foundation
  - [ ] 1.1 Create directory structure
    - Create all directories as specified in the design document project layout
    - Add `.gitkeep` files in placeholder directories: `android/parking-app/`,
      `android/companion-app/`, `docs/`, `tests/`
    - _Requirements: 01-REQ-1.1, 01-REQ-1.2, 01-REQ-1.3, 01-REQ-1.4,
      01-REQ-1.5, 01-REQ-1.6, 01-REQ-1.7_

  - [ ] 1.2 Create `scripts/check-tools.sh`
    - Check for: `cargo`, `go`, `protoc`, `protoc-gen-go`, `protoc-gen-go-grpc`,
      `podman` (or `docker`)
    - Print version of each found tool; exit non-zero with message for any missing
    - _Requirements: 01-REQ-5.E1_

  - [ ] 1.3 Create root `Makefile` skeleton
    - Define all target names from the design document (build, test, proto, lint,
      clean, infra-up, infra-down, infra-status, build-containers)
    - Each target initially prints "not yet implemented" and exits 0
    - Include a `check-tools` target that runs `scripts/check-tools.sh`
    - Wire `check-tools` as a dependency of `build`, `test`, `proto`
    - _Requirements: 01-REQ-5.1 through 01-REQ-5.6_

  - [ ] 1.4 Write structure verification test
    - Create `tests/test_structure.sh` that asserts all required directories exist
    - **Validates: Property 7 (Directory Completeness)**
    - _Requirements: 01-REQ-1.1 through 01-REQ-1.7_

  - [ ] 1.V Verify task group 1
    - [ ] `tests/test_structure.sh` passes
    - [ ] `make check-tools` runs without error (given tools are installed)
    - [ ] All Makefile targets are callable (print placeholder message)
    - [ ] Requirements 01-REQ-1.1–1.7, 01-REQ-5.E1 acceptance criteria met

- [ ] 2. Proto Definitions
  - [ ] 2.1 Write `proto/common/common.proto`
    - Define: `Location`, `VehicleId`, `AdapterInfo`, `AdapterState` enum,
      `ErrorDetails`
    - Use `proto3` syntax, set `go_package`
    - Follow the design document proto specification exactly
    - _Requirements: 01-REQ-4.3, 01-REQ-4.6_

  - [ ] 2.2 Write `proto/services/update_service.proto`
    - Define `UpdateService` with RPCs: `InstallAdapter`, `WatchAdapterStates`
      (server streaming), `ListAdapters`, `RemoveAdapter`, `GetAdapterStatus`
    - Define all request/response messages
    - Import `common/common.proto`
    - _Requirements: 01-REQ-4.1, 01-REQ-4.6_

  - [ ] 2.3 Write `proto/services/parking_adapter.proto`
    - Define `ParkingAdapter` with RPCs: `StartSession`, `StopSession`,
      `GetStatus`, `GetRate`
    - Define all request/response messages
    - Import `common/common.proto`
    - _Requirements: 01-REQ-4.2, 01-REQ-4.6_

  - [ ] 2.4 Set up Go proto generation
    - Create `proto/gen/go/` output directory
    - Add `make proto` target that invokes `protoc` with `--go_out` and
      `--go-grpc_out` for all `.proto` files
    - Verify generated Go packages compile
    - _Requirements: 01-REQ-4.4, 01-REQ-5.3_

  - [ ] 2.5 Write proto syntax validation test
    - Create `tests/test_proto.sh` that runs `protoc` in lint/check mode on all
      `.proto` files
    - **Validates: Property 2 (Proto-Binding Consistency)**
    - _Requirements: 01-REQ-4.4, 01-REQ-4.5_

  - [ ] 2.V Verify task group 2
    - [ ] `protoc` compiles all `.proto` files without errors
    - [ ] `make proto` generates Go packages under `proto/gen/go/`
    - [ ] Generated Go packages compile: `cd proto/gen/go && go build ./...`
      (or equivalent)
    - [ ] `tests/test_proto.sh` passes
    - [ ] Requirements 01-REQ-4.1–4.6 acceptance criteria met

- [ ] 3. Checkpoint — Proto Definitions Complete
  - All proto files written, Go generation working
  - Commit and verify clean state before proceeding to Rust workspace

- [ ] 4. Rust Workspace and Proto Crate
  - [ ] 4.1 Create `rhivos/Cargo.toml` workspace manifest
    - List initial members: `parking-proto`
    - Define `[workspace.dependencies]` for shared crate versions (tonic, prost,
      tokio, clap, tracing)
    - Use resolver = "2"
    - _Requirements: 01-REQ-2.1_

  - [ ] 4.2 Create `rhivos/parking-proto` crate
    - `Cargo.toml`: depend on `tonic` and `prost` from workspace
    - `build.rs`: invoke `tonic_build::configure().compile_protos()` on all
      `.proto` files under `../../proto/`
    - `src/lib.rs`: re-export generated modules via `tonic::include_proto!()`
    - _Requirements: 01-REQ-4.5_

  - [ ] 4.3 Verify proto crate compilation
    - Run `cd rhivos && cargo build -p parking-proto`
    - Fix any path or import issues in `build.rs`
    - Verify generated types are accessible (e.g., `parking_proto::common::Location`)
    - **Validates: Property 2 (Proto-Binding Consistency)**
    - _Requirements: 01-REQ-4.5_

  - [ ] 4.V Verify task group 4
    - [ ] `cd rhivos && cargo build -p parking-proto` succeeds
    - [ ] `cd rhivos && cargo test -p parking-proto` succeeds (even if no
      tests yet)
    - [ ] Generated Rust types match proto definitions
    - [ ] Requirements 01-REQ-2.1, 01-REQ-4.5 acceptance criteria met

- [ ] 5. Rust Skeleton Services
  - [ ] 5.1 Create `locking-service` skeleton
    - `Cargo.toml`: depend on `parking-proto`, `tonic`, `tokio`, `clap`,
      `tracing`, `tracing-subscriber`
    - `src/main.rs`: parse `--listen-addr` flag (default `0.0.0.0:50051`),
      start tonic gRPC server, log listen address, handle SIGINT/SIGTERM
    - No service RPCs registered (locking-service uses Kuksa proto, not
      parking-proto; it is a client, not a server in this spec)
    - Add crate to workspace members
    - _Requirements: 01-REQ-7.1, 01-REQ-2.4_

  - [ ] 5.2 Create `cloud-gateway-client` skeleton
    - Same pattern as 5.1 with default port `0.0.0.0:50052`
    - No RPCs served (it is an MQTT client, not a gRPC server)
    - Add crate to workspace members
    - _Requirements: 01-REQ-7.1, 01-REQ-2.4_

  - [ ] 5.3 Create `update-service` skeleton
    - Same pattern as 5.1 with default port `0.0.0.0:50053`
    - Register `UpdateService` with stub handlers returning `UNIMPLEMENTED`
    - Add crate to workspace members
    - _Requirements: 01-REQ-7.1, 01-REQ-7.3, 01-REQ-7.4, 01-REQ-2.4_

  - [ ] 5.4 Create `parking-operator-adaptor` skeleton
    - Same pattern as 5.1 with default port `0.0.0.0:50054`
    - Register `ParkingAdapter` with stub handlers returning `UNIMPLEMENTED`
    - Add crate to workspace members
    - _Requirements: 01-REQ-7.1, 01-REQ-7.3, 01-REQ-7.4, 01-REQ-2.4_

  - [ ] 5.5 Write skeleton contract tests
    - For `update-service`: start server on random port, send each RPC, assert
      `UNIMPLEMENTED` status
    - For `parking-operator-adaptor`: same pattern
    - Place tests in each crate's `src/main.rs` as `#[cfg(test)]` module
    - **Property 3: Skeleton Contract**
    - **Validates: Requirements 01-REQ-7.4**
    - _Requirements: 01-REQ-2.3_

  - [ ] 5.V Verify task group 5
    - [ ] `cd rhivos && cargo build --workspace` succeeds
    - [ ] `cd rhivos && cargo test --workspace` succeeds (skeleton tests pass)
    - [ ] `cd rhivos && cargo clippy --workspace -- -D warnings` clean
    - [ ] Requirements 01-REQ-2.2, 01-REQ-2.3, 01-REQ-2.4, 01-REQ-7.1,
      01-REQ-7.3, 01-REQ-7.4 acceptance criteria met

- [ ] 6. Checkpoint — Rust Workspace Complete
  - All Rust crates compile, tests pass, clippy clean
  - Commit and verify clean state before proceeding to Go modules

- [ ] 7. Go Modules and Backend Skeletons
  - [ ] 7.1 Create `backend/parking-fee-service` Go module
    - `go.mod` with module path `github.com/rhadp/parking-fee-service/backend/parking-fee-service`
    - `main.go`: parse `--listen-addr` flag (default `:8080`), start
      `net/http` server with stub handlers returning HTTP 501, log listen
      address, handle OS signals for graceful shutdown
    - Register stub routes: `GET /healthz`, `POST /api/v1/operators/lookup`,
      `GET /api/v1/adapters/{id}`, `POST /api/v1/sessions`,
      `DELETE /api/v1/sessions/{id}`, `GET /api/v1/sessions/{id}/fee`
    - _Requirements: 01-REQ-3.1, 01-REQ-3.3, 01-REQ-7.2, 01-REQ-7.4_

  - [ ] 7.2 Create `backend/cloud-gateway` Go module
    - `go.mod`, `main.go` with same patterns as 7.1
    - Default port `:8081`
    - Register stub routes: `GET /healthz`,
      `POST /api/v1/vehicles/{vin}/lock`,
      `POST /api/v1/vehicles/{vin}/unlock`,
      `GET /api/v1/vehicles/{vin}/status`
    - _Requirements: 01-REQ-3.1, 01-REQ-3.3, 01-REQ-7.2, 01-REQ-7.4_

  - [ ] 7.3 Write Go skeleton tests
    - `backend/parking-fee-service/main_test.go`: start `httptest.Server`, send
      requests to each route, assert HTTP 501 response
    - `backend/cloud-gateway/main_test.go`: same pattern
    - **Property 3: Skeleton Contract**
    - **Validates: Requirements 01-REQ-7.4**
    - _Requirements: 01-REQ-3.4_

  - [ ] 7.4 Wire Go targets into root Makefile
    - `make build` compiles both Go services
    - `make test` runs `go test ./...` in each Go module directory
    - `make lint` runs `go vet ./...` in each Go module directory
    - _Requirements: 01-REQ-5.1, 01-REQ-5.2, 01-REQ-5.5_

  - [ ] 7.V Verify task group 7
    - [ ] `make build` builds both Go services
    - [ ] `make test` runs Go tests (skeleton tests pass)
    - [ ] `make lint` runs Go linter clean
    - [ ] Requirements 01-REQ-3.1–3.4, 01-REQ-7.2, 01-REQ-7.4 acceptance
      criteria met

- [ ] 8. Mock CLI Applications
  - [ ] 8.1 Create `mock/parking-app-cli`
    - Go module with `main.go` using a CLI framework (e.g., `cobra` or
      stdlib `flag` with subcommands)
    - Implement subcommands: `install-adapter`, `list-adapters`,
      `remove-adapter`, `adapter-status`, `watch-adapters`, `start-session`,
      `stop-session`, `get-status`, `get-rate`
    - Each subcommand constructs a gRPC request using generated Go stubs,
      sends it, prints the response or error
    - Global flags: `--update-service-addr`, `--adapter-addr`
    - _Requirements: 01-REQ-8.1, 01-REQ-8.3, 01-REQ-8.4_

  - [ ] 8.2 Create `mock/companion-app-cli`
    - Go module with `main.go`
    - Implement subcommands: `lock`, `unlock`, `status`
    - Each subcommand sends an HTTP request to the CLOUD_GATEWAY REST API
    - Global flags: `--gateway-addr`, `--vin`, `--token`
    - _Requirements: 01-REQ-8.2, 01-REQ-8.3_

  - [ ] 8.3 Create `mock/sensors` Rust crate
    - Add as workspace member in `rhivos/Cargo.toml` via `"../mock/sensors"`
    - `Cargo.toml`: depend on Kuksa client crate or vendored Kuksa proto, plus
      `clap`, `tokio`, `tonic`
    - `src/main.rs`: implement subcommands `set-location`, `set-speed`,
      `set-door` that publish to Kuksa Databroker via gRPC
    - Flag: `--databroker-addr` (default `localhost:55555`)
    - _Requirements: 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.3_

  - [ ] 8.4 Write mock CLI tests
    - `mock/parking-app-cli/main_test.go`: verify subcommand parsing and flag
      defaults
    - `mock/companion-app-cli/main_test.go`: verify subcommand parsing and flag
      defaults
    - `mock/sensors/src/main.rs` `#[cfg(test)]`: verify CLI argument parsing
    - **Property 5: Mock Interface Fidelity**
    - **Validates: Requirements 01-REQ-8.1, 01-REQ-8.4**
    - _Requirements: 01-REQ-8.1, 01-REQ-8.2, 01-REQ-9.1_

  - [ ] 8.5 Wire mock targets into root Makefile
    - `make build` includes mock CLI Go builds and mock-sensors Rust build
    - `make test` includes mock CLI tests
    - _Requirements: 01-REQ-5.1, 01-REQ-5.2_

  - [ ] 8.V Verify task group 8
    - [ ] `make build` builds all mock tools
    - [ ] `make test` runs and passes mock CLI tests
    - [ ] `mock/parking-app-cli --help` shows all subcommands
    - [ ] `mock/companion-app-cli --help` shows all subcommands
    - [ ] `mock-sensors --help` shows all subcommands
    - [ ] Requirements 01-REQ-8.1–8.4, 01-REQ-9.1–9.3 acceptance criteria met

- [ ] 9. Checkpoint — All Code Compiles
  - `make build` succeeds for all Rust and Go components
  - `make test` passes all tests
  - `make lint` is clean
  - Commit and verify clean state

- [ ] 10. Local Infrastructure
  - [ ] 10.1 Create `infra/compose.yaml`
    - Define `databroker` service using `ghcr.io/eclipse-kuksa/kuksa-databroker`
      image, port 55555
    - Define `mosquitto` service using `eclipse-mosquitto:2` image, port 1883
    - _Requirements: 01-REQ-6.1, 01-REQ-6.2, 01-REQ-6.3_

  - [ ] 10.2 Create infrastructure configuration files
    - `infra/config/mosquitto/mosquitto.conf`: listener 1883, allow_anonymous
    - Kuksa configuration if needed (VSS overlay, access control)
    - _Requirements: 01-REQ-6.5_

  - [ ] 10.3 Wire infrastructure targets into Makefile
    - `make infra-up`: `podman compose -f infra/compose.yaml up -d`
    - `make infra-down`: `podman compose -f infra/compose.yaml down`
    - `make infra-status`: `podman compose -f infra/compose.yaml ps`
    - _Requirements: 01-REQ-5.4_

  - [ ] 10.4 Write infrastructure smoke test
    - Create `tests/test_infra.sh` that:
      1. Runs `make infra-up`
      2. Waits for services to be ready (poll ports)
      3. Verifies Kuksa responds on port 55555 (e.g., `grpcurl` or `nc`)
      4. Verifies Mosquitto responds on port 1883 (e.g., `mosquitto_pub` or `nc`)
      5. Runs `make infra-down`
      6. Verifies containers are removed
    - **Property 4: Infrastructure Lifecycle Idempotency**
    - **Validates: Requirements 01-REQ-6.2, 01-REQ-6.3, 01-REQ-6.4**

  - [ ] 10.V Verify task group 10
    - [ ] `make infra-up` starts both containers
    - [ ] `make infra-status` shows both running
    - [ ] `make infra-down` stops and removes both containers
    - [ ] `tests/test_infra.sh` passes
    - [ ] Requirements 01-REQ-6.1–6.5 acceptance criteria met

- [ ] 11. Container Build Definitions
  - [ ] 11.1 Create Rust service Containerfiles
    - Multi-stage builds: `rust:1.75` builder stage → `debian:bookworm-slim`
      runtime stage
    - One Containerfile per Rust service in `containers/rhivos/`
    - Copy only the compiled binary into the runtime image
    - _Requirements: 01-REQ-10.1, 01-REQ-10.2_

  - [ ] 11.2 Create Go service Containerfiles
    - Multi-stage builds: `golang:1.22` builder stage → `gcr.io/distroless/static`
      or `debian:bookworm-slim` runtime stage
    - One Containerfile per Go service in `containers/backend/`
    - _Requirements: 01-REQ-10.1, 01-REQ-10.2_

  - [ ] 11.3 Create mock tool Containerfiles
    - Same patterns for mock CLIs and sensors in `containers/mock/`
    - _Requirements: 01-REQ-10.1, 01-REQ-10.2_

  - [ ] 11.4 Wire container build target into Makefile
    - `make build-containers`: iterate over all Containerfiles and build with
      `podman build`, tagging each as `{service-name}:latest`
    - _Requirements: 01-REQ-10.3, 01-REQ-10.4_

  - [ ] 11.5 Write container build test
    - Create `tests/test_containers.sh` that builds each image and verifies it
      starts (container runs, prints startup log, exits or binds to port)
    - **Property 6: Container Image Validity**
    - **Validates: Requirements 01-REQ-10.2, 01-REQ-10.4**

  - [ ] 11.V Verify task group 11
    - [ ] `make build-containers` completes without errors
    - [ ] Each image is tagged `{service-name}:latest`
    - [ ] `tests/test_containers.sh` passes
    - [ ] Requirements 01-REQ-10.1–10.4 acceptance criteria met

- [ ] 12. Final Verification and Documentation
  - [ ] 12.1 Complete root Makefile
    - Ensure all targets call real commands (no more "not yet implemented"
      placeholders)
    - Verify `make clean` removes all build artifacts
    - _Requirements: 01-REQ-5.6_

  - [ ] 12.2 Run full verification suite
    - `make clean && make proto && make build && make test && make lint`
    - Verify all pass with zero errors and zero warnings
    - **Property 1: Build Completeness**
    - **Validates: Requirements 01-REQ-2.2, 01-REQ-3.3, 01-REQ-5.1**

  - [ ] 12.3 Run `tests/test_structure.sh`
    - **Property 7: Directory Completeness**

  - [ ] 12.4 Update `README.md`
    - Ensure Quick Start section matches actual Makefile targets and project
      structure
    - Verify all referenced commands work as documented

  - [ ] 12.V Verify task group 12
    - [ ] `make build` succeeds
    - [ ] `make test` passes all tests
    - [ ] `make lint` is clean
    - [ ] `tests/test_structure.sh` passes
    - [ ] README commands match reality
    - [ ] All 01-REQ requirements have been verified

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Implemented By Task | Verified By Test |
|-------------|---------------------|------------------|
| 01-REQ-1.1 | 1.1 | `tests/test_structure.sh` |
| 01-REQ-1.2 | 1.1 | `tests/test_structure.sh` |
| 01-REQ-1.3 | 1.1 | `tests/test_structure.sh` |
| 01-REQ-1.4 | 1.1 | `tests/test_structure.sh` |
| 01-REQ-1.5 | 1.1 | `tests/test_structure.sh` |
| 01-REQ-1.6 | 1.1 | `tests/test_structure.sh` |
| 01-REQ-1.7 | 1.1 | `tests/test_structure.sh` |
| 01-REQ-1.E1 | 1.3 | `make build` with missing dir |
| 01-REQ-2.1 | 4.1 | `cargo build --workspace` |
| 01-REQ-2.2 | 5.V | `cargo build --workspace` |
| 01-REQ-2.3 | 5.V | `cargo test --workspace` |
| 01-REQ-2.4 | 5.1–5.4 | `ls rhivos/target/debug/` |
| 01-REQ-2.E1 | — | Standard Cargo behavior |
| 01-REQ-3.1 | 7.1, 7.2 | `go build ./...` per module |
| 01-REQ-3.2 | 8.1, 8.2 | `go build ./...` per module |
| 01-REQ-3.3 | 7.V | `make build` |
| 01-REQ-3.4 | 7.3 | `go test ./...` per module |
| 01-REQ-3.E1 | — | Standard Go behavior |
| 01-REQ-4.1 | 2.2 | `tests/test_proto.sh` |
| 01-REQ-4.2 | 2.3 | `tests/test_proto.sh` |
| 01-REQ-4.3 | 2.1 | `tests/test_proto.sh` |
| 01-REQ-4.4 | 2.4 | `make proto && go build proto/gen/go/...` |
| 01-REQ-4.5 | 4.2, 4.3 | `cargo build -p parking-proto` |
| 01-REQ-4.6 | 2.1–2.3 | `tests/test_proto.sh` |
| 01-REQ-4.E1 | — | Standard protoc behavior |
| 01-REQ-4.E2 | 2.4 | `make proto` regeneration |
| 01-REQ-5.1 | 7.4, 8.5, 12.1 | `make build` |
| 01-REQ-5.2 | 7.4, 8.5, 12.1 | `make test` |
| 01-REQ-5.3 | 2.4 | `make proto` |
| 01-REQ-5.4 | 10.3 | `make infra-up && make infra-down` |
| 01-REQ-5.5 | 7.4 | `make lint` |
| 01-REQ-5.6 | 12.1 | `make clean` |
| 01-REQ-5.E1 | 1.2 | `scripts/check-tools.sh` |
| 01-REQ-6.1 | 10.1 | `tests/test_infra.sh` |
| 01-REQ-6.2 | 10.1 | `tests/test_infra.sh` |
| 01-REQ-6.3 | 10.1 | `tests/test_infra.sh` |
| 01-REQ-6.4 | 10.3 | `tests/test_infra.sh` |
| 01-REQ-6.5 | 10.2 | `tests/test_infra.sh` |
| 01-REQ-6.E1 | — | Standard container runtime behavior |
| 01-REQ-6.E2 | — | Standard container runtime behavior |
| 01-REQ-7.1 | 5.1–5.4 | `cargo test --workspace` |
| 01-REQ-7.2 | 7.1, 7.2 | `go test ./...` per module |
| 01-REQ-7.3 | 5.3, 5.4 | `cargo test --workspace` |
| 01-REQ-7.4 | 5.5, 7.3 | Skeleton contract tests |
| 01-REQ-7.E1 | 5.1–5.4 | Skeleton contract tests (bind failure) |
| 01-REQ-8.1 | 8.1 | `mock/parking-app-cli/main_test.go` |
| 01-REQ-8.2 | 8.2 | `mock/companion-app-cli/main_test.go` |
| 01-REQ-8.3 | 8.1, 8.2 | CLI tests (flag parsing) |
| 01-REQ-8.4 | 8.1 | `mock/parking-app-cli/main_test.go` |
| 01-REQ-8.E1 | 8.1, 8.2 | CLI tests (unreachable target) |
| 01-REQ-9.1 | 8.3 | `cargo test -p mock-sensors` |
| 01-REQ-9.2 | 8.3 | `cargo test -p mock-sensors` |
| 01-REQ-9.3 | 8.3 | `cargo test -p mock-sensors` |
| 01-REQ-9.E1 | 8.3 | `cargo test -p mock-sensors` |
| 01-REQ-10.1 | 11.1, 11.2, 11.3 | `tests/test_containers.sh` |
| 01-REQ-10.2 | 11.1, 11.2, 11.3 | `tests/test_containers.sh` |
| 01-REQ-10.3 | 11.4 | `make build-containers` |
| 01-REQ-10.4 | 11.4 | `podman images` |
| 01-REQ-10.E1 | — | Standard container runtime behavior |

## Notes

- **Session sizing:** Each numbered task group (1, 2, 4, 5, 7, 8, 10, 11, 12)
  is scoped for one coding session. Checkpoints (3, 6, 9) are commit-and-verify
  gates between phases.
- **Kuksa proto for mock-sensors:** The mock-sensors crate needs Kuksa's gRPC
  proto definitions. Use the `kuksa-client` Rust crate from crates.io if
  available, or vendor the proto files from the
  `eclipse-kuksa/kuksa-databroker` repository. Do not reimplement the Kuksa
  protocol.
- **Go proto generation is committed:** Generated Go files under `proto/gen/go/`
  are committed to the repository. This follows standard Go proto workflow and
  avoids requiring `protoc` for Go-only builds.
- **Skeleton services have no business logic:** All RPCs return UNIMPLEMENTED.
  Real implementations are in specs 02–05.
- **`locking-service` and `cloud-gateway-client` are not gRPC servers:** These
  services act as clients (to Kuksa and Mosquitto respectively). Their
  skeletons start a process that logs and waits, but do not register RPC
  handlers via parking-proto. Their gRPC interactions are with external systems
  (Kuksa, MQTT), not with parking-proto-defined services.
