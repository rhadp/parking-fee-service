# Implementation Plan: RHIVOS QM Partition (Phase 2.3)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the RHIVOS QM partition services for the SDV Parking Demo
System. The approach is test-first: task group 1 creates failing tests that
encode all test contracts from `test_spec.md`. Subsequent groups build the
actual implementations to make those tests pass incrementally.

Ordering rationale:
1. Tests first (red) — establishes the verification baseline
2. Mock PARKING_OPERATOR (Go) — dependency for adaptor testing
3. PARKING_OPERATOR_ADAPTOR (Rust) — core service, depends on mock operator
4. UPDATE_SERVICE (Rust) — adapter lifecycle, depends on proto and container
   runtime
5. Mock PARKING_APP CLI enhancements (Go) — depends on both services running
6. Integration testing — depends on all components

## Test Commands

- Mock PARKING_OPERATOR unit tests: `cd mock/parking-operator && go test -v -count=1 ./...`
- PARKING_OPERATOR_ADAPTOR unit tests: `cd rhivos && cargo test -p parking-operator-adaptor`
- UPDATE_SERVICE unit tests: `cd rhivos && cargo test -p update-service`
- All Rust tests: `cd rhivos && cargo test`
- Mock CLI tests: `cd mock/parking-app-cli && go test -v -count=1 ./...`
- Integration tests: `cd tests/integration && go test -v -count=1 -tags=integration ./...`
- Linter (Rust): `cd rhivos && cargo clippy -- -D warnings`
- Linter (Go): `go vet ./mock/... ./backend/...`
- All tests: `make check`

## Tasks

- [ ] 1. Write failing tests
  - [ ] 1.1 Set up test infrastructure for mock PARKING_OPERATOR
    - Create `mock/parking-operator/main_test.go` with test helpers
    - Write handler tests for `POST /parking/start` (TS-04-29)
    - Write handler tests for `POST /parking/stop` (TS-04-30)
    - Write handler tests for `GET /parking/{session_id}/status` (TS-04-31)
    - Write handler tests for `GET /rate/{zone_id}` (TS-04-32)
    - Write edge case tests for unknown session stop (TS-04-E14)
    - Write edge case tests for unknown session status (TS-04-E15)
    - Write edge case tests for unknown zone rate (TS-04-E16)
    - Write property test for fee accuracy (TS-04-P7)
    - _Test Spec: TS-04-29 through TS-04-32, TS-04-E14 through TS-04-E16, TS-04-P7_

  - [ ] 1.2 Write PARKING_OPERATOR_ADAPTOR unit tests
    - Create `rhivos/parking-operator-adaptor/src/session_manager.rs` test module
    - Write tests for StartSession returning session_id (TS-04-2)
    - Write tests for StopSession returning fee (TS-04-3)
    - Write tests for GetStatus returning state (TS-04-4)
    - Write tests for GetRate returning zone rate (TS-04-5)
    - Write tests for manual override updates SessionActive (TS-04-10)
    - Write edge case: StartSession while active (TS-04-E1)
    - Write edge case: StopSession unknown session (TS-04-E2)
    - Write edge case: operator unreachable (TS-04-E3)
    - Write edge case: unlock with no session (TS-04-E4)
    - Write edge case: duplicate lock event (TS-04-E6)
    - Write property tests: session state consistency (TS-04-P1),
      autonomous idempotency (TS-04-P2), override precedence (TS-04-P3)
    - _Test Spec: TS-04-2 through TS-04-5, TS-04-10, TS-04-E1 through
      TS-04-E4, TS-04-E6, TS-04-P1 through TS-04-P3_

  - [ ] 1.3 Write UPDATE_SERVICE unit tests
    - Create test modules in `rhivos/update-service/src/`
    - Write tests for InstallAdapter returning DOWNLOADING (TS-04-16)
    - Write tests for WatchAdapterStates streaming (TS-04-17)
    - Write tests for ListAdapters (TS-04-18)
    - Write tests for RemoveAdapter (TS-04-19)
    - Write tests for GetAdapterStatus (TS-04-20)
    - Write tests for checksum verification (TS-04-22)
    - Write tests for DOWNLOADING -> INSTALLING transition (TS-04-23)
    - Write tests for offloading timeout (TS-04-24, TS-04-25)
    - Write tests for offloading events (TS-04-26)
    - Write tests for valid state transitions (TS-04-27)
    - Write tests for invalid state transitions (TS-04-28)
    - Write edge cases: already installed (TS-04-E8), unknown adapter
      (TS-04-E9), container failure (TS-04-E10), checksum mismatch
      (TS-04-E11), registry unreachable (TS-04-E12), re-install during
      offload (TS-04-E13)
    - Write property tests: state machine integrity (TS-04-P4), checksum gate
      (TS-04-P5), offloading correctness (TS-04-P6), event stream
      completeness (TS-04-P8)
    - _Test Spec: TS-04-16 through TS-04-28, TS-04-E8 through TS-04-E13,
      TS-04-P4 through TS-04-P6, TS-04-P8_

  - [ ] 1.4 Write integration test stubs
    - Create `tests/integration/qm_test.go` (or extend existing)
    - Write test stubs for DATA_BROKER lock-to-session (TS-04-38)
    - Write test stubs for CLI-to-UpdateService lifecycle (TS-04-39)
    - Write test stubs for adaptor-to-operator REST (TS-04-40)
    - Write test stubs for adaptor gRPC service (TS-04-1)
    - Write test stubs for UPDATE_SERVICE gRPC service (TS-04-15)
    - Write test stubs for lock/unlock events (TS-04-6 through TS-04-9,
      TS-04-11 through TS-04-14)
    - Write test stubs for CLI commands (TS-04-33 through TS-04-37)
    - Write edge case: operator unreachable during autonomous start (TS-04-E5)
    - Write edge case: DATA_BROKER unreachable (TS-04-E7)
    - Write edge case: CLI unreachable service (TS-04-E17)
    - _Test Spec: TS-04-1, TS-04-6 through TS-04-9, TS-04-11 through
      TS-04-15, TS-04-33 through TS-04-40, TS-04-E5, TS-04-E7, TS-04-E17_

  - [ ] 1.V Verify task group 1
    - [ ] All test files compile without syntax errors:
      `cd mock/parking-operator && go vet ./...`
      `cd rhivos && cargo check -p parking-operator-adaptor -p update-service`
    - [ ] All tests FAIL (red) — no implementation yet
    - [ ] No linter warnings introduced

