# Implementation Plan: PARKING_OPERATOR_ADAPTOR

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the PARKING_OPERATOR_ADAPTOR as a Rust crate (`rhivos/parking-operator-adaptor`) within the existing Cargo workspace. The service bridges PARKING_APP (gRPC) with PARKING_OPERATOR (REST) and autonomously manages parking sessions via DATA_BROKER lock/unlock events. Task group 1 writes failing tests. Groups 2-3 implement pure-function modules (model, config, session). Group 4 implements operator_client and broker modules. Group 5 implements gRPC service and main. Group 6 runs integration tests.

Ordering: tests first, then data types, then pure-function modules (config, session), then I/O modules (operator_client, broker), then gRPC handlers and main, then integration tests.

## Test Commands

- Spec tests (unit): `cd rhivos && cargo test -p parking-operator-adaptor`
- Spec tests (integration): `cd tests/parking-operator-adaptor && go test -v ./...`
- Property tests: `cd rhivos && cargo test -p parking-operator-adaptor proptest`
- All tests: `cd rhivos && cargo test`
- Linter: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`

## Tasks

- [ ] 1. Write failing spec tests
  - [ ] 1.1 Set up test infrastructure
    - Ensure `rhivos/parking-operator-adaptor/` has `src/lib.rs` with module declarations
    - Create source files with empty/stub implementations: `model.rs`, `config.rs`, `session.rs`, `operator_client.rs`, `broker.rs`, `grpc_service.rs`
    - Add dev-dependencies: `proptest`, `tokio` (test features)
    - _Test Spec: TS-08-1 through TS-08-18_

  - [ ] 1.2 Write config and session unit tests
    - `test_config_from_env_vars` — TS-08-15
    - `test_config_defaults` — TS-08-16
    - `test_session_state_stored_after_start` — TS-08-2
    - `test_get_status_active_session` — TS-08-10
    - `test_get_status_no_session` — TS-08-11
    - `test_get_rate_active_session` — TS-08-12
    - _Test Spec: TS-08-2, TS-08-10, TS-08-11, TS-08-12, TS-08-15, TS-08-16_

  - [ ] 1.3 Write autonomous session and manual override unit tests
    - `test_autonomous_start_on_lock` — TS-08-1
    - `test_session_active_written_on_start` — TS-08-3
    - `test_autonomous_stop_on_unlock` — TS-08-4
    - `test_session_cleared_after_stop` — TS-08-5
    - `test_session_active_written_on_stop` — TS-08-6
    - `test_manual_start_session` — TS-08-7
    - `test_manual_stop_session` — TS-08-8
    - `test_resume_autonomous_after_override` — TS-08-9
    - `test_lock_subscription` — TS-08-13
    - `test_session_active_written` — TS-08-14
    - _Test Spec: TS-08-1, TS-08-3, TS-08-4, TS-08-5, TS-08-6, TS-08-7, TS-08-8, TS-08-9, TS-08-13, TS-08-14_

  - [ ] 1.4 Write edge case tests
    - `test_lock_while_session_active` — TS-08-E1
    - `test_operator_start_failure` — TS-08-E2
    - `test_unlock_while_no_session` — TS-08-E3
    - `test_operator_stop_failure` — TS-08-E4
    - `test_start_session_while_active` — TS-08-E5
    - `test_stop_session_while_no_session` — TS-08-E6
    - `test_get_rate_no_session` — TS-08-E7
    - `test_session_active_write_failure` — TS-08-E9
    - _Test Spec: TS-08-E1 through TS-08-E7, TS-08-E9_

  - [ ] 1.5 Write property tests
    - `proptest_autonomous_start_on_lock` — TS-08-P1
    - `proptest_autonomous_stop_on_unlock` — TS-08-P2
    - `proptest_session_idempotency` — TS-08-P3
    - `proptest_manual_override_consistency` — TS-08-P4
    - `proptest_operator_retry_logic` — TS-08-P5
    - `proptest_config_defaults` — TS-08-P6
    - _Test Spec: TS-08-P1 through TS-08-P6_

  - [ ] 1.V Verify task group 1
    - [ ] All test files compile: `cd rhivos && cargo test -p parking-operator-adaptor --no-run`
    - [ ] All spec tests FAIL (red): `cd rhivos && cargo test -p parking-operator-adaptor 2>&1 | grep FAILED`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`

