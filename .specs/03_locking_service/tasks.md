# Implementation Plan: LOCKING_SERVICE

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the LOCKING_SERVICE as a Rust binary in `rhivos/locking-service/`. The service subscribes to lock/unlock commands from DATA_BROKER, validates safety constraints, and publishes state/response signals. Task group 1 writes failing tests. Groups 2-3 implement core logic (command parsing, safety checks, response building). Group 4 implements the DATA_BROKER client and main loop. Group 5 runs integration tests.

Ordering: tests first (TDD), then pure-function modules (no external dependencies), then the broker client, then the async main loop, then integration validation.

## Test Commands

- Spec tests (unit): `cd rhivos && cargo test -p locking-service`
- Spec tests (integration): `cd tests/locking-service && go test -v ./...`
- Property tests: `cd rhivos && cargo test -p locking-service -- --include-ignored proptest`
- All Rust tests: `cd rhivos && cargo test`
- Linter: `cd rhivos && cargo clippy -p locking-service -- -D warnings`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Add dependencies to locking-service Cargo.toml
    - Add: serde, serde_json, tokio, tonic, prost, tracing, tracing-subscriber, proptest (dev)
    - Vendor kuksa.val.v1 proto definitions into `rhivos/locking-service/proto/` (or `proto/kuksa/`)
    - Add tonic-build to build.rs for proto code generation
    - _Test Spec: TS-03-1 through TS-03-18_

  - [x] 1.2 Write command parsing and validation unit tests
    - Create `rhivos/locking-service/src/command.rs` with test module
    - `test_parse_valid_command` — TS-03-2
    - `test_validate_empty_command_id` — TS-03-4
    - `test_validate_invalid_action` — TS-03-5
    - `test_validate_unsupported_door` — TS-03-6
    - `test_parse_invalid_json` — TS-03-E3
    - `test_parse_missing_field` — TS-03-E4
    - `test_validate_non_driver_door` — TS-03-E5
    - _Test Spec: TS-03-2, TS-03-4, TS-03-5, TS-03-6, TS-03-E3, TS-03-E4, TS-03-E5_

  - [x] 1.3 Write safety check unit tests
    - Create `rhivos/locking-service/src/safety.rs` with test module
    - `test_lock_rejected_vehicle_moving` — TS-03-7
    - `test_lock_rejected_door_open` — TS-03-8
    - `test_lock_allowed_safe` — TS-03-9
    - `test_unlock_bypasses_safety` — TS-03-10
    - `test_speed_unset_treated_zero` — TS-03-E6
    - `test_door_unset_treated_closed` — TS-03-E7
    - _Test Spec: TS-03-7, TS-03-8, TS-03-9, TS-03-10, TS-03-E6, TS-03-E7_

  - [x] 1.4 Write response builder and state management tests
    - Create `rhivos/locking-service/src/response.rs` with test module
    - `test_success_response_format` — TS-03-14
    - `test_failure_response_format` — TS-03-15
    - `test_response_timestamp` — TS-03-16
    - `test_lock_sets_state_true` — TS-03-11
    - `test_unlock_sets_state_false` — TS-03-12
    - `test_lock_idempotent` — TS-03-E8
    - `test_unlock_idempotent` — TS-03-E9
    - `test_response_publish_failure` — TS-03-E10
    - _Test Spec: TS-03-11, TS-03-12, TS-03-14, TS-03-15, TS-03-16, TS-03-E8, TS-03-E9, TS-03-E10_

  - [x] 1.5 Write property tests
    - Create property tests in relevant modules or `tests/` directory
    - `proptest_command_validation_completeness` — TS-03-P1
    - `proptest_safety_gate_lock` — TS-03-P2
    - `proptest_unlock_always_succeeds` — TS-03-P3
    - `proptest_state_response_consistency` — TS-03-P4
    - `proptest_idempotent_operations` — TS-03-P5
    - `proptest_response_completeness` — TS-03-P6
    - _Test Spec: TS-03-P1 through TS-03-P6_

  - [x] 1.6 Write config test
    - `test_databroker_addr_default` — TS-03-3
    - `test_databroker_addr_env` — TS-03-3
    - _Test Spec: TS-03-3_

  - [x] 1.V Verify task group 1
    - [x] All test files compile: `cd rhivos && cargo test -p locking-service --no-run`
    - [x] All unit tests FAIL (red): `cd rhivos && cargo test -p locking-service 2>&1 | grep FAILED`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p locking-service -- -D warnings`

- [ ] 2. Command parsing and response modules
  - [ ] 2.1 Implement command module
    - Define `LockCommand`, `Action` structs/enums with serde Deserialize
    - Implement `parse_command()`: deserialize JSON, return Result
    - Implement `validate_command()`: check command_id non-empty, doors contains "driver"
    - _Requirements: 03-REQ-2.1, 03-REQ-2.2, 03-REQ-2.3_

  - [ ] 2.2 Implement response module
    - Define `CommandResponse` struct with serde Serialize
    - Implement `success_response(command_id)`: returns JSON string
    - Implement `failure_response(command_id, reason)`: returns JSON string
    - Timestamps use `std::time::SystemTime::now()` as Unix seconds
    - _Requirements: 03-REQ-5.1, 03-REQ-5.2, 03-REQ-5.3_

  - [ ] 2.3 Implement config module
    - Read `DATABROKER_ADDR` from env, default to `http://localhost:55556`
    - _Requirements: 03-REQ-1.3_

  - [ ] 2.V Verify task group 2
    - [ ] Command and response tests pass: `cd rhivos && cargo test -p locking-service -- command response config`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p locking-service -- -D warnings`
    - [ ] _Test Spec: TS-03-2, TS-03-3, TS-03-4, TS-03-5, TS-03-6, TS-03-14, TS-03-15, TS-03-16, TS-03-E3, TS-03-E4, TS-03-E5_

- [ ] 3. Safety checks and state management
  - [ ] 3.1 Define BrokerClient trait
    - Define `BrokerClient` trait with async methods: `get_float`, `get_bool`, `set_bool`, `set_string`, `subscribe`
    - This trait enables mock testing of safety and state logic
    - _Requirements: 03-REQ-1.1 (interface)_

  - [ ] 3.2 Implement safety module
    - Implement `check_safety(broker)`: reads Vehicle.Speed and IsOpen signals
    - Speed < 1.0 and IsOpen == false → Safe
    - Speed >= 1.0 → VehicleMoving
    - IsOpen == true → DoorOpen
    - None values treated as safe defaults (speed=0.0, door=closed)
    - _Requirements: 03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3, 03-REQ-3.E1, 03-REQ-3.E2_

  - [ ] 3.3 Implement command processor
    - Implement `process_command(broker, cmd, lock_state)` orchestrating: validate → safety check (lock only) → update state → publish response
    - Handles idempotent operations (lock when locked, unlock when unlocked)
    - Unlock bypasses safety checks
    - _Requirements: 03-REQ-3.4, 03-REQ-4.1, 03-REQ-4.2, 03-REQ-4.E1, 03-REQ-4.E2_

  - [ ] 3.4 Create mock BrokerClient for tests
    - Implement mock that returns configurable speed/door values
    - Records set_bool and set_string calls for assertion
    - _Test Spec: TS-03-7 through TS-03-12, TS-03-E6 through TS-03-E10_

  - [ ] 3.V Verify task group 3
    - [ ] Safety and state tests pass: `cd rhivos && cargo test -p locking-service -- safety process`
    - [ ] Property tests pass: `cd rhivos && cargo test -p locking-service -- proptest`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p locking-service -- -D warnings`
    - [ ] _Test Spec: TS-03-7 through TS-03-12, TS-03-E6 through TS-03-E10, TS-03-P1 through TS-03-P6_