- [ ] 2. Mock PARKING_OPERATOR (Go)
  - [ ] 2.1 Create mock module structure
    - Create `mock/parking-operator/go.mod`
      (module: `github.com/rhadp/parking-fee-service/mock/parking-operator`)
    - Create `mock/parking-operator/main.go` with HTTP server setup
    - Add to `go.work` if not already present
    - _Requirements: 04-REQ-8.1_

  - [ ] 2.2 Implement in-memory store
    - Create `mock/parking-operator/store.go`
    - Implement `Session` and `Zone` types
    - Implement session CRUD operations
    - Pre-configure zones: `zone-munich-central` (EUR 2.50/hr),
      `zone-munich-west` (EUR 1.50/hr)
    - _Requirements: 04-REQ-8.2 through 04-REQ-8.5_

  - [ ] 2.3 Implement REST handlers
    - Create `mock/parking-operator/handler.go`
    - `POST /parking/start`: create session, return session_id + status
    - `POST /parking/stop`: calculate fee, mark stopped, return response
    - `GET /parking/{session_id}/status`: return session state
    - `GET /rate/{zone_id}`: return zone rate
    - `GET /health`: return `{"status": "ok"}`
    - Implement fee calculation: `rate_per_hour * (duration_seconds / 3600.0)`
    - _Requirements: 04-REQ-8.1 through 04-REQ-8.5_

  - [ ] 2.4 Implement error handling
    - Return HTTP 404 for unknown session_id (stop, status)
    - Return HTTP 404 for unknown zone_id (rate)
    - Return HTTP 400 for malformed request bodies
    - _Requirements: 04-REQ-8.E1 through 04-REQ-8.E3_

  - [ ] 2.V Verify task group 2
    - [ ] Mock operator unit tests pass:
      `cd mock/parking-operator && go test -v -count=1 ./...`
    - [ ] Fee accuracy property test passes (TS-04-P7)
    - [ ] Edge case tests pass (TS-04-E14 through TS-04-E16)
    - [ ] No linter warnings: `go vet ./mock/parking-operator/...`
    - [ ] Requirements 04-REQ-8.1 through 04-REQ-8.5,
      04-REQ-8.E1 through 04-REQ-8.E3 met

