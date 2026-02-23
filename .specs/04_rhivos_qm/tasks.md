# Implementation Plan: RHIVOS QM Partition (Phase 2.3)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the RHIVOS QM partition services: PARKING_OPERATOR_ADAPTOR
(Rust), UPDATE_SERVICE (Rust), mock PARKING_OPERATOR (Go), and mock
PARKING_APP CLI enhancements (Go). The approach is test-first: task group 1
creates failing tests that encode all 66 test contracts from `test_spec.md`.
Subsequent groups implement code to make those tests pass incrementally.

Ordering rationale:
1. Tests first (red) — establishes the verification baseline
2. Mock PARKING_OPERATOR (Go) — simplest component with no dependencies
3. PARKING_OPERATOR_ADAPTOR gRPC + REST client — depends on mock operator
4. PARKING_OPERATOR_ADAPTOR autonomous session management — depends on
   DATA_BROKER subscription and session manager
5. UPDATE_SERVICE core — adapter state machine, gRPC interface
6. UPDATE_SERVICE OCI + offloading — depends on core state machine
7. CLI enhancements and integration tests — depends on all services

## Test Commands

- Mock operator Go tests: `cd mock/parking-operator && go test -v -count=1 ./...`
- Rust unit tests (adaptor): `cd rhivos && cargo test -p parking-operator-adaptor`
- Rust unit tests (update-service): `cd rhivos && cargo test -p update-service`
- Rust all tests: `cd rhivos && cargo test`
- Integration tests (Rust): `cd rhivos && cargo test --test '*' -- --ignored`
- Go CLI tests: `cd mock/parking-app-cli && go test -v -count=1 ./...`
- Integration tests (all): `cd tests/integration && go test -v -count=1 ./...`
- Linter (Rust): `cd rhivos && cargo clippy -- -D warnings`
- Linter (Go): `cd mock/parking-operator && go vet ./... && cd ../parking-app-cli && go vet ./...`
- All tests: `make check`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Set up Rust integration test harness
    - Create `rhivos/tests/integration_helpers.rs` with helpers for starting
      and stopping services, gRPC client setup, DATA_BROKER interaction
    - Add helpers: `start_mock_operator`, `start_adaptor`, `start_update_service`,
      `grpc_connect`, `wait_for_port`, `publish_to_databroker`,
      `read_from_databroker`
    - Add `#[tokio::test]` and `#[ignore]` attributes for integration tests
      that require infrastructure
    - _Test Spec: all (shared infrastructure)_

  - [x] 1.2 Write mock PARKING_OPERATOR Go tests (unit)
    - Create `mock/parking-operator/main_test.go` with Go test functions
    - Translate TS-04-29 (configurable port) into Go test
    - Translate TS-04-30 through TS-04-33 (REST endpoints) into Go tests
    - Translate TS-04-E14, TS-04-E15, TS-04-E16 (error responses) into Go tests
    - Translate TS-04-P7 (fee accuracy) into Go property test
    - Group under `TestOperator_*` and `TestProperty_*` naming conventions
    - _Test Spec: TS-04-29, TS-04-30, TS-04-31, TS-04-32, TS-04-33,
      TS-04-E14, TS-04-E15, TS-04-E16, TS-04-P7_
    - _Requirements: 04-REQ-8.1, 04-REQ-8.2, 04-REQ-8.3, 04-REQ-8.4,
      04-REQ-8.5, 04-REQ-8.E1, 04-REQ-8.E2, 04-REQ-8.E3_

  - [x] 1.3 Write UPDATE_SERVICE Rust unit tests
    - Create Rust unit tests in `rhivos/update-service/src/` modules
    - Translate TS-04-22 (SHA-256 checksum verification) into Rust unit test
    - Translate TS-04-24 (configurable inactivity timeout) into Rust unit test
    - Translate TS-04-27, TS-04-28 (state machine transitions) into Rust
      unit tests
    - Translate TS-04-P4 (state machine integrity property) into Rust test
    - Group under `test_*` naming conventions in respective modules
    - _Test Spec: TS-04-22, TS-04-24, TS-04-27, TS-04-28, TS-04-P4_
    - _Requirements: 04-REQ-5.2, 04-REQ-6.1, 04-REQ-7.1, 04-REQ-7.2_

  - [x] 1.4 Write PARKING_OPERATOR_ADAPTOR + UPDATE_SERVICE Rust integration tests
    - Create `rhivos/tests/adaptor_integration.rs` for adaptor tests
    - Create `rhivos/tests/update_integration.rs` for update service tests
    - Translate TS-04-1 through TS-04-5 (gRPC interface) into Rust integration
      tests
    - Translate TS-04-6 through TS-04-10 (autonomous session) into Rust
      integration tests
    - Translate TS-04-11 through TS-04-14 (DATA_BROKER subscription) into Rust
      integration tests
    - Translate TS-04-15 through TS-04-20 (UPDATE_SERVICE gRPC) into Rust
      integration tests
    - Translate TS-04-E1 through TS-04-E13 (edge cases) into Rust integration
      tests
    - Translate TS-04-P1, TS-04-P2, TS-04-P3, TS-04-P5, TS-04-P6, TS-04-P8
      (property tests) into Rust integration tests
    - _Test Spec: TS-04-1 through TS-04-20, TS-04-E1 through TS-04-E13,
      TS-04-P1, TS-04-P2, TS-04-P3, TS-04-P5, TS-04-P6, TS-04-P8_
    - _Requirements: 04-REQ-1.1 through 04-REQ-6.3, all edge cases_

  - [x] 1.5 Write CLI and end-to-end integration tests
    - Translate TS-04-34 through TS-04-38 (CLI commands) into Go integration
      tests in `tests/integration/`
    - Translate TS-04-39, TS-04-40, TS-04-41 (end-to-end) into Go integration
      tests
    - Translate TS-04-E17 (CLI unreachable service) into Go test
    - Group under `TestCLI_*` and `TestE2E_*` naming conventions
    - _Test Spec: TS-04-34 through TS-04-41, TS-04-E17_
    - _Requirements: 04-REQ-9.1 through 04-REQ-9.5, 04-REQ-9.E1,
      04-REQ-10.1 through 04-REQ-10.3_

  - [x] 1.V Verify task group 1
    - [x] All Go test files are syntactically valid:
      `cd mock/parking-operator && go vet ./...`
    - [x] All Rust test files compile:
      `cd rhivos && cargo check --tests`
    - [x] Go unit tests FAIL (red) — no implementation yet:
      `cd mock/parking-operator && go test -count=1 ./... 2>&1 | grep -c FAIL`
    - [x] Rust unit tests FAIL (red) — no implementation yet
    - [x] No linter warnings introduced

