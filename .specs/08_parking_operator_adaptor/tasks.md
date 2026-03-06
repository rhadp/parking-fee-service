# Implementation Plan: PARKING_OPERATOR_ADAPTOR (Spec 08)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the PARKING_OPERATOR_ADAPTOR component that bridges vehicle lock/unlock events (via DATA_BROKER) to a parking operator REST API, manages parking session state, and exposes a gRPC interface for manual session control. Task group 1 writes all failing spec tests. Groups 2-6 implement functionality. Group 7 is the final checkpoint.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Rust workspace skeleton (`rhivos/Cargo.toml`), proto definitions (`proto/parking_adaptor.proto`), and build system (`Makefile`) |
| 02_data_broker | 3 | 1 | Requires running DATA_BROKER (Kuksa Databroker) with VSS signals configured, including `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` and `Vehicle.Parking.SessionActive` |
| 07_update_service | -- | -- | UPDATE_SERVICE manages adaptor container lifecycle (installed/started as container); no build-time dependency |

## Test Commands

- Unit tests: `cd rhivos && cargo test -p parking-operator-adaptor`
- Lint: `cd rhivos && cargo clippy -p parking-operator-adaptor`
- Integration tests: `make infra-up && cd rhivos && cargo test -p parking-operator-adaptor --features integration`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Create Rust crate skeleton for parking-operator-adaptor
    - Add `parking-operator-adaptor` to the Rust workspace in `rhivos/Cargo.toml`
    - Create `rhivos/parking-operator-adaptor/Cargo.toml` with dependencies: `tonic`, `prost`, `tokio`, `reqwest`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`
    - Create `rhivos/parking-operator-adaptor/src/main.rs` with a minimal `#[tokio::main]` entry point that compiles
    - Create `rhivos/parking-operator-adaptor/build.rs` for tonic-build proto compilation of `proto/parking_adaptor.proto`
    - Verify: `cd rhivos && cargo build -p parking-operator-adaptor` succeeds

  - [x] 1.2 Create proto/parking_adaptor.proto
    - Create `proto/parking_adaptor.proto` with the `ParkingAdaptor` service definition (`StartSession`, `StopSession`, `GetStatus`, `GetRate` RPCs)
    - `StopSessionRequest` has no fields (empty message)
    - `GetRateResponse` includes `rate_type`, `rate_amount`, `currency`, and `zone_id`
    - Verify: proto compiles via `cargo build -p parking-operator-adaptor`

  - [x] 1.3 Write failing unit tests for session state machine
    - Create `src/session/mod.rs` and `src/session/state.rs` with minimal `SessionState` enum and `SessionManager` struct (compiles but no logic)
    - Write `#[cfg(test)]` tests: `test_idle_to_starting_on_lock`, `test_starting_to_active_on_operator_ok`, `test_active_to_stopping_on_unlock`, `test_stopping_to_idle_on_operator_ok`, `test_double_lock_ignored` (TS-08-P1), `test_double_unlock_ignored` (TS-08-P2), `test_starting_to_idle_on_operator_error`
    - All tests should fail
    - _Test Spec: TS-08-1, TS-08-2, TS-08-P1, TS-08-P2_

  - [x] 1.4 Write failing unit tests for operator REST client
    - Create `src/operator/mod.rs`, `client.rs`, and `models.rs` with minimal struct stubs
    - Write `#[cfg(test)]` tests: `test_start_session_request_format`, `test_stop_session_request_format`, `test_start_session_response_parse`, `test_stop_session_response_parse`, `test_status_query_response_parse`
    - All tests should fail
    - _Test Spec: TS-08-9, TS-08-10_

  - [x] 1.5 Write failing unit tests for gRPC service
    - Create `src/grpc/mod.rs` and `src/grpc/service.rs` with minimal stubs
    - Write `#[cfg(test)]` tests: `test_get_status_idle` (TS-08-5), `test_get_rate` (TS-08-6), `test_start_session_already_active` (TS-08-E3), `test_stop_session_no_active` (TS-08-E4)
    - All tests should fail
    - _Test Spec: TS-08-5, TS-08-6, TS-08-E3, TS-08-E4_

  - [x] 1.V Verify task group 1
    - [x] `cd rhivos && cargo test -p parking-operator-adaptor` compiles but all tests fail
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p parking-operator-adaptor`

- [x] 2. Implement gRPC server (proto + service stub)
  - [x] 2.1 Implement configuration module
    - Create `src/config.rs` with `Config` struct reading from environment variables with defaults:
    - `PARKING_OPERATOR_URL` (default: `http://localhost:8080`), `DATA_BROKER_ADDR` (default: `http://localhost:55556`), `GRPC_PORT` (default: `50052`), `VEHICLE_ID` (default: `DEMO-VIN-001`), `ZONE_ID` (default: `zone-demo-1`)
    - _Requirements: 08-REQ-8.1_

  - [x] 2.2 Implement session state machine
    - Implement `SessionState` enum with variants: `Idle`, `Starting`, `Active`, `Stopping`
    - Implement `SessionManager` with: `state()`, `try_start()`, `confirm_start(session_id)`, `fail_start()`, `try_stop()`, `confirm_stop()`, `fail_stop()`, `session_id()`
    - Protect state with `tokio::sync::Mutex`
    - _Requirements: 08-REQ-3.1, 08-REQ-4.1_

  - [x] 2.3 Implement ParkingAdaptor gRPC service
    - Implement `src/grpc/service.rs` with struct implementing the `ParkingAdaptor` tonic trait
    - `StartSession`: acquire session lock, call `try_start`, call operator `start_session`, call `confirm_start` / `fail_start`, publish `SessionActive`
    - `StopSession`: acquire session lock, call `try_stop`, call operator `stop_session`, call `confirm_stop` / `fail_stop`, publish `SessionActive`
    - `GetStatus`: read current state and session_id from `SessionManager`
    - `GetRate`: return rate information; return `FAILED_PRECONDITION` if no zone configured
    - Map errors to appropriate gRPC status codes
    - _Requirements: 08-REQ-1.1, 08-REQ-1.2, 08-REQ-1.3, 08-REQ-9.1_

  - [x] 2.4 Wire gRPC server in main.rs
    - Read configuration, create `SessionManager`, start tonic gRPC server on configured port
    - Log startup message
    - _Requirements: 08-REQ-1.1_

  - [x] 2.V Verify task group 2
    - [x] Session state machine tests from task 1.3 pass
    - [x] gRPC tests from task 1.5 pass
    - [x] All existing tests still pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p parking-operator-adaptor`

- [x] 3. Implement DATA_BROKER subscription and state writing
  - [x] 3.1 Implement DATA_BROKER subscriber
    - `BrokerSubscriber::new(addr)` -- connects to Kuksa Databroker via gRPC (network TCP)
    - `subscribe_lock_events() -> impl Stream<Item = bool>` -- subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
    - Handle connection errors by logging and retrying with backoff
    - _Requirements: 08-REQ-2.1_

  - [x] 3.2 Implement DATA_BROKER publisher
    - `BrokerPublisher::new(addr)` -- connects to Kuksa Databroker via gRPC
    - `set_session_active(active: bool)` -- writes `Vehicle.Parking.SessionActive` to DATA_BROKER
    - _Requirements: 08-REQ-6.1, 08-REQ-6.2_

  - [x] 3.3 Write unit tests for broker modules
    - `test_subscriber_connects`, `test_publisher_sets_session_active_true`, `test_publisher_sets_session_active_false`

  - [x] 3.V Verify task group 3
    - [x] Broker module tests pass
    - [x] All existing tests still pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p parking-operator-adaptor`