- [ ] 3. PARKING_OPERATOR_ADAPTOR (Rust)
  - [ ] 3.1 Implement operator REST client
    - Create `rhivos/parking-operator-adaptor/src/operator_client.rs`
    - Implement `start_session(vehicle_id, zone_id, timestamp) -> Result<StartResponse>`
    - Implement `stop_session(session_id) -> Result<StopResponse>`
    - Implement `get_status(session_id) -> Result<StatusResponse>`
    - Implement `get_rate(zone_id) -> Result<RateResponse>`
    - Use `reqwest` for HTTP calls with serde JSON (de)serialization
    - Handle connection errors (return appropriate errors)
    - _Requirements: 04-REQ-1.2 through 04-REQ-1.5, 04-REQ-1.E3_

  - [ ] 3.2 Implement session manager
    - Create `rhivos/parking-operator-adaptor/src/session_manager.rs`
    - Implement `SessionManager` struct with active session state
    - Handle lock event: start session if none active (autonomous)
    - Handle unlock event: stop session if active (autonomous)
    - Handle manual StartSession: start regardless of lock state (override)
    - Handle manual StopSession: stop regardless of lock state (override)
    - Idempotency: ignore duplicate lock/unlock events
    - _Requirements: 04-REQ-2.1 through 04-REQ-2.5, 04-REQ-2.E1 through
      04-REQ-2.E3_

  - [ ] 3.3 Implement DATA_BROKER client
    - Create `rhivos/parking-operator-adaptor/src/databroker_client.rs`
    - Implement subscribe to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
    - Implement read for location signals
    - Implement write for `Vehicle.Parking.SessionActive`
    - Handle connection retry with exponential backoff
    - Use Kuksa Databroker gRPC API (network TCP)
    - _Requirements: 04-REQ-3.1 through 04-REQ-3.4, 04-REQ-3.E1_

  - [ ] 3.4 Implement gRPC service
    - Create `rhivos/parking-operator-adaptor/src/grpc_service.rs`
    - Implement `ParkingAdaptor` trait (replace stub from Phase 1.2)
    - Wire StartSession, StopSession to session manager
    - Wire GetStatus, GetRate to operator client
    - Return appropriate gRPC error codes for edge cases
    - _Requirements: 04-REQ-1.1 through 04-REQ-1.5, 04-REQ-1.E1 through
      04-REQ-1.E3_

  - [ ] 3.5 Implement main entry point
    - Update `rhivos/parking-operator-adaptor/src/main.rs`
    - Parse configuration from environment variables
    - Start gRPC server
    - Start DATA_BROKER subscription task
    - Wire session manager to DATA_BROKER events and operator client
    - _Requirements: 04-REQ-1.1, 04-REQ-3.1_

  - [ ] 3.6 Implement configuration
    - Create `rhivos/parking-operator-adaptor/src/config.rs`
    - `ADAPTOR_GRPC_ADDR` (default: `0.0.0.0:50052`)
    - `DATABROKER_ADDR` (default: `localhost:55555`)
    - `OPERATOR_URL` (default: `http://localhost:8090`)
    - `VEHICLE_ID` (default: `VIN12345`)

  - [ ] 3.V Verify task group 3
    - [ ] Adaptor unit tests pass:
      `cd rhivos && cargo test -p parking-operator-adaptor`
    - [ ] Session manager property tests pass (TS-04-P1 through TS-04-P3)
    - [ ] Edge case tests pass (TS-04-E1 through TS-04-E4, TS-04-E6)
    - [ ] No linter warnings:
      `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] Requirements 04-REQ-1.*, 04-REQ-2.*, 04-REQ-3.* met

- [ ] 4. UPDATE_SERVICE (Rust)
  - [ ] 4.1 Implement adapter state machine
    - Create `rhivos/update-service/src/adapter_manager.rs`
    - Implement `AdapterManager` with HashMap of adapters
    - Implement valid state transition enforcement (04-REQ-7.1)
    - Reject invalid transitions with warning log (04-REQ-7.2)
    - Implement broadcast channel for state events
    - _Requirements: 04-REQ-7.1, 04-REQ-7.2_

  - [ ] 4.2 Implement checksum verification
    - Create `rhivos/update-service/src/checksum.rs`
    - Implement SHA-256 computation over OCI manifest bytes
    - Implement comparison against provided checksum
    - Return error with details on mismatch
    - _Requirements: 04-REQ-5.2, 04-REQ-5.E1_

  - [ ] 4.3 Implement OCI client
    - Create `rhivos/update-service/src/oci_client.rs`
    - Implement manifest fetch from OCI registry
    - Implement blob/layer download
    - Handle registry connection errors
    - Integrate with checksum verification
    - _Requirements: 04-REQ-5.1, 04-REQ-5.3, 04-REQ-5.E2_

  - [ ] 4.4 Implement container runtime interface
    - Create `rhivos/update-service/src/container_runtime.rs`
    - Implement container create, start, stop, remove via podman CLI
    - Handle container start failures
    - Map container events to adapter state transitions
    - _Requirements: 04-REQ-4.E3_

  - [ ] 4.5 Implement offloader
    - Create `rhivos/update-service/src/offloader.rs`
    - Background tokio task checking stopped adapters periodically
    - Configurable inactivity timeout (default 24h)
    - Transition STOPPED -> OFFLOADING -> removal
    - Cancel offload if re-install requested
    - Emit state events during offloading
    - _Requirements: 04-REQ-6.1 through 04-REQ-6.3, 04-REQ-6.E1_

  - [ ] 4.6 Implement gRPC service
    - Create `rhivos/update-service/src/grpc_service.rs`
    - Implement `UpdateService` trait (replace stub from Phase 1.2)
    - InstallAdapter: initiate async download, return DOWNLOADING
    - WatchAdapterStates: server-streaming from broadcast channel
    - ListAdapters: snapshot of adapter map
    - RemoveAdapter: stop + remove container + remove from map
    - GetAdapterStatus: lookup in adapter map
    - Return appropriate gRPC error codes
    - _Requirements: 04-REQ-4.1 through 04-REQ-4.6, 04-REQ-4.E1 through
      04-REQ-4.E3_

  - [ ] 4.7 Implement main entry point and configuration
    - Update `rhivos/update-service/src/main.rs`
    - Create `rhivos/update-service/src/config.rs`
    - Parse configuration from env vars
    - Start gRPC server, offloader task
    - Wire adapter manager, OCI client, container runtime

  - [ ] 4.V Verify task group 4
    - [ ] UPDATE_SERVICE unit tests pass:
      `cd rhivos && cargo test -p update-service`
    - [ ] State machine property test passes (TS-04-P4)
    - [ ] Checksum gate property test passes (TS-04-P5)
    - [ ] Offloading property test passes (TS-04-P6)
    - [ ] Event stream property test passes (TS-04-P8)
    - [ ] Edge case tests pass (TS-04-E8 through TS-04-E13)
    - [ ] No linter warnings:
      `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] Requirements 04-REQ-4.* through 04-REQ-7.* met