- [x] 2. Mock PARKING_OPERATOR (Go)
  - [x] 2.1 Create Go module and in-memory store
    - Create `mock/parking-operator/go.mod`
      (module: `github.com/rhadp/parking-fee-service/mock/parking-operator`)
    - Create `mock/parking-operator/store.go` with `Session` and `Zone`
      structs, `SessionStore` with thread-safe in-memory map, and
      pre-configured zones (zone-munich-central: 2.50 EUR,
      zone-munich-west: 1.50 EUR)
    - _Requirements: 04-REQ-8.2, 04-REQ-8.3, 04-REQ-8.4, 04-REQ-8.5_

  - [x] 2.2 Implement HTTP handlers
    - Create `mock/parking-operator/handler.go` with:
      - `POST /parking/start` — creates session, returns session_id + status
      - `POST /parking/stop` — calculates fee, marks stopped, returns details
      - `GET /parking/{session_id}/status` — returns session status
      - `GET /rate/{zone_id}` — returns zone rate info
      - `GET /health` — returns `{"status": "ok"}`
    - Fee calculation: `rate_per_hour * (duration_seconds / 3600.0)`
    - Error responses: 404 for unknown session/zone with descriptive message
    - _Requirements: 04-REQ-8.1, 04-REQ-8.2, 04-REQ-8.3, 04-REQ-8.4,
      04-REQ-8.5, 04-REQ-8.E1, 04-REQ-8.E2, 04-REQ-8.E3_

  - [x] 2.3 Create main.go with configurable port
    - Create `mock/parking-operator/main.go` with HTTP server on configurable
      port (`PORT` env var, default 8090)
    - Register all routes
    - _Requirements: 04-REQ-8.1_

  - [x] 2.V Verify task group 2
    - [x] Mock operator Go tests pass:
      `cd mock/parking-operator && go test -v -count=1 ./...`
    - [x] All previously passing tests still pass
    - [x] No linter warnings: `cd mock/parking-operator && go vet ./...`
    - [x] Tests cover: TS-04-29, TS-04-30, TS-04-31, TS-04-32, TS-04-33,
      TS-04-E14, TS-04-E15, TS-04-E16, TS-04-P7