- [x] 4. Implement autonomous session management (lock/unlock events)
  - [x] 4.1 Implement operator REST client models
    - Implement serde structs: `StartRequest`, `StartResponse`, `StopRequest`, `StopResponse`, `StatusResponse`
    - Add `Serialize`/`Deserialize` derives
    - _Requirements: 08-REQ-7.1, 08-REQ-7.2_

  - [x] 4.2 Implement operator REST client
    - `OperatorClient::new(base_url)` -- creates reqwest client with 5s timeout
    - `start_session(vehicle_id, zone_id)`, `stop_session(session_id)`, `get_status(session_id)`
    - `OperatorError` enum: `Unreachable`, `Timeout`, `HttpError(status, body)`, `ParseError`
    - _Requirements: 08-REQ-7.1, 08-REQ-7.2, 08-REQ-7.3_

  - [x] 4.3 Implement autonomous event loop
    - Spawn a tokio task that subscribes to lock events via `BrokerSubscriber`
    - On `IsLocked = true` (if idle): `try_start` -> operator `start_session` -> `confirm_start` / `fail_start` -> publish `SessionActive`
    - On `IsLocked = false` (if active): `try_stop` -> operator `stop_session` -> `confirm_stop` / `fail_stop` -> publish `SessionActive`
    - Share `SessionManager` with gRPC handlers via `Arc<Mutex<...>>`
    - _Requirements: 08-REQ-3.1, 08-REQ-4.1_

  - [x] 4.4 Integration test: full lock/unlock cycle
    - Test that starts DATA_BROKER, mock operator, and adaptor
    - Write `IsLocked = true`, assert `SessionActive = true` and `GetStatus` returns active
    - Write `IsLocked = false`, assert `SessionActive = false` and `GetStatus` returns idle
    - _Test Spec: TS-08-1, TS-08-2, TS-08-3, TS-08-4_

  - [x] 4.V Verify task group 4
    - [x] Operator client tests from task 1.4 pass
    - [x] Integration tests for lock/unlock cycle pass
    - [x] All existing tests still pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p parking-operator-adaptor`

- [x] 5. Integration tests: REST and error handling
  - [x] 5.1 Integration test: REST start/stop cycle
    - Directly call mock PARKING_OPERATOR REST endpoints to validate contract
    - `POST /parking/start` returns session_id and status
    - `POST /parking/stop` returns session_id, duration, fee, and status
    - `GET /parking/status/{session_id}` returns current session status with rate information
    - _Test Spec: TS-08-9, TS-08-10_

  - [x] 5.2 Integration test: operator unreachable
    - Stop the mock operator, write `IsLocked = true`, verify session remains idle
    - Start a session, stop the mock operator, unlock, verify adaptor transitions to idle
    - _Test Spec: TS-08-E1, TS-08-E2_

  - [x] 5.V Verify task group 5
    - [x] REST integration tests pass
    - [x] Error handling tests pass
    - [x] All existing tests still pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p parking-operator-adaptor`

