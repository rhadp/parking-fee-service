# Implementation Plan: PARKING_OPERATOR_ADAPTOR

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the PARKING_OPERATOR_ADAPTOR as a Rust binary in `rhivos/parking-operator-adaptor/`. The service exposes a gRPC interface for manual session management, subscribes to DATA_BROKER lock/unlock events for autonomous session management, and communicates with the PARKING_OPERATOR via REST. Task group 1 writes failing tests. Groups 2-3 implement core logic (config, session state, operator REST client). Group 4 implements the DATA_BROKER client, gRPC server, and event loop. Group 5 runs integration tests. Group 6 is wiring verification.

Ordering: tests first (TDD), then pure-function modules (no external dependencies), then the operator REST client, then the broker client + gRPC server + event loop, then integration validation, then wiring verification.

## Test Commands

- Spec tests (unit): `cd rhivos && cargo test -p parking-operator-adaptor`
- Spec tests (integration): `cd tests/parking-operator-adaptor && go test -v ./...`
- Property tests: `cd rhivos && cargo test -p parking-operator-adaptor -- --include-ignored proptest`
- All Rust tests: `cd rhivos && cargo test`
- Linter: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`

## Tasks

- [ ] 1. Write failing spec tests
  - [ ] 1.1 Add dependencies to parking-operator-adaptor Cargo.toml
    - Add: serde, serde_json, tokio, tonic, prost, tracing, tracing-subscriber, reqwest, proptest (dev), wiremock (dev)
    - Vendor kuksa.val.v1 proto definitions into `rhivos/parking-operator-adaptor/proto/`
    - Vendor parking_adaptor.proto (from spec 01 group 6) into `rhivos/parking-operator-adaptor/proto/`
    - Add tonic-build to build.rs for proto code generation
    - _Test Spec: TS-08-1 through TS-08-22_

  - [ ] 1.2 Write config and session state unit tests
    - Create `rhivos/parking-operator-adaptor/src/config.rs` with test module
    - `test_config_defaults` — TS-08-18
    - `test_config_custom_values` — TS-08-19
    - `test_config_invalid_grpc_port` — TS-08-E10
    - Create `rhivos/parking-operator-adaptor/src/session.rs` with test module
    - `test_session_start_stop_fields` — TS-08-22
    - `test_get_status_active` — TS-08-4
    - `test_get_status_inactive` — TS-08-5
    - `test_get_rate_active` — TS-08-6
    - `test_get_rate_inactive` — TS-08-7
    - _Test Spec: TS-08-1, TS-08-4, TS-08-5, TS-08-6, TS-08-7, TS-08-18, TS-08-19, TS-08-22, TS-08-E10_

  - [ ] 1.3 Write operator REST client unit tests
    - Create `rhivos/parking-operator-adaptor/src/operator.rs` with test module
    - `test_start_session_request` — TS-08-8
    - `test_stop_session_request` — TS-08-9
    - `test_start_response_parsing` — TS-08-10
    - `test_retry_on_failure` — TS-08-E3
    - `test_retry_exhausted` — TS-08-E4
    - `test_retry_on_non_200` — TS-08-E5
    - _Test Spec: TS-08-8, TS-08-9, TS-08-10, TS-08-E3, TS-08-E4, TS-08-E5_

  - [ ] 1.4 Write event processing and gRPC handler tests
    - Create `rhivos/parking-operator-adaptor/src/event_loop.rs` with test module
    - `test_lock_event_starts_session` — TS-08-11
    - `test_unlock_event_stops_session` — TS-08-12
    - `test_session_active_set_true` — TS-08-13
    - `test_session_active_set_false` — TS-08-14
    - `test_manual_start_override` — TS-08-16
    - `test_manual_stop_override` — TS-08-17
    - _Test Spec: TS-08-2, TS-08-3, TS-08-11, TS-08-12, TS-08-13, TS-08-14, TS-08-16, TS-08-17_

  - [ ] 1.5 Write edge case and override tests
    - `test_start_session_already_active` — TS-08-E1
    - `test_stop_session_no_active` — TS-08-E2
    - `test_lock_event_noop_when_active` — TS-08-E6
    - `test_unlock_event_noop_when_inactive` — TS-08-E7
    - `test_session_active_publish_failure` — TS-08-E9
    - `test_override_resumes_autonomous` — TS-08-E11
    - _Test Spec: TS-08-E1, TS-08-E2, TS-08-E6, TS-08-E7, TS-08-E9, TS-08-E11_

  - [ ] 1.6 Write property tests
    - `proptest_session_state_consistency` — TS-08-P1
    - `proptest_idempotent_lock_events` — TS-08-P2
    - `proptest_override_non_persistence` — TS-08-P3
    - `proptest_retry_exhaustion_safety` — TS-08-P4
    - `proptest_session_active_consistency` — TS-08-P5
    - `proptest_sequential_event_processing` — TS-08-P6
    - _Test Spec: TS-08-P1 through TS-08-P6_

  - [ ] 1.V Verify task group 1
    - [ ] All test files compile: `cd rhivos && cargo test -p parking-operator-adaptor --no-run`
    - [ ] All unit tests FAIL (red): `cd rhivos && cargo test -p parking-operator-adaptor 2>&1 | grep FAILED`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`

