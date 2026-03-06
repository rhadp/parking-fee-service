# Implementation Plan: LOCKING_SERVICE (Spec 03)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the LOCKING_SERVICE component that subscribes to lock/unlock commands from DATA_BROKER, validates safety constraints (vehicle speed and door state), executes lock/unlock operations, and writes command responses. Task group 1 writes all failing spec tests. Groups 2-4 implement functionality to make those tests pass. Group 5 runs integration tests. Group 6 is the final checkpoint.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Rust workspace with `locking-service` crate skeleton; requires Kuksa Databroker in `docker-compose.yml` via `make infra-up` |
| 02_data_broker | 3 | 1 | Requires running DATA_BROKER with VSS signals configured (standard + custom overlay) |

## Test Commands

- Unit tests: `cd rhivos && cargo test -p locking-service`
- Integration tests: `make infra-up && cd rhivos && cargo test -p locking-service --features integration`
- Lint: `cd rhivos && cargo clippy -p locking-service`
- Build: `cd rhivos && cargo build -p locking-service`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Unit test scaffolding for configuration
    - Create `src/config.rs` tests
    - Test that `DATABROKER_UDS_PATH` defaults to `/tmp/kuksa/databroker.sock` when unset
    - Test that `DATABROKER_UDS_PATH` is parsed from environment when set
    - _Test Spec: TS-03-1 (preconditions)_

  - [x] 1.2 Unit test scaffolding for command parsing and validation
    - Create `src/command.rs` tests
    - Test valid lock command JSON parses successfully
    - Test valid unlock command JSON parses successfully
    - Test malformed JSON returns a parse error
    - Test JSON missing `action` field returns a validation error
    - Test JSON missing `command_id` field returns a validation error
    - Test JSON with invalid `action` value (e.g., `"reboot"`) returns a validation error
    - Test success response serializes to the expected JSON format
    - Test failure response serializes to the expected JSON format
    - _Test Spec: TS-03-E1, TS-03-E2, TS-03-E3, TS-03-E4_

  - [x] 1.3 Unit test scaffolding for safety validation
    - Create `src/safety.rs` tests
    - Test that speed == 0.0 and door closed passes safety check
    - Test that speed >= 1.0 fails safety check with reason `"vehicle_moving"`
    - Test that door open == true fails safety check with reason `"door_ajar"`
    - Test that both constraints violated returns the first failure (speed checked first)
    - _Test Spec: TS-03-P1, TS-03-P3_

  - [x] 1.4 Integration test scaffolding
    - Create `tests/integration.rs` with `#[cfg(feature = "integration")]` gated tests
    - Test lock command happy path (TS-03-1)
    - Test unlock command happy path (TS-03-2)
    - Test safety rejection for vehicle moving (TS-03-3)
    - Test safety rejection for door ajar (TS-03-4)
    - Test invalid JSON handling (TS-03-E1)
    - Test missing fields handling (TS-03-E2)
    - Test invalid action handling (TS-03-E3)
    - All tests should assert expected outcomes but fail because the implementation does not exist yet
    - _Test Spec: TS-03-1, TS-03-2, TS-03-3, TS-03-4, TS-03-E1, TS-03-E2, TS-03-E3_

  - [x] 1.5 Add `integration` feature flag to Cargo.toml
    - Add a Cargo feature `integration` (no dependencies, used only for `#[cfg(feature = "integration")]` gating)

  - [x] 1.V Verify task group 1
    - [x] `cargo test -p locking-service` compiles; all unit tests fail
    - [x] `cargo test -p locking-service --features integration` compiles (with infra running); all integration tests fail
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p locking-service`

- [x] 2. DATA_BROKER gRPC client
  - [x] 2.1 Implement `config.rs`
    - Parse environment variable `DATABROKER_UDS_PATH`
    - Return a `Config` struct
    - Apply default value `/tmp/kuksa/databroker.sock` when unset
    - _Requirements: 03-REQ-1.1_

  - [x] 2.2 Implement `databroker_client.rs`
    - Create a tonic gRPC client that connects to DATA_BROKER via Unix Domain Socket at the configured path
    - Implement `subscribe_signal(path) -> Stream<SignalUpdate>` for subscribing to signal changes
    - Implement `get_signal(path) -> Option<Value>` for reading a signal's current value
    - Implement `set_signal(path, value)` for writing a signal value (bool or string)
    - Handle connection errors with retry and exponential backoff (1s, 2s, 4s, ..., max 30s)
    - _Requirements: 03-REQ-1.1, 03-REQ-1.2_

  - [x] 2.3 Implement `main.rs` startup
    - Load configuration
    - Connect to DATA_BROKER via gRPC/UDS with retry
    - Subscribe to `Vehicle.Command.Door.Lock`
    - Install SIGTERM/SIGINT handler for graceful shutdown
    - Log "LOCKING_SERVICE started"
    - _Requirements: 03-REQ-1.1, 03-REQ-1.2, 03-REQ-7.1_

  - [x] 2.V Verify task group 2
    - [x] Unit tests for config pass: `cd rhivos && cargo test -p locking-service`
    - [x] `cargo build -p locking-service` succeeds
    - [x] Binary connects to DATA_BROKER (with `make infra-up`) and subscribes to the command signal
    - [x] SIGTERM causes a clean exit with code 0
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p locking-service`