- [ ] 4. Checkpoint - Core Logic Complete
  - All unit and property tests pass
  - No integration tests yet (those require live DATA_BROKER)
  - Ask the user if questions arise

- [ ] 5. DATA_BROKER gRPC client and main loop
  - [ ] 5.1 Vendor Kuksa proto definitions
    - Add `kuksa.val.v1` proto files to `rhivos/locking-service/proto/`
    - Configure `build.rs` with tonic-build to generate Rust code from protos
    - _Requirements: 03-REQ-1.1 (gRPC client)_

  - [ ] 5.2 Implement real BrokerClient
    - Implement the `BrokerClient` trait using tonic-generated kuksa.val.v1 client
    - `connect(addr)`: establish gRPC channel
    - `subscribe(signal)`: create kuksa Subscribe stream
    - `get_float`, `get_bool`: kuksa Get with type conversion
    - `set_bool`, `set_string`: kuksa Set with type conversion
    - _Requirements: 03-REQ-1.1, 03-REQ-1.3_

  - [ ] 5.3 Implement main loop
    - Parse config, connect to DATA_BROKER with retry logic
    - Publish initial lock state (IsLocked = false)
    - Subscribe to Vehicle.Command.Door.Lock
    - Process commands sequentially via process_command()
    - Handle SIGTERM/SIGINT via tokio signal handler
    - Log version, address, and ready status on startup
    - _Requirements: 03-REQ-1.1, 03-REQ-1.2, 03-REQ-4.3, 03-REQ-6.1, 03-REQ-6.2_

  - [ ] 5.4 Implement retry logic
    - Connection retry: exponential backoff 1s, 2s, 4s, up to 5 attempts
    - Subscription retry: up to 3 resubscribe attempts
    - Response publish failure: log and continue
    - _Requirements: 03-REQ-1.E1, 03-REQ-1.E2, 03-REQ-5.E1_

  - [ ] 5.V Verify task group 5
    - [ ] Binary compiles: `cd rhivos && cargo build -p locking-service`
    - [ ] All unit tests still pass: `cd rhivos && cargo test -p locking-service`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p locking-service -- -D warnings`

- [ ] 6. Integration test validation
  - [ ] 6.1 Create integration test module
    - Create `tests/locking-service/` Go module (or add to existing test structure)
    - Shared helpers: start/stop databroker, start/stop locking-service, gRPC signal helpers
    - Add `go.work` entry for `./tests/locking-service`
    - _Test Spec: TS-03-1, TS-03-13, TS-03-17, TS-03-18_

  - [ ] 6.2 Write and run integration tests
    - `TestCommandSubscription` — TS-03-1: verify subscription and command processing
    - `TestInitialState` — TS-03-13: verify IsLocked=false on startup
    - `TestGracefulShutdown` — TS-03-17: verify clean exit on SIGTERM
    - `TestStartupLogging` — TS-03-18: verify log output
    - `TestDatabrokerUnreachable` — TS-03-E1: verify retry behavior
    - `TestSubscriptionInterrupted` — TS-03-E2: verify resubscribe
    - `TestSigtermDuringCommand` — TS-03-E11: verify in-flight completion
    - _Test Spec: TS-03-1, TS-03-13, TS-03-17, TS-03-18, TS-03-E1, TS-03-E2, TS-03-E11_

  - [ ] 6.V Verify task group 6
    - [ ] All integration tests pass: `cd tests/locking-service && go test -v ./...`
    - [ ] All unit tests still pass: `cd rhivos && cargo test -p locking-service`
    - [ ] All existing tests still pass: `make test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p locking-service -- -D warnings`
    - [ ] All requirements 03-REQ-1 through 03-REQ-6 acceptance criteria met

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
| 03-REQ-1.1 | TS-03-1 | 5.2, 5.3 | tests/locking-service::TestCommandSubscription |
| 03-REQ-1.2 | TS-03-2 | 2.1 | locking-service::command::test_parse_valid_command |
| 03-REQ-1.3 | TS-03-3 | 2.3 | locking-service::config::test_databroker_addr_default |
| 03-REQ-1.E1 | TS-03-E1 | 5.4 | tests/locking-service::TestDatabrokerUnreachable |
| 03-REQ-1.E2 | TS-03-E2 | 5.4 | tests/locking-service::TestSubscriptionInterrupted |
| 03-REQ-2.1 | TS-03-4 | 2.1 | locking-service::command::test_validate_empty_command_id |
| 03-REQ-2.2 | TS-03-5 | 2.1 | locking-service::command::test_validate_invalid_action |
| 03-REQ-2.3 | TS-03-6 | 2.1 | locking-service::command::test_validate_unsupported_door |
| 03-REQ-2.E1 | TS-03-E3 | 2.1 | locking-service::command::test_parse_invalid_json |
| 03-REQ-2.E2 | TS-03-E4 | 2.1 | locking-service::command::test_parse_missing_field |
| 03-REQ-2.E3 | TS-03-E5 | 2.1 | locking-service::command::test_validate_non_driver_door |
| 03-REQ-3.1 | TS-03-7 | 3.2 | locking-service::safety::test_lock_rejected_vehicle_moving |
| 03-REQ-3.2 | TS-03-8 | 3.2 | locking-service::safety::test_lock_rejected_door_open |
| 03-REQ-3.3 | TS-03-9 | 3.2 | locking-service::safety::test_lock_allowed_safe |
| 03-REQ-3.4 | TS-03-10 | 3.3 | locking-service::safety::test_unlock_bypasses_safety |
| 03-REQ-3.E1 | TS-03-E6 | 3.2 | locking-service::safety::test_speed_unset_treated_zero |
| 03-REQ-3.E2 | TS-03-E7 | 3.2 | locking-service::safety::test_door_unset_treated_closed |
| 03-REQ-4.1 | TS-03-11 | 3.3 | locking-service::test_lock_sets_state_true |
| 03-REQ-4.2 | TS-03-12 | 3.3 | locking-service::test_unlock_sets_state_false |
| 03-REQ-4.3 | TS-03-13 | 5.3 | tests/locking-service::TestInitialState |
| 03-REQ-4.E1 | TS-03-E8 | 3.3 | locking-service::test_lock_idempotent |
| 03-REQ-4.E2 | TS-03-E9 | 3.3 | locking-service::test_unlock_idempotent |
| 03-REQ-5.1 | TS-03-14 | 2.2 | locking-service::response::test_success_response_format |
| 03-REQ-5.2 | TS-03-15 | 2.2 | locking-service::response::test_failure_response_format |
| 03-REQ-5.3 | TS-03-16 | 2.2 | locking-service::response::test_response_timestamp |
| 03-REQ-5.E1 | TS-03-E10 | 5.4 | locking-service::test_response_publish_failure |
| 03-REQ-6.1 | TS-03-17 | 5.3 | tests/locking-service::TestGracefulShutdown |
| 03-REQ-6.2 | TS-03-18 | 5.3 | tests/locking-service::TestStartupLogging |
| 03-REQ-6.E1 | TS-03-E11 | 5.3 | tests/locking-service::TestSigtermDuringCommand |
| Property 1 | TS-03-P1 | 2.1 | locking-service::proptest_command_validation_completeness |
| Property 2 | TS-03-P2 | 3.2 | locking-service::proptest_safety_gate_lock |
| Property 3 | TS-03-P3 | 3.3 | locking-service::proptest_unlock_always_succeeds |
| Property 4 | TS-03-P4 | 3.3 | locking-service::proptest_state_response_consistency |
| Property 5 | TS-03-P5 | 3.3 | locking-service::proptest_idempotent_operations |
| Property 6 | TS-03-P6 | 3.3 | locking-service::proptest_response_completeness |

## Notes

- Unit tests for command, safety, and response modules are pure-function tests with no external dependencies. They use serde_json for payload testing and a mock BrokerClient for safety/state tests.
- The mock BrokerClient records calls to `set_bool` and `set_string` so tests can assert on side effects without a live DATA_BROKER.
- Property tests use the `proptest` crate. Annotate with `#[ignore]` if they are slow, and run separately via `cargo test -- --include-ignored proptest`.
- Integration tests live in `tests/locking-service/` as a Go module (consistent with spec 01 and 02 patterns). They shell out to start/stop the locking-service binary and the databroker container.
- Integration tests require Podman and skip gracefully when unavailable.
- The kuksa.val.v1 proto files are vendored into the locking-service crate rather than shared, since Rust proto codegen is crate-local via build.rs (unlike Go, where generated code is shared via gen/go/).
- Task group 1 has 6 subtasks — at the upper limit. Each subtask creates a focused test file for one module, so they are individually small.