- [x] 3. PARKING_OPERATOR_ADAPTOR: gRPC service and REST client
  - [x] 3.1 Create crate structure and configuration
    - Create `rhivos/parking-operator-adaptor/Cargo.toml` with dependencies
      (tonic, prost, tokio, reqwest, serde, serde_json)
    - Create `rhivos/parking-operator-adaptor/build.rs` using tonic-build
      for `parking_adaptor.proto`
    - Create `rhivos/parking-operator-adaptor/src/config.rs` with env var
      configuration: `ADAPTOR_GRPC_ADDR`, `DATABROKER_ADDR`, `OPERATOR_URL`,
      `VEHICLE_ID`
    - _Requirements: 04-REQ-1.1_

  - [x] 3.2 Implement PARKING_OPERATOR REST client
    - Create `rhivos/parking-operator-adaptor/src/operator_client.rs`
    - Implement async REST calls using `reqwest`:
      - `start_session(vehicle_id, zone_id, timestamp)` -> `POST /parking/start`
      - `stop_session(session_id)` -> `POST /parking/stop`
      - `get_status(session_id)` -> `GET /parking/{session_id}/status`
      - `get_rate(zone_id)` -> `GET /rate/{zone_id}`
    - Return typed response structs with serde deserialization
    - Handle connection errors with descriptive error types
    - _Requirements: 04-REQ-1.2, 04-REQ-1.3, 04-REQ-1.4, 04-REQ-1.5,
      04-REQ-1.E3_

  - [x] 3.3 Implement ParkingAdaptor gRPC service
    - Create `rhivos/parking-operator-adaptor/src/grpc_service.rs`
    - Implement `ParkingAdaptor` trait from generated proto code:
      - `StartSession` — calls operator client, returns session_id + status
      - `StopSession` — calls operator client, returns fee + duration + currency
      - `GetStatus` — calls operator client, returns session state
      - `GetRate` — calls operator client, returns rate info
    - Return `ALREADY_EXISTS` if session active on StartSession
    - Return `NOT_FOUND` on StopSession with unknown session
    - Return `UNAVAILABLE` when operator unreachable
    - _Requirements: 04-REQ-1.1, 04-REQ-1.2, 04-REQ-1.3, 04-REQ-1.4,
      04-REQ-1.5, 04-REQ-1.E1, 04-REQ-1.E2, 04-REQ-1.E3_

  - [x] 3.4 Create session manager
    - Create `rhivos/parking-operator-adaptor/src/session_manager.rs`
    - Implement `SessionManager` with `active_session: Option<ActiveSession>`
      and `override_active: bool`
    - Methods: `start_session`, `stop_session`, `has_active_session`,
      `current_session_id`
    - Thread-safe via `Arc<Mutex<...>>`
    - _Requirements: 04-REQ-1.E1, 04-REQ-2.5_

  - [x] 3.5 Create main.rs entry point
    - Create `rhivos/parking-operator-adaptor/src/main.rs`
    - Start gRPC server on configured address
    - Initialize operator client, session manager
    - _Requirements: 04-REQ-1.1_

  - [x] 3.V Verify task group 3
    - [x] Adaptor gRPC integration tests pass (against mock operator):
      TS-04-1, TS-04-2, TS-04-3, TS-04-4, TS-04-5
    - [x] Edge case tests pass: TS-04-E1, TS-04-E2, TS-04-E3
    - [x] All previously passing tests still pass
    - [x] No linter warnings:
      `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`