- [x] 3. Command parsing and validation
  - [x] 3.1 Implement `command.rs` -- Command struct and parsing
    - Define `Command` struct with serde deserialization
    - Define `CommandResponse` struct with serde serialization
    - Implement `Command::from_json(json_str) -> Result<Command, ValidationError>`
    - _Requirements: 03-REQ-2.1_

  - [x] 3.2 Implement `command.rs` -- Validation logic
    - All required fields present and non-empty: `command_id`, `action`, `doors`, `source`, `vin`, `timestamp`
    - `action` must be `"lock"` or `"unlock"`
    - Return structured errors on failure
    - _Requirements: 03-REQ-2.1, 03-REQ-2.2, 03-REQ-6.1_

  - [x] 3.3 Implement `command.rs` -- Response construction
    - `CommandResponse::success(command_id, timestamp) -> CommandResponse`
    - `CommandResponse::failure(command_id, reason, timestamp) -> CommandResponse`
    - `CommandResponse::to_json(&self) -> String`
    - _Requirements: 03-REQ-5.1, 03-REQ-5.2_

  - [x] 3.V Verify task group 3
    - [x] All unit tests for command parsing and validation pass
    - [x] `Command::from_json` correctly parses valid commands and rejects invalid ones
    - [x] Response serialization produces the expected JSON format
    - [x] All existing tests still pass: `cd rhivos && cargo test -p locking-service`
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p locking-service`

- [ ] 4. Safety checks and lock/unlock execution
  - [ ] 4.1 Implement `safety.rs`
    - Implement `check_safety(databroker_client) -> Result<(), String>`
    - Read `Vehicle.Speed` from DATA_BROKER; if speed >= 1.0, return `Err("vehicle_moving")`
    - Read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from DATA_BROKER; if `true`, return `Err("door_ajar")`
    - If signal has no current value (not yet set), treat as safe (speed = 0, door closed)
    - Return `Ok(())` if all constraints pass
    - _Requirements: 03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3_

  - [ ] 4.2 Implement `executor.rs`
    - On `"lock"`: write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true`
    - On `"unlock"`: write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false`
    - Serialize `CommandResponse` to JSON and write to `Vehicle.Command.Door.Response`
    - _Requirements: 03-REQ-4.1, 03-REQ-4.2, 03-REQ-5.1, 03-REQ-5.2_

  - [ ] 4.3 Wire processing pipeline in `main.rs`
    - Receive signal update from `Vehicle.Command.Door.Lock` subscription stream
    - Parse JSON using `Command::from_json`; on failure: write failure response (`"invalid_command"`), continue
    - Validate action; on unknown action: write failure response (`"invalid_action"`), continue
    - Call `check_safety`; on constraint violation: write failure response with reason, continue
    - Call `execute_command` to update lock state
    - Write success response
    - _Requirements: 03-REQ-2.1, 03-REQ-2.2, 03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3, 03-REQ-4.1, 03-REQ-4.2, 03-REQ-5.1, 03-REQ-5.2, 03-REQ-6.1_

  - [ ] 4.4 Implement DATA_BROKER connection recovery
    - Detect broken DATA_BROKER streams during operation
    - Log error and retry connection with exponential backoff
    - Re-establish subscription to `Vehicle.Command.Door.Lock` after reconnection
    - _Requirements: 03-REQ-8.1_

  - [ ] 4.V Verify task group 4
    - [ ] All unit tests for safety validation pass
    - [ ] With infra running, lock command with speed = 0 and door closed results in `IsLocked = true` and a success response
    - [ ] Command with speed > 0 results in a `"vehicle_moving"` failure response
    - [ ] Command with door open results in a `"door_ajar"` failure response
    - [ ] Invalid commands produce appropriate failure responses
    - [ ] All existing tests still pass: `cd rhivos && cargo test -p locking-service`
    - [ ] No linter warnings introduced: `cd rhivos && cargo clippy -p locking-service`

- [ ] 5. Integration testing
  - [ ] 5.1 Verify lock command happy path (TS-03-1)
    - Run the integration test that writes a lock command to DATA_BROKER and verifies `IsLocked = true` and a success response; fix any issues found
    - _Test Spec: TS-03-1_

  - [ ] 5.2 Verify unlock command happy path (TS-03-2)
    - Run the integration test that writes an unlock command and verifies `IsLocked = false` and a success response; fix any issues found
    - _Test Spec: TS-03-2_

  - [ ] 5.3 Verify safety constraint rejection -- vehicle moving (TS-03-3)
    - Run the integration test that sets speed > 0, sends a lock command, and verifies the `"vehicle_moving"` failure response; fix any issues found
    - _Test Spec: TS-03-3_

  - [ ] 5.4 Verify safety constraint rejection -- door ajar (TS-03-4)
    - Run the integration test that sets door open, sends a lock command, and verifies the `"door_ajar"` failure response; fix any issues found
    - _Test Spec: TS-03-4_

  - [ ] 5.5 Verify invalid command handling (TS-03-E1, TS-03-E2, TS-03-E3)
    - Run integration tests for malformed JSON, missing fields, and invalid action values
    - Verify appropriate failure responses; fix any issues found
    - _Test Spec: TS-03-E1, TS-03-E2, TS-03-E3_

  - [ ] 5.6 Verify response format (TS-03-E4)
    - Run integration tests that validate the exact JSON structure of success and failure responses; fix any issues found
    - _Test Spec: TS-03-E4_

  - [ ] 5.7 Verify property-based safety invariants (TS-03-P1, TS-03-P2, TS-03-P3)
    - No state change when any safety constraint is violated
    - Every command produces exactly one response
    - Successful execution implies all safety constraints were satisfied
    - Fix any issues found
    - _Test Spec: TS-03-P1, TS-03-P2, TS-03-P3_

  - [ ] 5.V Verify task group 5
    - [ ] All integration tests pass: `cd rhivos && cargo test -p locking-service --features integration`
    - [ ] All existing tests still pass: `cd rhivos && cargo test -p locking-service`
    - [ ] No linter warnings introduced: `cd rhivos && cargo clippy -p locking-service`

- [ ] 6. Checkpoint
  - [ ] 6.1 Full build and test run
    - Run in sequence: `cargo build`, `cargo clippy`, `cargo test`, `make infra-up`, `cargo test --features integration`
    - Confirm all steps pass with zero errors and zero warnings

  - [ ] 6.2 Manual smoke test
    - Start infrastructure: `make infra-up`
    - Run `cargo run -p locking-service`
    - Set `Vehicle.Speed = 0.0` and `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = false`
    - Write a lock command to `Vehicle.Command.Door.Lock`
    - Verify `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true`
    - Verify `Vehicle.Command.Door.Response` contains a success response
    - Set `Vehicle.Speed = 50.0`, write another lock command, verify `"vehicle_moving"` response
    - Send SIGTERM and verify exit with code 0

  - [ ] 6.3 Requirements coverage review
    - Verify every requirement in `requirements.md` has at least one passing test

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 03-REQ-1.1 | TS-03-1, TS-03-2 | 2.1, 2.2, 2.3 | Unit tests for config, integration tests |
| 03-REQ-1.2 | TS-03-1 | 2.2 | Integration tests |
| 03-REQ-2.1 | TS-03-1, TS-03-2, TS-03-E2 | 3.1, 3.2 | Unit tests for command parsing |
| 03-REQ-2.2 | TS-03-E1, TS-03-E2 | 3.2, 4.3 | Unit + integration tests |
| 03-REQ-3.1 | TS-03-3, TS-03-P1, TS-03-P3 | 4.1 | Unit tests for safety, integration tests |
| 03-REQ-3.2 | TS-03-4, TS-03-P1, TS-03-P3 | 4.1 | Unit tests for safety, integration tests |
| 03-REQ-3.3 | TS-03-3, TS-03-4, TS-03-P1 | 4.1, 4.3 | Integration tests |
| 03-REQ-4.1 | TS-03-1, TS-03-P1 | 4.2, 4.3 | Integration tests |
| 03-REQ-4.2 | TS-03-2, TS-03-P1 | 4.2, 4.3 | Integration tests |
| 03-REQ-5.1 | TS-03-1, TS-03-2, TS-03-E4, TS-03-P2 | 3.3, 4.2, 4.3 | Unit + integration tests |
| 03-REQ-5.2 | TS-03-3, TS-03-4, TS-03-E1, TS-03-E2, TS-03-E3, TS-03-E4, TS-03-P2 | 3.3, 4.2, 4.3 | Unit + integration tests |
| 03-REQ-6.1 | TS-03-E3 | 3.2, 4.3 | Unit + integration tests |
| 03-REQ-7.1 | -- | 2.3 | Manual smoke test |
| 03-REQ-8.1 | -- | 4.4 | Manual smoke test |