- [ ] 2. Config and session state modules
  - [ ] 2.1 Implement config module
    - Read all 5 env vars with defaults: PARKING_OPERATOR_URL, DATA_BROKER_ADDR, GRPC_PORT, VEHICLE_ID, ZONE_ID
    - Validate GRPC_PORT is a valid u16
    - Return Config struct or error
    - _Requirements: 08-REQ-7.1, 08-REQ-7.2, 08-REQ-7.3, 08-REQ-7.4, 08-REQ-7.5, 08-REQ-7.E1_

  - [ ] 2.2 Implement session module
    - Define `SessionState`, `Rate`, `Session` structs
    - Implement `Session::new()`, `start()`, `stop()`, `is_active()`, `status()`, `rate()`
    - start() populates all fields, stop() clears them
    - _Requirements: 08-REQ-6.1, 08-REQ-6.2, 08-REQ-6.3_

  - [ ] 2.V Verify task group 2
    - [ ] Config and session tests pass: `cd rhivos && cargo test -p parking-operator-adaptor -- config session`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [ ] _Test Spec: TS-08-1, TS-08-4, TS-08-5, TS-08-6, TS-08-7, TS-08-18, TS-08-19, TS-08-22, TS-08-E10_

- [ ] 3. Operator REST client
  - [ ] 3.1 Implement operator module
    - Define `OperatorClient` with reqwest::Client and base_url
    - Implement `start_session(vehicle_id, zone_id)`: POST /parking/start with JSON body
    - Implement `stop_session(session_id)`: POST /parking/stop with JSON body
    - Parse responses into StartResponse / StopResponse structs
    - _Requirements: 08-REQ-2.1, 08-REQ-2.2, 08-REQ-2.3, 08-REQ-2.4_

  - [ ] 3.2 Implement retry logic
    - Wrap REST calls with retry: max 3 retries, exponential backoff 1s, 2s, 4s
    - Retry on connection error, timeout, or non-200 status
    - Return OperatorError after all retries exhausted
    - _Requirements: 08-REQ-2.E1, 08-REQ-2.E2_

  - [ ] 3.V Verify task group 3
    - [ ] Operator tests pass: `cd rhivos && cargo test -p parking-operator-adaptor -- operator`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [ ] _Test Spec: TS-08-8, TS-08-9, TS-08-10, TS-08-E3, TS-08-E4, TS-08-E5_