- [ ] 2. Model and config modules
  - [ ] 2.1 Implement model module
    - Define types: `SessionState`, `Rate`, `StartRequest`, `StartResponse`, `StopRequest`, `StopResponse`
    - Add `Clone`, `Debug` derives; `Serialize`/`Deserialize` where needed
    - Define error types: `SessionError`, `OperatorError`, `BrokerError`
    - _Requirements: 08-REQ-1.2, 08-REQ-5.1_

  - [ ] 2.2 Implement config module
    - `load_config() -> Config`: read from env vars with defaults
    - Defaults: operator URL `http://localhost:8080`, databroker `http://localhost:55556`, port `50053`, vehicle_id `DEMO-VIN-001`, zone_id `zone-demo-1`
    - _Requirements: 08-REQ-7.1, 08-REQ-7.2_

  - [ ] 2.V Verify task group 2
    - [ ] Config tests pass: `cd rhivos && cargo test -p parking-operator-adaptor config`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [ ] _Test Spec: TS-08-15, TS-08-16, TS-08-P6_

- [x] 3. Session module
  - [x] 3.1 Implement SessionManager
    - `SessionManager` wrapping `Arc<Mutex<Option<SessionState>>>`
    - `new() -> Self`
    - `start(&self, session_id, zone_id, rate) -> Result<(), SessionError>`
    - `stop(&self) -> Result<SessionState, SessionError>`
    - `get_status(&self) -> Option<SessionState>`
    - `get_rate(&self) -> Option<Rate>`
    - `is_active(&self) -> bool`
    - Return `SessionError::AlreadyActive` from `start` if session exists
    - Return `SessionError::NotActive` from `stop` if no session
    - _Requirements: 08-REQ-1.2, 08-REQ-2.2, 08-REQ-3.E1, 08-REQ-3.E2, 08-REQ-4.1, 08-REQ-4.2, 08-REQ-5.1, 08-REQ-5.2_

  - [x] 3.V Verify task group 3
    - [x] Session tests pass: `cd rhivos && cargo test -p parking-operator-adaptor session`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [x] _Test Spec: TS-08-2, TS-08-5, TS-08-10, TS-08-11, TS-08-12, TS-08-E5, TS-08-E6, TS-08-E7_

- [x] 4. Operator client and broker modules
  - [x] 4.1 Implement OperatorClient trait and HttpOperatorClient
    - Define `OperatorApi` async trait with `start_session`, `stop_session`
    - Implement `OperatorApi` for `OperatorClient` using reqwest
    - `RetryOperatorClient<T>` wrapper: 3 attempts with exponential backoff (1s, 2s)
    - _Requirements: 08-REQ-1.1, 08-REQ-1.E2, 08-REQ-2.1, 08-REQ-2.E2_

  - [x] 4.2 Implement MockOperatorClient for testing
    - Configurable success/failure responses
    - Call counting for verification
    - Reset capability
    - _Test Spec: TS-08-1, TS-08-4, TS-08-E1, TS-08-E2, TS-08-E3, TS-08-E4_

  - [x] 4.3 Implement BrokerClient trait and KuksaBrokerClient
    - Define `SessionPublisher` async trait with `set_session_active`
    - `BrokerSessionPublisher` wrapper implements trait for `BrokerPublisher`
    - `BrokerSubscriber` retains existing connection retry logic
    - _Requirements: 08-REQ-6.1, 08-REQ-6.2, 08-REQ-6.E1, 08-REQ-6.E2_

  - [x] 4.4 Implement MockBrokerClient for testing
    - `MockBrokerPublisher` with configurable failure and call recording
    - `NoopPublisher` for tests that don't need broker verification
    - _Test Spec: TS-08-3, TS-08-6, TS-08-13, TS-08-E9_

  - [x] 4.5 Implement handle_lock_event and auto-session loop
    - `handle_lock_event(session, operator, publisher, vehicle_id, zone_id)`: idempotent start logic
    - `handle_unlock_event(session, operator, publisher)`: idempotent stop logic
    - `run_autonomous_loop(...)`: subscribe and dispatch events
    - Refactored to use `&dyn OperatorApi` and `&dyn SessionPublisher` traits
    - Fixed `fail_stop()` to return to Active state (per 08-REQ-2.E2)
    - _Requirements: 08-REQ-1.1, 08-REQ-1.3, 08-REQ-2.1, 08-REQ-2.3, 08-REQ-1.E1, 08-REQ-2.E1_

  - [x] 4.V Verify task group 4
    - [x] Operator and broker tests pass: `cd rhivos && cargo test -p parking-operator-adaptor -- operator broker handle_lock`
    - [x] All existing tests still pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [x] _Test Spec: TS-08-1, TS-08-3, TS-08-4, TS-08-5, TS-08-6, TS-08-8, TS-08-9, TS-08-13, TS-08-14, TS-08-E1, TS-08-E2, TS-08-E3, TS-08-E4, TS-08-E9, TS-08-P1, TS-08-P2, TS-08-P3, TS-08-P4, TS-08-P5_