- [ ] 4. PARKING_OPERATOR_ADAPTOR: autonomous session management
  - [ ] 4.1 Implement DATA_BROKER client
    - Create `rhivos/parking-operator-adaptor/src/databroker_client.rs`
    - Implement Kuksa DATA_BROKER gRPC client using `kuksa.val.v1` proto:
      - `subscribe(signal_path)` — opens streaming subscription
      - `read(signal_path)` — single get request
      - `write(signal_path, value)` — set request
    - Connect via network gRPC (TCP) at configurable address
    - Implement exponential backoff retry on connection failure
    - _Requirements: 04-REQ-3.1, 04-REQ-3.2, 04-REQ-3.3, 04-REQ-3.4,
      04-REQ-3.E1_

  - [ ] 4.2 Implement autonomous lock/unlock event handling
    - Add lock event subscription loop to main.rs or session_manager.rs
    - On lock event (`IsLocked = true`) + no active session:
      - Read location from DATA_BROKER
      - Call `POST /parking/start` on operator
      - Write `SessionActive = true` to DATA_BROKER
    - On unlock event (`IsLocked = false`) + active session:
      - Call `POST /parking/stop` on operator
      - Write `SessionActive = false` to DATA_BROKER
    - Ignore lock events when session already active
    - Ignore unlock events when no session active
    - On operator unreachable: log error, do NOT write SessionActive
    - _Requirements: 04-REQ-2.1, 04-REQ-2.2, 04-REQ-2.3, 04-REQ-2.4,
      04-REQ-2.E1, 04-REQ-2.E2, 04-REQ-2.E3_

  - [ ] 4.3 Integrate override with autonomous behavior
    - Update gRPC service to set `override_active` flag in session manager
    - Ensure manual StartSession/StopSession updates `SessionActive` in
      DATA_BROKER
    - Ensure autonomous behavior respects override state
    - _Requirements: 04-REQ-2.5_

  - [ ] 4.V Verify task group 4
    - [ ] Autonomous session tests pass (require DATA_BROKER):
      TS-04-6, TS-04-7, TS-04-8, TS-04-9, TS-04-10
    - [ ] DATA_BROKER subscription tests pass:
      TS-04-11, TS-04-12, TS-04-13, TS-04-14
    - [ ] Edge case tests pass: TS-04-E4, TS-04-E5, TS-04-E6, TS-04-E7
    - [ ] Property tests pass: TS-04-P1, TS-04-P2, TS-04-P3
    - [ ] All previously passing tests still pass
    - [ ] No linter warnings:
      `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`