- [ ] 5. Mock PARKING_APP CLI Enhancements (Go)
  - [ ] 5.1 Implement `install` command
    - Update `mock/parking-app-cli/` install command
    - Add `--image-ref` and `--checksum` flags
    - Create gRPC client connection to UPDATE_SERVICE
    - Call InstallAdapter, print response
    - Handle connection errors
    - _Requirements: 04-REQ-9.1_

  - [ ] 5.2 Implement `watch` command
    - Update watch command
    - Create streaming gRPC client to UPDATE_SERVICE
    - Print each AdapterStateEvent as received
    - Handle Ctrl+C gracefully
    - _Requirements: 04-REQ-9.2_

  - [ ] 5.3 Implement `list` command
    - Update list command
    - Call ListAdapters, print table of adapters
    - _Requirements: 04-REQ-9.3_

  - [ ] 5.4 Implement `start-session` command
    - Update start-session command
    - Add `--vehicle-id` and `--zone-id` flags
    - Create gRPC client to PARKING_OPERATOR_ADAPTOR
    - Call StartSession, print response
    - _Requirements: 04-REQ-9.4_

  - [ ] 5.5 Implement `stop-session` command
    - Update stop-session command
    - Add `--session-id` flag
    - Call StopSession, print response
    - _Requirements: 04-REQ-9.5_

  - [ ] 5.6 Add error handling for unreachable services
    - Print descriptive error with target address
    - Exit with non-zero code
    - _Requirements: 04-REQ-9.E1_

  - [ ] 5.V Verify task group 5
    - [ ] CLI commands compile and build:
      `cd mock/parking-app-cli && go build ./...`
    - [ ] CLI unit/integration tests pass:
      `cd mock/parking-app-cli && go test -v -count=1 ./...`
    - [ ] Edge case: unreachable service test passes (TS-04-E17)
    - [ ] No linter warnings: `go vet ./mock/parking-app-cli/...`
    - [ ] Requirements 04-REQ-9.* met