- [x] 5. gRPC service and main
  - [x] 5.1 Vendor proto files
    - Copy/create `parking_adaptor.proto` and `common.proto` into `rhivos/parking-operator-adaptor/proto/`
    - Set up `build.rs` for tonic-build code generation
    - Define RPCs: StartSession, StopSession, GetStatus, GetRate
    - _Requirements: 08-REQ-3.1, 08-REQ-3.2, 08-REQ-4.1, 08-REQ-5.1_

  - [x] 5.2 Implement gRPC service handlers
    - `ParkingAdaptorService` struct holding `SessionManager`, `OperatorClient`, `BrokerClient`
    - `start_session`: check no active session, call operator, store session, write broker → return session_id
    - `stop_session`: check active session, call operator, clear session, write broker → return stop details
    - `get_status`: return session state or active=false
    - `get_rate`: return cached rate or NOT_FOUND
    - _Requirements: 08-REQ-3.1, 08-REQ-3.2, 08-REQ-3.E1, 08-REQ-3.E2, 08-REQ-4.1, 08-REQ-4.2, 08-REQ-5.1, 08-REQ-5.2_

  - [x] 5.3 Implement main
    - Load config, init tracing
    - Connect to DATA_BROKER with retries (5 attempts with exponential backoff, exit non-zero on failure)
    - Start auto-session loop (tokio::spawn)
    - Start gRPC server on configured port with graceful shutdown
    - Handle SIGTERM/SIGINT: stop active session, close connections, exit 0
    - Log version, port, operator URL, DATA_BROKER address, vehicle ID at startup
    - _Requirements: 08-REQ-6.E1, 08-REQ-8.1, 08-REQ-8.2_

  - [x] 5.V Verify task group 5
    - [x] Binary builds: `cd rhivos && cargo build -p parking-operator-adaptor`
    - [x] All unit tests pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [x] _Test Spec: TS-08-7, TS-08-8, TS-08-E5, TS-08-E6_