- [ ] 5. UPDATE_SERVICE: core state machine and gRPC interface
  - [ ] 5.1 Create crate structure and configuration
    - Create `rhivos/update-service/Cargo.toml` with dependencies
      (tonic, prost, tokio, sha2, reqwest)
    - Create `rhivos/update-service/build.rs` using tonic-build for
      `update_service.proto`
    - Create `rhivos/update-service/src/config.rs` with env var
      configuration: `UPDATE_GRPC_ADDR`, `REGISTRY_URL`,
      `OFFLOAD_TIMEOUT_HOURS`, `CONTAINER_STORE_PATH`
    - _Requirements: 04-REQ-4.1, 04-REQ-6.1_

  - [ ] 5.2 Implement adapter state machine
    - Create `rhivos/update-service/src/adapter_manager.rs`
    - Implement `AdapterManager` with `HashMap<String, AdapterRecord>`
    - Implement valid state transition enforcement per 04-REQ-7.1
    - Reject invalid transitions with warning log per 04-REQ-7.2
    - Implement `broadcast::Sender<AdapterStateEvent>` for state event
      notifications
    - _Requirements: 04-REQ-7.1, 04-REQ-7.2_

  - [ ] 5.3 Implement checksum verification
    - Create `rhivos/update-service/src/checksum.rs`
    - Implement SHA-256 digest computation using `sha2` crate
    - Implement verification function comparing computed vs. provided checksum
    - _Requirements: 04-REQ-5.2_

  - [ ] 5.4 Implement UpdateService gRPC service
    - Create `rhivos/update-service/src/grpc_service.rs`
    - Implement `UpdateService` trait from generated proto code:
      - `InstallAdapter` — creates adapter record, starts async download,
        returns job_id + adapter_id + DOWNLOADING state
      - `WatchAdapterStates` — returns server-streaming response from
        broadcast receiver
      - `ListAdapters` — returns snapshot of all known adapters
      - `RemoveAdapter` — stops and removes adapter, returns success
      - `GetAdapterStatus` — returns single adapter info
    - Return `ALREADY_EXISTS` for duplicate installs
    - Return `NOT_FOUND` for unknown adapter IDs
    - _Requirements: 04-REQ-4.1, 04-REQ-4.2, 04-REQ-4.3, 04-REQ-4.4,
      04-REQ-4.5, 04-REQ-4.6, 04-REQ-4.E1, 04-REQ-4.E2_

  - [ ] 5.5 Create main.rs entry point
    - Create `rhivos/update-service/src/main.rs`
    - Start gRPC server on configured address
    - Initialize adapter manager
    - _Requirements: 04-REQ-4.1_

  - [ ] 5.V Verify task group 5
    - [ ] State machine unit tests pass: TS-04-27, TS-04-28, TS-04-P4
    - [ ] Checksum unit test passes: TS-04-22
    - [ ] Config unit test passes: TS-04-24
    - [ ] UPDATE_SERVICE gRPC integration tests pass:
      TS-04-15, TS-04-16, TS-04-17, TS-04-18, TS-04-19, TS-04-20
    - [ ] Edge case tests pass: TS-04-E8, TS-04-E9, TS-04-E10
    - [ ] All previously passing tests still pass
    - [ ] No linter warnings:
      `cd rhivos && cargo clippy -p update-service -- -D warnings`

- [ ] 6. UPDATE_SERVICE: OCI pulling, checksum gate, and offloading
  - [ ] 6.1 Implement OCI client
    - Create `rhivos/update-service/src/oci_client.rs`
    - Implement OCI manifest fetch: `GET /v2/{name}/manifests/{reference}`
    - Implement layer fetch: `GET /v2/{name}/blobs/{digest}`
    - Integrate with checksum verification after manifest pull
    - On checksum match: transition to INSTALLING
    - On checksum mismatch: transition to ERROR, discard image
    - On registry unreachable: transition to ERROR
    - _Requirements: 04-REQ-5.1, 04-REQ-5.2, 04-REQ-5.3, 04-REQ-5.E1,
      04-REQ-5.E2_

  - [ ] 6.2 Implement container runtime integration
    - Create `rhivos/update-service/src/container_runtime.rs`
    - Implement podman/crun CLI invocations via `std::process::Command`:
      create, start, stop, remove, inspect
    - Map container lifecycle events to adapter state transitions
    - Handle container start failure -> ERROR state with reason
    - _Requirements: 04-REQ-4.E3_

  - [ ] 6.3 Implement offloader
    - Create `rhivos/update-service/src/offloader.rs`
    - Implement background tokio task checking stopped adapters periodically
    - When `last_active.elapsed() > offload_timeout`: transition to
      OFFLOADING, remove container resources, then remove adapter
    - Emit `AdapterStateEvent` during offloading transitions
    - Handle re-install during OFFLOADING: cancel offload, re-download
    - _Requirements: 04-REQ-6.1, 04-REQ-6.2, 04-REQ-6.3, 04-REQ-6.E1_

  - [ ] 6.V Verify task group 6
    - [ ] OCI pull integration test passes: TS-04-21
    - [ ] Checksum gate integration tests pass: TS-04-23, TS-04-E11
    - [ ] Registry error test passes: TS-04-E12
    - [ ] Offloading tests pass: TS-04-25, TS-04-26, TS-04-E13
    - [ ] Property tests pass: TS-04-P5, TS-04-P6, TS-04-P8
    - [ ] All previously passing tests still pass
    - [ ] No linter warnings:
      `cd rhivos && cargo clippy -p update-service -- -D warnings`