- [ ] 6. Integration testing
  - [ ] 6.1 Set up integration test harness
    - Create/update `tests/integration/qm_test.go`
    - Add test helpers for starting/stopping services (mock operator,
      adaptor, update-service)
    - Add helpers for DATA_BROKER interaction (set/get signals)
    - Add helpers for waiting on conditions with timeouts
    - _Test Spec: shared infrastructure for TS-04-38 through TS-04-40_

  - [ ] 6.2 Implement lock-to-session integration test
    - Start DATA_BROKER, mock PARKING_OPERATOR, PARKING_OPERATOR_ADAPTOR
    - Set location in DATA_BROKER
    - Set IsLocked = true
    - Verify SessionActive = true
    - Verify mock operator received start request
    - Set IsLocked = false
    - Verify SessionActive = false
    - Verify mock operator received stop request
    - _Test Spec: TS-04-38, TS-04-6 through TS-04-9, TS-04-11 through TS-04-14_

  - [ ] 6.3 Implement CLI-to-UpdateService integration test
    - Start UPDATE_SERVICE
    - Run install command via CLI binary
    - Run list command, verify adapter appears
    - _Test Spec: TS-04-39, TS-04-33 through TS-04-35_

  - [ ] 6.4 Implement adaptor-to-operator integration test
    - Start mock PARKING_OPERATOR and PARKING_OPERATOR_ADAPTOR
    - Call StartSession via gRPC
    - Verify mock operator received start request
    - Call StopSession via gRPC
    - Verify mock operator received stop request
    - _Test Spec: TS-04-40, TS-04-1, TS-04-36, TS-04-37_

  - [ ] 6.5 Implement edge case integration tests
    - Test operator unreachable during autonomous start (TS-04-E5)
    - Test DATA_BROKER unreachable at startup (TS-04-E7)
    - Test CLI error on unreachable service (TS-04-E17)
    - _Test Spec: TS-04-E5, TS-04-E7, TS-04-E17_

  - [ ] 6.6 Update Makefile for new components
    - Add mock PARKING_OPERATOR to build targets
    - Add integration test targets for QM partition tests
    - Ensure `make test` includes new Go tests
    - Ensure `make lint` covers new code

  - [ ] 6.V Verify task group 6
    - [ ] All integration tests pass:
      `cd tests/integration && go test -v -count=1 -tags=integration ./...`
    - [ ] All unit tests still pass: `make test`
    - [ ] All linters pass: `make lint`
    - [ ] Requirements 04-REQ-10.1 through 04-REQ-10.3 met
    - [ ] All 57 test spec entries covered (40 acceptance + 17 edge + 8 property)