- [ ] 4. DATA_BROKER client, gRPC server, and event loop
  - [ ] 4.1 Implement broker module
    - Implement BrokerClient with tonic-generated kuksa.val.v1 client
    - `connect(addr)`: establish gRPC channel with retry (1s, 2s, 4s, up to 5 attempts)
    - `subscribe_bool(signal)`: create kuksa Subscribe stream for IsLocked
    - `set_bool(signal, value)`: kuksa Set for Vehicle.Parking.SessionActive
    - _Requirements: 08-REQ-3.1, 08-REQ-3.2, 08-REQ-3.E3_

  - [ ] 4.2 Implement gRPC server
    - Implement ParkingAdaptorService with tonic from parking_adaptor.proto
    - StartSession: validate no active session, delegate to operator, update session, publish signal
    - StopSession: validate active session, delegate to operator, clear session, publish signal
    - GetStatus: read session state, return response
    - GetRate: read session rate, return response
    - _Requirements: 08-REQ-1.1, 08-REQ-1.2, 08-REQ-1.3, 08-REQ-1.4, 08-REQ-1.5, 08-REQ-1.E1, 08-REQ-1.E2_

  - [ ] 4.3 Implement event loop
    - Create SessionEvent enum for serialized processing
    - Use tokio::mpsc channel to receive events from both DATA_BROKER subscription and gRPC handlers
    - Process events sequentially: lock→start, unlock→stop, manual start/stop, queries
    - Handle idempotent cases (lock when active, unlock when inactive)
    - _Requirements: 08-REQ-3.3, 08-REQ-3.4, 08-REQ-3.E1, 08-REQ-3.E2, 08-REQ-5.1, 08-REQ-5.2, 08-REQ-5.3, 08-REQ-9.1, 08-REQ-9.2_

  - [ ] 4.4 Implement main entry point
    - Parse config, connect to DATA_BROKER with retry
    - Publish initial SessionActive=false
    - Subscribe to IsLocked signal
    - Start gRPC server
    - Start event loop
    - Handle SIGTERM/SIGINT via tokio signal handler
    - Log startup info and ready message
    - _Requirements: 08-REQ-4.3, 08-REQ-8.1, 08-REQ-8.2, 08-REQ-8.3, 08-REQ-8.E1_

  - [ ] 4.V Verify task group 4
    - [ ] Binary compiles: `cd rhivos && cargo build -p parking-operator-adaptor`
    - [ ] All unit tests pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [ ] Property tests pass: `cd rhivos && cargo test -p parking-operator-adaptor -- --include-ignored proptest`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`

- [ ] 5. Integration test validation
  - [ ] 5.1 Create integration test module
    - Create `tests/parking-operator-adaptor/` Go module (or add to existing test structure)
    - Shared helpers: start/stop databroker, start/stop mock operator HTTP server, start/stop adaptor, gRPC client helpers
    - Add `go.work` entry for `./tests/parking-operator-adaptor`
    - _Test Spec: TS-08-15, TS-08-20, TS-08-21, TS-08-E8, TS-08-E12_

  - [ ] 5.2 Write and run integration tests
    - `TestInitialSessionActive` — TS-08-15: verify SessionActive=false on startup
    - `TestStartupLogging` — TS-08-20: verify log output
    - `TestGracefulShutdown` — TS-08-21: verify clean exit on SIGTERM
    - `TestDatabrokerUnreachable` — TS-08-E8: verify retry behavior
    - `TestSessionLostOnRestart` — TS-08-E12: verify session state lost on restart
    - _Test Spec: TS-08-15, TS-08-20, TS-08-21, TS-08-E8, TS-08-E12_

  - [ ] 5.3 Write and run smoke tests
    - `TestLockStartUnlockStopFlow` — TS-08-SMOKE-1: end-to-end autonomous flow
    - `TestManualOverrideFlow` — TS-08-SMOKE-2: manual gRPC start/stop
    - `TestOverrideThenAutonomousResume` — TS-08-SMOKE-3: manual override then autonomous resume
    - _Test Spec: TS-08-SMOKE-1, TS-08-SMOKE-2, TS-08-SMOKE-3_

  - [ ] 5.V Verify task group 5
    - [ ] All integration tests pass: `cd tests/parking-operator-adaptor && go test -v ./...`
    - [ ] All unit tests still pass: `cd rhivos && cargo test -p parking-operator-adaptor`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [ ] All requirements 08-REQ-1 through 08-REQ-9 acceptance criteria met

- [ ] 6. Wiring verification
  - [ ] 6.1 Verify all requirements covered
    - Cross-check every 08-REQ-* requirement against test results
    - Confirm all edge case tests pass
    - Confirm all property tests pass
    - _Requirements: all 08-REQ-*_

  - [ ] 6.2 Verify integration with dependencies
    - Confirm DATA_BROKER subscription works with spec 02 (Kuksa Databroker)
    - Confirm gRPC proto compatibility with spec 01 group 6 (parking_adaptor.proto)
    - Confirm Cargo workspace membership from spec 01 group 3
    - _Dependencies: 01_project_setup groups 3, 6; 02_data_broker group 2_

  - [ ] 6.3 Verify operational readiness
    - Confirm startup logging includes all config values
    - Confirm graceful shutdown works
    - Confirm container builds successfully
    - _Requirements: 08-REQ-8.1, 08-REQ-8.2, 08-REQ-8.3_

  - [ ] 6.V Verify task group 6
    - [ ] Full test suite passes: `cd rhivos && cargo test -p parking-operator-adaptor && cd tests/parking-operator-adaptor && go test -v ./...`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p parking-operator-adaptor -- -D warnings`
    - [ ] All smoke tests pass
    - [ ] Traceability table complete — all requirements mapped to passing tests

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
| 08-REQ-1.1 | TS-08-1 | 4.2 | parking-operator-adaptor::config::test_config_defaults |
| 08-REQ-1.2 | TS-08-2 | 4.2 | parking-operator-adaptor::grpc_server::test_start_session_rpc |
| 08-REQ-1.3 | TS-08-3 | 4.2 | parking-operator-adaptor::grpc_server::test_stop_session_rpc |
| 08-REQ-1.4 | TS-08-4, TS-08-5 | 4.2 | parking-operator-adaptor::session::test_get_status_active |
| 08-REQ-1.5 | TS-08-6, TS-08-7 | 4.2 | parking-operator-adaptor::session::test_get_rate_active |
| 08-REQ-1.E1 | TS-08-E1 | 4.2 | parking-operator-adaptor::grpc_server::test_start_session_already_active |
| 08-REQ-1.E2 | TS-08-E2 | 4.2 | parking-operator-adaptor::grpc_server::test_stop_session_no_active |
| 08-REQ-2.1 | TS-08-8 | 3.1 | parking-operator-adaptor::operator::test_start_session_request |
| 08-REQ-2.2 | TS-08-9 | 3.1 | parking-operator-adaptor::operator::test_stop_session_request |
| 08-REQ-2.3 | TS-08-10 | 3.1 | parking-operator-adaptor::operator::test_start_response_parsing |
| 08-REQ-2.4 | TS-08-9 | 3.1 | parking-operator-adaptor::operator::test_stop_session_request |
| 08-REQ-2.E1 | TS-08-E3, TS-08-E4 | 3.2 | parking-operator-adaptor::operator::test_retry_on_failure |
| 08-REQ-2.E2 | TS-08-E5 | 3.2 | parking-operator-adaptor::operator::test_retry_on_non_200 |
| 08-REQ-3.1 | TS-08-11 | 4.1, 4.4 | parking-operator-adaptor::event_loop::test_lock_event_starts_session |
| 08-REQ-3.2 | TS-08-11 | 4.1 | parking-operator-adaptor::event_loop::test_lock_event_starts_session |
| 08-REQ-3.3 | TS-08-11 | 4.3 | parking-operator-adaptor::event_loop::test_lock_event_starts_session |
| 08-REQ-3.4 | TS-08-12 | 4.3 | parking-operator-adaptor::event_loop::test_unlock_event_stops_session |
| 08-REQ-3.E1 | TS-08-E6 | 4.3 | parking-operator-adaptor::event_loop::test_lock_event_noop_when_active |
| 08-REQ-3.E2 | TS-08-E7 | 4.3 | parking-operator-adaptor::event_loop::test_unlock_event_noop_when_inactive |
| 08-REQ-3.E3 | TS-08-E8 | 4.1 | tests/parking-operator-adaptor::TestDatabrokerUnreachable |
| 08-REQ-4.1 | TS-08-13 | 4.3 | parking-operator-adaptor::event_loop::test_session_active_set_true |
| 08-REQ-4.2 | TS-08-14 | 4.3 | parking-operator-adaptor::event_loop::test_session_active_set_false |
| 08-REQ-4.3 | TS-08-15 | 4.4 | tests/parking-operator-adaptor::TestInitialSessionActive |
| 08-REQ-4.E1 | TS-08-E9 | 4.3 | parking-operator-adaptor::event_loop::test_session_active_publish_failure |
| 08-REQ-5.1 | TS-08-16 | 4.2, 4.3 | parking-operator-adaptor::event_loop::test_manual_start_override |
| 08-REQ-5.2 | TS-08-17 | 4.2, 4.3 | parking-operator-adaptor::event_loop::test_manual_stop_override |
| 08-REQ-5.3 | TS-08-E11 | 4.3 | parking-operator-adaptor::event_loop::test_override_resumes_autonomous |
| 08-REQ-5.E1 | TS-08-E11 | 4.3 | parking-operator-adaptor::event_loop::test_override_resumes_autonomous |
| 08-REQ-6.1 | TS-08-22 | 2.2 | parking-operator-adaptor::session::test_session_start_stop_fields |
| 08-REQ-6.2 | TS-08-22 | 2.2 | parking-operator-adaptor::session::test_session_start_stop_fields |
| 08-REQ-6.3 | TS-08-22 | 2.2 | parking-operator-adaptor::session::test_session_start_stop_fields |
| 08-REQ-6.E1 | TS-08-E12 | 4.4 | tests/parking-operator-adaptor::TestSessionLostOnRestart |
| 08-REQ-7.1 | TS-08-18, TS-08-19 | 2.1 | parking-operator-adaptor::config::test_config_defaults |
| 08-REQ-7.2 | TS-08-18, TS-08-19 | 2.1 | parking-operator-adaptor::config::test_config_defaults |
| 08-REQ-7.3 | TS-08-18, TS-08-19 | 2.1 | parking-operator-adaptor::config::test_config_defaults |
| 08-REQ-7.4 | TS-08-18, TS-08-19 | 2.1 | parking-operator-adaptor::config::test_config_defaults |
| 08-REQ-7.5 | TS-08-18, TS-08-19 | 2.1 | parking-operator-adaptor::config::test_config_defaults |
| 08-REQ-7.E1 | TS-08-E10 | 2.1 | parking-operator-adaptor::config::test_config_invalid_grpc_port |
| 08-REQ-8.1 | TS-08-20 | 4.4 | tests/parking-operator-adaptor::TestStartupLogging |
| 08-REQ-8.2 | TS-08-20 | 4.4 | tests/parking-operator-adaptor::TestStartupLogging |
| 08-REQ-8.3 | TS-08-21 | 4.4 | tests/parking-operator-adaptor::TestGracefulShutdown |
| 08-REQ-8.E1 | TS-08-21 | 4.4 | tests/parking-operator-adaptor::TestGracefulShutdown |
| 08-REQ-9.1 | TS-08-P6 | 4.3 | parking-operator-adaptor::proptest_sequential_event_processing |
| 08-REQ-9.2 | TS-08-P6 | 4.3 | parking-operator-adaptor::proptest_sequential_event_processing |
| 08-REQ-9.E1 | TS-08-P6 | 4.3 | parking-operator-adaptor::proptest_sequential_event_processing |
| Property 1 | TS-08-P1 | 2.2 | parking-operator-adaptor::proptest_session_state_consistency |
| Property 2 | TS-08-P2 | 4.3 | parking-operator-adaptor::proptest_idempotent_lock_events |
| Property 3 | TS-08-P3 | 4.3 | parking-operator-adaptor::proptest_override_non_persistence |
| Property 4 | TS-08-P4 | 3.2 | parking-operator-adaptor::proptest_retry_exhaustion_safety |
| Property 5 | TS-08-P5 | 4.3 | parking-operator-adaptor::proptest_session_active_consistency |
| Property 6 | TS-08-P6 | 4.3 | parking-operator-adaptor::proptest_sequential_event_processing |

## Notes

- Unit tests for config and session modules are pure-function tests with no external dependencies.
- The operator module tests use wiremock (or similar) for mock HTTP server — no real PARKING_OPERATOR needed.
- Event loop tests use mock OperatorClient and mock BrokerClient trait implementations to test event processing logic in isolation.
- Property tests use the `proptest` crate. Annotate with `#[ignore]` if slow, run separately via `cargo test -- --include-ignored proptest`.
- Integration tests live in `tests/parking-operator-adaptor/` as a Go module (consistent with other spec patterns). They start the adaptor binary, a mock PARKING_OPERATOR HTTP server, and the DATA_BROKER container.
- Integration tests require Podman and skip gracefully when unavailable.
- The kuksa.val.v1 proto files are vendored into the parking-operator-adaptor crate (Rust proto codegen is crate-local via build.rs).
- The parking_adaptor.proto is also vendored from spec 01 group 6 definitions.
- Task group 1 has 6 subtasks — at the upper limit. Each subtask creates focused test files for one module.
- Dependencies: spec 01 group 3 (workspace skeleton), spec 01 group 6 (proto definitions), spec 02 group 2 (DATA_BROKER for integration tests).