- [ ] 7. CLI enhancements, integration tests, and final verification
  - [ ] 7.1 Implement CLI commands for UPDATE_SERVICE
    - Update `mock/parking-app-cli/` to add working `install`, `watch`, `list`
      commands
    - `install` — calls `InstallAdapter` with `--image-ref` and `--checksum`
      flags, prints response
    - `watch` — calls `WatchAdapterStates`, prints events until interrupted
    - `list` — calls `ListAdapters`, prints table of adapters
    - Error handling: print error message with target address, exit non-zero
    - _Requirements: 04-REQ-9.1, 04-REQ-9.2, 04-REQ-9.3, 04-REQ-9.E1_

  - [ ] 7.2 Implement CLI commands for PARKING_OPERATOR_ADAPTOR
    - Add working `start-session` and `stop-session` commands
    - `start-session` — calls `StartSession` with `--vehicle-id` and
      `--zone-id` flags, prints response
    - `stop-session` — calls `StopSession` with `--session-id` flag,
      prints response
    - Error handling: print error message with target address, exit non-zero
    - _Requirements: 04-REQ-9.4, 04-REQ-9.5, 04-REQ-9.E1_

  - [ ] 7.3 Run and fix integration tests
    - Run end-to-end integration tests: TS-04-39, TS-04-40, TS-04-41
    - Run CLI integration tests: TS-04-34, TS-04-35, TS-04-36, TS-04-37,
      TS-04-38
    - Run CLI error handling test: TS-04-E17
    - Fix any remaining failures
    - _Requirements: 04-REQ-10.1, 04-REQ-10.2, 04-REQ-10.3_

  - [ ] 7.4 Run full quality gate
    - `make check` (build + test + lint)
    - All 66 spec tests pass (41 acceptance + 17 edge + 8 property)
    - `git status` shows clean working tree

  - [ ] 7.V Verify task group 7
    - [ ] CLI tests pass: TS-04-34, TS-04-35, TS-04-36, TS-04-37, TS-04-38,
      TS-04-E17
    - [ ] Integration tests pass: TS-04-39, TS-04-40, TS-04-41
    - [ ] All 66 spec tests pass
    - [ ] `make check` exits 0
    - [ ] No linter warnings
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
| 04-REQ-1.1 | TS-04-1 | 3.1, 3.3, 3.5 | `TestAdaptor_GrpcService` |
| 04-REQ-1.2 | TS-04-2 | 3.2, 3.3 | `TestAdaptor_StartSession` |
| 04-REQ-1.3 | TS-04-3 | 3.2, 3.3 | `TestAdaptor_StopSession` |
| 04-REQ-1.4 | TS-04-4 | 3.2, 3.3 | `TestAdaptor_GetStatus` |
| 04-REQ-1.5 | TS-04-5 | 3.2, 3.3 | `TestAdaptor_GetRate` |
| 04-REQ-1.E1 | TS-04-E1 | 3.3, 3.4 | `TestEdge_StartSessionAlreadyActive` |
| 04-REQ-1.E2 | TS-04-E2 | 3.3 | `TestEdge_StopSessionUnknown` |
| 04-REQ-1.E3 | TS-04-E3 | 3.2, 3.3 | `TestEdge_StartSessionOperatorUnreachable` |
| 04-REQ-2.1 | TS-04-6 | 4.2 | `TestAutonomous_LockStartsSession` |
| 04-REQ-2.2 | TS-04-7 | 4.2 | `TestAutonomous_UnlockStopsSession` |
| 04-REQ-2.3 | TS-04-8 | 4.2 | `TestAutonomous_StartWritesSessionActive` |
| 04-REQ-2.4 | TS-04-9 | 4.2 | `TestAutonomous_StopWritesSessionActive` |
| 04-REQ-2.5 | TS-04-10 | 4.3 | `TestAutonomous_OverrideUpdatesSessionActive` |
| 04-REQ-2.E1 | TS-04-E4 | 4.2 | `TestEdge_UnlockNoSession` |
| 04-REQ-2.E2 | TS-04-E5 | 4.2 | `TestEdge_AutonomousStartOperatorUnreachable` |
| 04-REQ-2.E3 | TS-04-E6 | 4.2 | `TestEdge_LockWhileSessionActive` |
| 04-REQ-3.1 | TS-04-11 | 4.1 | `TestDatabroker_Connection` |
| 04-REQ-3.2 | TS-04-12 | 4.1 | `TestDatabroker_SubscribeIsLocked` |
| 04-REQ-3.3 | TS-04-13 | 4.1 | `TestDatabroker_ReadLocation` |
| 04-REQ-3.4 | TS-04-14 | 4.1 | `TestDatabroker_WriteSessionActive` |
| 04-REQ-3.E1 | TS-04-E7 | 4.1 | `TestEdge_DatabrokerUnreachableRetry` |
| 04-REQ-4.1 | TS-04-15 | 5.1, 5.4, 5.5 | `TestUpdateService_GrpcService` |
| 04-REQ-4.2 | TS-04-16 | 5.4 | `TestUpdateService_InstallAdapter` |
| 04-REQ-4.3 | TS-04-17 | 5.4 | `TestUpdateService_WatchAdapterStates` |
| 04-REQ-4.4 | TS-04-18 | 5.4 | `TestUpdateService_ListAdapters` |
| 04-REQ-4.5 | TS-04-19 | 5.4 | `TestUpdateService_RemoveAdapter` |
| 04-REQ-4.6 | TS-04-20 | 5.4 | `TestUpdateService_GetAdapterStatus` |
| 04-REQ-4.E1 | TS-04-E8 | 5.4 | `TestEdge_InstallAlreadyInstalled` |
| 04-REQ-4.E2 | TS-04-E9 | 5.4 | `TestEdge_RemoveUnknownAdapter` |
| 04-REQ-4.E3 | TS-04-E10 | 6.2 | `TestEdge_ContainerStartFailure` |
| 04-REQ-5.1 | TS-04-21 | 6.1 | `TestOCI_ImagePull` |
| 04-REQ-5.2 | TS-04-22 | 5.3 | `test_checksum_verification` |
| 04-REQ-5.3 | TS-04-23 | 6.1 | `TestOCI_ChecksumMatchTransition` |
| 04-REQ-5.E1 | TS-04-E11 | 6.1 | `TestEdge_ChecksumMismatch` |
| 04-REQ-5.E2 | TS-04-E12 | 6.1 | `TestEdge_RegistryUnreachable` |
| 04-REQ-6.1 | TS-04-24 | 5.1 | `test_configurable_offload_timeout` |
| 04-REQ-6.2 | TS-04-25 | 6.3 | `TestOffloading_StoppedAdapterOffloaded` |
| 04-REQ-6.3 | TS-04-26 | 6.3 | `TestOffloading_EmitsStateEvents` |
| 04-REQ-6.E1 | TS-04-E13 | 6.3 | `TestEdge_ReinstallDuringOffloading` |
| 04-REQ-7.1 | TS-04-27 | 5.2 | `test_valid_state_transitions` |
| 04-REQ-7.2 | TS-04-28 | 5.2 | `test_invalid_state_transitions` |
| 04-REQ-8.1 | TS-04-29 | 2.3 | `TestOperator_ConfigurablePort` |
| 04-REQ-8.2 | TS-04-30 | 2.2 | `TestOperator_StartSession` |
| 04-REQ-8.3 | TS-04-31 | 2.2 | `TestOperator_StopSession` |
| 04-REQ-8.4 | TS-04-32 | 2.2 | `TestOperator_SessionStatus` |
| 04-REQ-8.5 | TS-04-33 | 2.2 | `TestOperator_ZoneRate` |
| 04-REQ-8.E1 | TS-04-E14 | 2.2 | `TestEdge_StopUnknownSession404` |
| 04-REQ-8.E2 | TS-04-E15 | 2.2 | `TestEdge_StatusUnknownSession404` |
| 04-REQ-8.E3 | TS-04-E16 | 2.2 | `TestEdge_RateUnknownZone404` |
| 04-REQ-9.1 | TS-04-34 | 7.1 | `TestCLI_Install` |
| 04-REQ-9.2 | TS-04-35 | 7.1 | `TestCLI_Watch` |
| 04-REQ-9.3 | TS-04-36 | 7.1 | `TestCLI_List` |
| 04-REQ-9.4 | TS-04-37 | 7.2 | `TestCLI_StartSession` |
| 04-REQ-9.5 | TS-04-38 | 7.2 | `TestCLI_StopSession` |
| 04-REQ-9.E1 | TS-04-E17 | 7.1, 7.2 | `TestEdge_CLIServiceUnreachable` |
| 04-REQ-10.1 | TS-04-39 | 7.3 | `TestE2E_LockToSession` |
| 04-REQ-10.2 | TS-04-40 | 7.3 | `TestE2E_CLIToUpdateService` |
| 04-REQ-10.3 | TS-04-41 | 7.3 | `TestE2E_AdaptorToOperator` |
| Property 1 | TS-04-P1 | 4.2 | `TestProperty_SessionStateConsistency` |
| Property 2 | TS-04-P2 | 4.2 | `TestProperty_AutonomousIdempotency` |
| Property 3 | TS-04-P3 | 4.3 | `TestProperty_OverridePrecedence` |
| Property 4 | TS-04-P4 | 5.2 | `test_property_state_machine_integrity` |
| Property 5 | TS-04-P5 | 6.1 | `TestProperty_ChecksumGate` |
| Property 6 | TS-04-P6 | 6.3 | `TestProperty_OffloadingCorrectness` |
| Property 7 | TS-04-P7 | 2.2 | `TestProperty_FeeAccuracy` |
| Property 8 | TS-04-P8 | 5.4, 6.3 | `TestProperty_EventStreamCompleteness` |