- [ ] 7. Final verification and cleanup
  - [ ] 7.1 Run all tests and fix failures
    - Run full test suite: `make check`
    - Run all spec tests: unit, integration, property, edge case
    - Fix any remaining failures

  - [ ] 7.2 Update documentation
    - Update Makefile help text for new targets
    - Ensure all environment variables are documented in design.md

  - [ ] 7.3 Run full quality gate
    - `make check` (build + test + lint)
    - Integration tests pass
    - `git status` shows clean working tree

  - [ ] 7.V Verify task group 7
    - [ ] All tests pass (unit, integration, property, edge case)
    - [ ] `make check` exits 0
    - [ ] No linter warnings
    - [ ] All changes committed and pushed
    - [ ] Feature branch merged to develop

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
| 04-REQ-1.1 | TS-04-1 | 3.4, 3.5 | `TestAdaptorGrpcService` |
| 04-REQ-1.2 | TS-04-2 | 3.1, 3.4 | `TestStartSessionReturnsSessionId` |
| 04-REQ-1.3 | TS-04-3 | 3.1, 3.4 | `TestStopSessionReturnsFee` |
| 04-REQ-1.4 | TS-04-4 | 3.1, 3.4 | `TestGetStatusReturnsState` |
| 04-REQ-1.5 | TS-04-5 | 3.1, 3.4 | `TestGetRateReturnsZoneRate` |
| 04-REQ-1.E1 | TS-04-E1 | 3.2, 3.4 | `TestStartSessionAlreadyActive` |
| 04-REQ-1.E2 | TS-04-E2 | 3.2, 3.4 | `TestStopSessionUnknown` |
| 04-REQ-1.E3 | TS-04-E3 | 3.1, 3.4 | `TestOperatorUnreachable` |
| 04-REQ-2.1 | TS-04-6 | 3.2, 3.3 | `TestLockEventStartsSession` |
| 04-REQ-2.2 | TS-04-7 | 3.2, 3.3 | `TestUnlockEventStopsSession` |
| 04-REQ-2.3 | TS-04-8 | 3.2, 3.3 | `TestSessionActiveSetOnStart` |
| 04-REQ-2.4 | TS-04-9 | 3.2, 3.3 | `TestSessionActiveClearedOnStop` |
| 04-REQ-2.5 | TS-04-10 | 3.2, 3.4 | `TestManualOverride` |
| 04-REQ-2.E1 | TS-04-E4 | 3.2 | `TestUnlockNoSession` |
| 04-REQ-2.E2 | TS-04-E5 | 3.2 | `TestOperatorUnreachableAutonomous` |
| 04-REQ-2.E3 | TS-04-E6 | 3.2 | `TestDuplicateLockIgnored` |
| 04-REQ-3.1 | TS-04-11 | 3.3, 3.5 | `TestDataBrokerConnection` |
| 04-REQ-3.2 | TS-04-12 | 3.3 | `TestDataBrokerSubscription` |
| 04-REQ-3.3 | TS-04-13 | 3.3 | `TestDataBrokerLocationRead` |
| 04-REQ-3.4 | TS-04-14 | 3.3 | `TestDataBrokerSessionWrite` |
| 04-REQ-3.E1 | TS-04-E7 | 3.3 | `TestDataBrokerRetry` |
| 04-REQ-4.1 | TS-04-15 | 4.6, 4.7 | `TestUpdateServiceGrpc` |
| 04-REQ-4.2 | TS-04-16 | 4.1, 4.6 | `TestInstallAdapterDownloading` |
| 04-REQ-4.3 | TS-04-17 | 4.1, 4.6 | `TestWatchAdapterStatesStream` |
| 04-REQ-4.4 | TS-04-18 | 4.1, 4.6 | `TestListAdapters` |
| 04-REQ-4.5 | TS-04-19 | 4.1, 4.4, 4.6 | `TestRemoveAdapter` |
| 04-REQ-4.6 | TS-04-20 | 4.1, 4.6 | `TestGetAdapterStatus` |
| 04-REQ-4.E1 | TS-04-E8 | 4.1, 4.6 | `TestInstallAlreadyInstalled` |
| 04-REQ-4.E2 | TS-04-E9 | 4.1, 4.6 | `TestRemoveUnknownAdapter` |
| 04-REQ-4.E3 | TS-04-E10 | 4.4, 4.6 | `TestContainerStartFailure` |
| 04-REQ-5.1 | TS-04-21 | 4.3 | `TestOciImagePull` |
| 04-REQ-5.2 | TS-04-22 | 4.2 | `TestChecksumVerification` |
| 04-REQ-5.3 | TS-04-23 | 4.2, 4.3 | `TestChecksumPassTransition` |
| 04-REQ-5.E1 | TS-04-E11 | 4.2, 4.3 | `TestChecksumMismatchError` |
| 04-REQ-5.E2 | TS-04-E12 | 4.3 | `TestRegistryUnreachable` |
| 04-REQ-6.1 | TS-04-24 | 4.5 | `TestOffloadTimeout` |
| 04-REQ-6.2 | TS-04-25 | 4.5 | `TestOffloadRemovesResources` |
| 04-REQ-6.3 | TS-04-26 | 4.5 | `TestOffloadEmitsEvents` |
| 04-REQ-6.E1 | TS-04-E13 | 4.5 | `TestReinstallDuringOffload` |
| 04-REQ-7.1 | TS-04-27 | 4.1 | `TestValidStateTransitions` |
| 04-REQ-7.2 | TS-04-28 | 4.1 | `TestInvalidStateTransitions` |
| 04-REQ-8.1 | TS-04-29 | 2.1, 2.3 | `TestMockOperatorServer` |
| 04-REQ-8.2 | TS-04-29 | 2.3 | `TestMockOperatorStartSession` |
| 04-REQ-8.3 | TS-04-30 | 2.3 | `TestMockOperatorStopSession` |
| 04-REQ-8.4 | TS-04-31 | 2.3 | `TestMockOperatorSessionStatus` |
| 04-REQ-8.5 | TS-04-32 | 2.3 | `TestMockOperatorZoneRate` |
| 04-REQ-8.E1 | TS-04-E14 | 2.4 | `TestMockOperatorStopUnknown` |
| 04-REQ-8.E2 | TS-04-E15 | 2.4 | `TestMockOperatorStatusUnknown` |
| 04-REQ-8.E3 | TS-04-E16 | 2.4 | `TestMockOperatorRateUnknown` |
| 04-REQ-9.1 | TS-04-33 | 5.1 | `TestCLIInstall` |
| 04-REQ-9.2 | TS-04-34 | 5.2 | `TestCLIWatch` |
| 04-REQ-9.3 | TS-04-35 | 5.3 | `TestCLIList` |
| 04-REQ-9.4 | TS-04-36 | 5.4 | `TestCLIStartSession` |
| 04-REQ-9.5 | TS-04-37 | 5.5 | `TestCLIStopSession` |
| 04-REQ-9.E1 | TS-04-E17 | 5.6 | `TestCLIUnreachableService` |
| 04-REQ-10.1 | TS-04-38 | 6.2 | `TestIntegration_LockToSession` |
| 04-REQ-10.2 | TS-04-39 | 6.3 | `TestIntegration_CLILifecycle` |
| 04-REQ-10.3 | TS-04-40 | 6.4 | `TestIntegration_AdaptorToOperator` |
| Property 1 | TS-04-P1 | 3.2 | `TestProperty_SessionConsistency` |
| Property 2 | TS-04-P2 | 3.2 | `TestProperty_AutonomousIdempotency` |
| Property 3 | TS-04-P3 | 3.2 | `TestProperty_OverridePrecedence` |
| Property 4 | TS-04-P4 | 4.1 | `TestProperty_StateMachineIntegrity` |
| Property 5 | TS-04-P5 | 4.2 | `TestProperty_ChecksumGate` |
| Property 6 | TS-04-P6 | 4.5 | `TestProperty_OffloadingCorrectness` |
| Property 7 | TS-04-P7 | 2.3 | `TestProperty_FeeAccuracy` |
| Property 8 | TS-04-P8 | 4.1, 4.5, 4.6 | `TestProperty_EventStreamCompleteness` |

## Notes

- **Test implementation language:** Unit tests for Rust services use Rust's
  built-in test framework with `#[tokio::test]`. Unit tests for Go services
  use the standard `testing` package. Integration tests are Go tests in
  `tests/integration/` tagged with `//go:build integration`.
- **Infrastructure dependency:** Integration tests (task group 6) require
  DATA_BROKER running via `make infra-up`. Unit tests must not depend on
  external services.
- **Mock operator as test dependency:** Task group 2 (mock operator) must be
  complete before task group 3 (adaptor) can run integration-level unit tests
  against a real HTTP server.
- **Container runtime for UPDATE_SERVICE:** Full integration testing of
  container lifecycle requires podman. Unit tests should mock the container
  runtime interface.
- **Kuksa proto dependency:** The PARKING_OPERATOR_ADAPTOR's DATA_BROKER client
  uses Kuksa Databroker's proto definitions (kuksa.val.v1). These need to be
  added to the Rust build configuration.
- **Session sizing:** Each task group is scoped for one coding session.
  Task group 3 (adaptor) is the largest and may need to be split across two
  sessions.