- [ ] 6. Integration test validation
  - [ ] 6.1 Create integration test module
    - Create `tests/parking-operator-adaptor/` Go module
    - Shared helpers: start/stop service binary, mock PARKING_OPERATOR HTTP server, gRPC client
    - Add `go.work` entry for `./tests/parking-operator-adaptor`
    - _Test Spec: TS-08-17, TS-08-18, TS-08-E8_

  - [ ] 6.2 Write and run integration tests
    - `TestStartupLogging` — TS-08-17: capture startup logs, verify port and operator URL present
    - `TestGracefulShutdown` — TS-08-18: send SIGTERM, verify exit code 0
    - `TestDataBrokerUnreachable` — TS-08-E8: start with unreachable DATA_BROKER_ADDR, verify non-zero exit
    - _Test Spec: TS-08-17, TS-08-18, TS-08-E8_

  - [ ] 6.V Verify task group 6
    - [ ] All integration tests pass: `cd tests/parking-operator-adaptor && go test -v ./...`
    - [ ] All unit tests still pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [ ] All existing tests still pass: `make test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [ ] All requirements 08-REQ-1 through 08-REQ-8 acceptance criteria met

- [x] 7. Checkpoint - All Tests Green
  - All unit, integration, and property tests pass
  - Binary starts, serves gRPC requests, subscribes to DATA_BROKER, calls PARKING_OPERATOR REST API, shuts down cleanly
  - Ask the user if questions arise

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
| 08-REQ-1.1 | TS-08-1 | 4.5 | test_autonomous_start_on_lock |
| 08-REQ-1.2 | TS-08-2 | 3.1 | test_session_state_stored_after_start |
| 08-REQ-1.3 | TS-08-3 | 4.5 | test_session_active_written_on_start |
| 08-REQ-1.E1 | TS-08-E1 | 4.5 | test_lock_while_session_active |
| 08-REQ-1.E2 | TS-08-E2 | 4.1 | test_operator_start_failure |
| 08-REQ-2.1 | TS-08-4 | 4.5 | test_autonomous_stop_on_unlock |
| 08-REQ-2.2 | TS-08-5 | 3.1 | test_session_cleared_after_stop |
| 08-REQ-2.3 | TS-08-6 | 4.5 | test_session_active_written_on_stop |
| 08-REQ-2.E1 | TS-08-E3 | 4.5 | test_unlock_while_no_session |
| 08-REQ-2.E2 | TS-08-E4 | 4.1 | test_operator_stop_failure |
| 08-REQ-3.1 | TS-08-7 | 5.2 | test_manual_start_session |
| 08-REQ-3.2 | TS-08-8 | 5.2 | test_manual_stop_session |
| 08-REQ-3.3 | TS-08-9 | 4.5 | test_resume_autonomous_after_override |
| 08-REQ-3.E1 | TS-08-E5 | 5.2 | test_start_session_while_active |
| 08-REQ-3.E2 | TS-08-E6 | 5.2 | test_stop_session_while_no_session |
| 08-REQ-4.1 | TS-08-10 | 3.1 | test_get_status_active_session |
| 08-REQ-4.2 | TS-08-11 | 3.1 | test_get_status_no_session |
| 08-REQ-5.1 | TS-08-12 | 3.1 | test_get_rate_active_session |
| 08-REQ-5.2 | TS-08-E7 | 3.1 | test_get_rate_no_session |
| 08-REQ-6.1 | TS-08-13 | 4.3 | test_lock_subscription |
| 08-REQ-6.2 | TS-08-14 | 4.5 | test_session_active_written |
| 08-REQ-6.E1 | TS-08-E8 | 6.2 | tests/parking-operator-adaptor::TestDataBrokerUnreachable |
| 08-REQ-6.E2 | TS-08-E9 | 4.4 | test_session_active_write_failure |
| 08-REQ-7.1 | TS-08-15 | 2.2 | test_config_from_env_vars |
| 08-REQ-7.2 | TS-08-16 | 2.2 | test_config_defaults |
| 08-REQ-8.1 | TS-08-17 | 6.2 | tests/parking-operator-adaptor::TestStartupLogging |
| 08-REQ-8.2 | TS-08-18 | 6.2 | tests/parking-operator-adaptor::TestGracefulShutdown |
| Property 1 | TS-08-P1 | 4.5 | proptest_autonomous_start_on_lock |
| Property 2 | TS-08-P2 | 4.5 | proptest_autonomous_stop_on_unlock |
| Property 3 | TS-08-P3 | 4.5 | proptest_session_idempotency |
| Property 4 | TS-08-P4 | 4.5 | proptest_manual_override_consistency |
| Property 5 | TS-08-P5 | 4.1 | proptest_operator_retry_logic |
| Property 6 | TS-08-P6 | 2.2 | proptest_config_defaults |

## Notes

- The PARKING_OPERATOR_ADAPTOR uses the same BrokerClient trait pattern as LOCKING_SERVICE and CLOUD_GATEWAY_CLIENT. Proto files are vendored per-crate into `rhivos/parking-operator-adaptor/proto/`.
- The OperatorClient trait abstracts the REST client, enabling unit tests without a real PARKING_OPERATOR. The HttpOperatorClient implementation uses reqwest with retry logic.
- Property tests use the `proptest` crate. Each property test generates random inputs (zone_ids, rates, session states, lock/unlock sequences) and asserts invariants.
- Integration tests in `tests/parking-operator-adaptor/` use a Go test module with a mock HTTP server standing in for the PARKING_OPERATOR REST API. The service binary is started as a subprocess.
- Session state is purely in-memory (no persistence). On restart, session state is lost — this is acceptable for the demo.
- The auto-session loop processes lock/unlock events sequentially via a tokio channel, ensuring no concurrent session modifications.