## Notes

- **Test implementation languages:** Rust integration tests use `#[tokio::test]`
  with `#[ignore]` for tests requiring infrastructure. Go tests use standard
  `testing` package with `net/http/httptest` for HTTP handler tests. End-to-end
  integration tests are in `tests/integration/` as a standalone Go module.
- **Infrastructure requirements:** Integration tests for autonomous session
  management (task group 4) require DATA_BROKER running (`make infra-up`).
  Tests tagged with `#[ignore]` can be run with `cargo test -- --ignored`.
- **Mock PARKING_OPERATOR testing:** Go unit tests for the mock operator use
  `httptest.NewServer()` for isolated handler testing without port allocation.
- **State machine testing:** Unit tests for the adapter state machine (task
  group 5) do NOT require infrastructure. They test the `AdapterManager`
  directly with in-memory state.
- **OCI registry mocking:** Integration tests for OCI pulling (task group 6)
  use a mock HTTP server simulating the OCI distribution API
  (`/v2/.../manifests/...`, `/v2/.../blobs/...`).
- **Session sizing:** Each task group is scoped for one coding session.
  Task group 1 (failing tests) and task group 2 (mock operator) are the
  smallest. Task groups 3-4 (adaptor) and 5-6 (update service) are
  medium-sized. Task group 7 (CLI + integration) is the largest.
- **Offloading timeout for tests:** Integration tests for offloading should
  use a very short timeout (2-3 seconds) to avoid slow test execution.