- [x] 6. Integration tests: manual override and consistency
  - [x] 6.1 Integration test: manual start and stop
    - Call `StartSession` via gRPC, verify session active, verify `SessionActive = true` on DATA_BROKER
    - Call `StopSession` via gRPC, verify session idle, verify `SessionActive = false`
    - _Test Spec: TS-08-7, TS-08-8_

  - [x] 6.2 Integration test: double lock / double unlock
    - Two consecutive `IsLocked = true` events, verify only one operator start call (TS-08-P1)
    - Two consecutive `IsLocked = false` events, verify only one operator stop call (TS-08-P2)
    - _Test Spec: TS-08-P1, TS-08-P2_

  - [x] 6.3 Integration test: manual start then autonomous unlock
    - Call `StartSession` via gRPC
    - Write `IsLocked = true` to DATA_BROKER (should be ignored, session already active -- TS-08-P5)
    - Write `IsLocked = false` to DATA_BROKER (should trigger stop -- TS-08-P3)
    - Verify session is idle
    - _Test Spec: TS-08-P3, TS-08-P5_

  - [x] 6.4 Integration test: state-signal consistency
    - Full cycle of lock/start, verify `SessionActive`, unlock/stop, verify `SessionActive`
    - At every step, `Vehicle.Parking.SessionActive` on DATA_BROKER matches the adaptor's internal state
    - _Test Spec: TS-08-P4_

  - [x] 6.5 Integration test: error gRPC calls
    - `StartSession` when session already active returns `ALREADY_EXISTS` (TS-08-E3)
    - `StopSession` when no session active returns `NOT_FOUND` (TS-08-E4)
    - `GetRate` with no zone returns `FAILED_PRECONDITION` (TS-08-E6)
    - _Test Spec: TS-08-E3, TS-08-E4, TS-08-E6_

  - [x] 6.V Verify task group 6
    - [x] All manual override and consistency tests pass
    - [x] All existing tests still pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p parking-operator-adaptor`

- [ ] 7. Checkpoint
  - [ ] 7.1 Run all unit tests
    - `cd rhivos && cargo test -p parking-operator-adaptor` -- all tests must pass

  - [ ] 7.2 Run lint
    - `cd rhivos && cargo clippy -p parking-operator-adaptor` -- no warnings or errors

  - [ ] 7.3 Run integration tests
    - Start infrastructure: `make infra-up`
    - Run all integration tests, verify all pass
    - Stop infrastructure: `make infra-down`

  - [ ] 7.4 Verify Definition of Done
    - Verify all items from the Definition of Done in `design.md` are satisfied
    - Document any deviations or known limitations

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 08-REQ-1.1 | TS-08-5, TS-08-6 | 2.3, 2.4 | gRPC service tests |
| 08-REQ-1.2 | TS-08-7 | 2.3 | Manual start integration test |
| 08-REQ-1.3 | TS-08-8 | 2.3 | Manual stop integration test |
| 08-REQ-1.E1 | TS-08-E3 | 2.3 | gRPC error test |
| 08-REQ-1.E2 | TS-08-E4 | 2.3 | gRPC error test |
| 08-REQ-2.1 | TS-08-1, TS-08-2 | 3.1 | Broker subscriber tests |
| 08-REQ-2.E1 | TS-08-E5 | 3.1 | Broker error handling test |
| 08-REQ-3.1 | TS-08-1 | 4.3 | Autonomous lock integration test |
| 08-REQ-3.E1 | TS-08-P1 | 4.3 | Double lock test |
| 08-REQ-4.1 | TS-08-2 | 4.3 | Autonomous unlock integration test |
| 08-REQ-4.E1 | TS-08-P2 | 4.3 | Double unlock test |
| 08-REQ-5.1 | TS-08-7, TS-08-8, TS-08-P3 | 2.3, 4.3 | Manual override integration tests |
| 08-REQ-5.2 | TS-08-P5 | 4.3 | Manual start + lock ignored test |
| 08-REQ-6.1 | TS-08-3 | 3.2, 4.3 | SessionActive integration test |
| 08-REQ-6.2 | TS-08-4 | 3.2, 4.3 | SessionActive integration test |
| 08-REQ-6.E1 | TS-08-E5 | 3.2 | Broker write failure test |
| 08-REQ-7.1 | TS-08-9 | 4.1, 4.2 | REST start integration test |
| 08-REQ-7.2 | TS-08-9 | 4.1, 4.2 | REST stop integration test |
| 08-REQ-7.3 | TS-08-10 | 4.2 | REST status integration test |
| 08-REQ-7.E1 | TS-08-E1, TS-08-E2 | 4.2 | Operator unreachable test |
| 08-REQ-7.E2 | TS-08-E1 | 4.2 | Operator non-200 test |
| 08-REQ-8.1 | -- | 2.1 | Config loading verified at startup |
| 08-REQ-9.1 | TS-08-6 | 2.3 | GetRate gRPC test |
| 08-REQ-9.E1 | TS-08-E6 | 2.3 | GetRate no zone test |
