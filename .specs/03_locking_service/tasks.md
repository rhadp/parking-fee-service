# Tasks: LOCKING_SERVICE (Spec 03)

> Implementation tasks for the LOCKING_SERVICE component.
> Implements requirements from `.specs/03_locking_service/requirements.md`.
> Follows the design in `.specs/03_locking_service/design.md`.
> Verified by tests in `.specs/03_locking_service/test_spec.md`.

## Dependencies

| Spec | Dependency | Relationship |
|------|-----------|--------------|
| 01_project_setup | Groups 2, 4 | Requires Rust workspace with `locking-service` crate skeleton; requires Kuksa Databroker in `docker-compose.yml` via `make infra-up` |
| 02_data_broker | Groups 2, 3 | Requires running DATA_BROKER with VSS signals configured (standard + custom overlay) |

## Test Commands

| Action | Command |
|--------|---------|
| Unit tests | `cd rhivos && cargo test -p locking-service` |
| Integration tests | Requires running Kuksa Databroker (`make infra-up`), then `cd rhivos && cargo test -p locking-service --features integration` |
| Lint | `cd rhivos && cargo clippy -p locking-service` |
| Build | `cd rhivos && cargo build -p locking-service` |

---

## Group 1: Write Failing Spec Tests

**Goal:** Establish test scaffolding that encodes the expected behavior from the requirements and test spec. All tests should compile but fail (red phase of red-green-refactor).

### Task 1.1: Unit test scaffolding for configuration

Create `src/config.rs` tests:
- Test that `DATABROKER_UDS_PATH` defaults to `/tmp/kuksa/databroker.sock` when unset.
- Test that `DATABROKER_UDS_PATH` is parsed from environment when set.

**Covers:** TS-03-1 (preconditions)

### Task 1.2: Unit test scaffolding for command parsing and validation

Create `src/command.rs` tests:
- Test valid lock command JSON parses successfully.
- Test valid unlock command JSON parses successfully.
- Test malformed JSON returns a parse error.
- Test JSON missing `action` field returns a validation error.
- Test JSON missing `command_id` field returns a validation error.
- Test JSON with invalid `action` value (e.g., `"reboot"`) returns a validation error.
- Test success response serializes to the expected JSON format.
- Test failure response serializes to the expected JSON format.

**Covers:** TS-03-E1, TS-03-E2, TS-03-E3, TS-03-E4

### Task 1.3: Unit test scaffolding for safety validation

Create `src/safety.rs` tests:
- Test that speed == 0.0 and door closed passes safety check.
- Test that speed >= 1.0 fails safety check with reason `"vehicle_moving"`.
- Test that door open == true fails safety check with reason `"door_ajar"`.
- Test that both constraints violated returns the first failure (speed checked first).

**Covers:** TS-03-P1, TS-03-P3

### Task 1.4: Integration test scaffolding

Create `tests/integration.rs` with `#[cfg(feature = "integration")]` gated tests:
- Test lock command happy path (TS-03-1).
- Test unlock command happy path (TS-03-2).
- Test safety rejection for vehicle moving (TS-03-3).
- Test safety rejection for door ajar (TS-03-4).
- Test invalid JSON handling (TS-03-E1).
- Test missing fields handling (TS-03-E2).
- Test invalid action handling (TS-03-E3).

All tests should assert expected outcomes but fail because the implementation does not exist yet.

### Task 1.5: Add `integration` feature flag to Cargo.toml

Add a Cargo feature `integration` (no dependencies, used only for `#[cfg(feature = "integration")]` gating).

**Exit criteria:** `cargo test -p locking-service` compiles. All unit tests fail. `cargo test -p locking-service --features integration` compiles (with infra running). All integration tests fail.

---

## Group 2: DATA_BROKER gRPC Client

**Goal:** Implement the gRPC client that connects to DATA_BROKER via UDS and provides subscribe, get, and set operations.

### Task 2.1: Implement `config.rs`

Parse environment variable `DATABROKER_UDS_PATH`. Return a `Config` struct. Apply default value `/tmp/kuksa/databroker.sock` when unset.

**Covers:** 03-REQ-1.1 (config portion)

### Task 2.2: Implement `databroker_client.rs`

- Create a tonic gRPC client that connects to DATA_BROKER via Unix Domain Socket at the configured path.
- Implement `subscribe_signal(path) -> Stream<SignalUpdate>` for subscribing to signal changes.
- Implement `get_signal(path) -> Option<Value>` for reading a signal's current value.
- Implement `set_signal(path, value)` for writing a signal value (bool or string).
- Handle connection errors with retry and exponential backoff (1s, 2s, 4s, ..., max 30s).

**Covers:** 03-REQ-1.1, 03-REQ-1.2

### Task 2.3: Implement `main.rs` startup

- Load configuration.
- Connect to DATA_BROKER via gRPC/UDS with retry.
- Subscribe to `Vehicle.Command.Door.Lock`.
- Install SIGTERM/SIGINT handler for graceful shutdown.
- Log "LOCKING_SERVICE started".

**Covers:** 03-REQ-1.1, 03-REQ-1.2, 03-REQ-7.1

**Exit criteria:** `cargo build -p locking-service` succeeds. Unit tests for config pass. The binary connects to DATA_BROKER (with `make infra-up`) and subscribes to the command signal. SIGTERM causes a clean exit with code 0.

---

## Group 3: Command Parsing and Validation

**Goal:** Implement command JSON parsing, field validation, and response construction.

### Task 3.1: Implement `command.rs` -- Command struct and parsing

Define `Command` struct with serde deserialization. Define `CommandResponse` struct with serde serialization. Implement `Command::from_json(json_str) -> Result<Command, ValidationError>`.

**Covers:** 03-REQ-2.1

### Task 3.2: Implement `command.rs` -- Validation logic

Implement validation:
- All required fields present and non-empty: `command_id`, `action`, `doors`, `source`, `vin`, `timestamp`.
- `action` must be `"lock"` or `"unlock"`.
- Return structured errors on failure.

**Covers:** 03-REQ-2.1, 03-REQ-2.2, 03-REQ-6.1

### Task 3.3: Implement `command.rs` -- Response construction

Implement helper methods:
- `CommandResponse::success(command_id, timestamp) -> CommandResponse`
- `CommandResponse::failure(command_id, reason, timestamp) -> CommandResponse`
- `CommandResponse::to_json(&self) -> String`

**Covers:** 03-REQ-5.1, 03-REQ-5.2

**Exit criteria:** All unit tests for command parsing and validation pass. `Command::from_json` correctly parses valid commands and rejects invalid ones. Response serialization produces the expected JSON format.

---

## Group 4: Safety Checks and Lock/Unlock Execution

**Goal:** Implement safety constraint validation and the lock/unlock execution pipeline. Wire everything together in the main processing loop.

### Task 4.1: Implement `safety.rs`

- Implement `check_safety(databroker_client) -> Result<(), String>`:
  - Read `Vehicle.Speed` from DATA_BROKER. If speed >= 1.0, return `Err("vehicle_moving")`.
  - Read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from DATA_BROKER. If `true`, return `Err("door_ajar")`.
  - If signal has no current value (not yet set), treat as safe (speed = 0, door closed).
  - Return `Ok(())` if all constraints pass.

**Covers:** 03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3

### Task 4.2: Implement `executor.rs`

- Implement `execute_command(databroker_client, command) -> Result<(), Error>`:
  - On `"lock"`: write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true`.
  - On `"unlock"`: write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false`.
- Implement `write_response(databroker_client, response) -> Result<(), Error>`:
  - Serialize `CommandResponse` to JSON and write to `Vehicle.Command.Door.Response`.

**Covers:** 03-REQ-4.1, 03-REQ-4.2, 03-REQ-5.1, 03-REQ-5.2

### Task 4.3: Wire processing pipeline in `main.rs`

Integrate the full processing loop in `main.rs`:
1. Receive signal update from `Vehicle.Command.Door.Lock` subscription stream.
2. Parse JSON using `Command::from_json`. On failure: write failure response (`"invalid_command"`), continue.
3. Validate action. On unknown action: write failure response (`"invalid_action"`), continue.
4. Call `check_safety`. On constraint violation: write failure response with reason, continue.
5. Call `execute_command` to update lock state.
6. Write success response.

**Covers:** 03-REQ-2.1, 03-REQ-2.2, 03-REQ-3.1, 03-REQ-3.2, 03-REQ-3.3, 03-REQ-4.1, 03-REQ-4.2, 03-REQ-5.1, 03-REQ-5.2, 03-REQ-6.1

### Task 4.4: Implement DATA_BROKER connection recovery

Detect broken DATA_BROKER streams during operation. Log error and retry connection with exponential backoff. Re-establish subscription to `Vehicle.Command.Door.Lock` after reconnection.

**Covers:** 03-REQ-8.1

**Exit criteria:** All unit tests for safety validation pass. With infra running, sending a lock command with speed = 0 and door closed results in `IsLocked = true` and a success response. Sending a command with speed > 0 results in a `"vehicle_moving"` failure response. Sending a command with door open results in a `"door_ajar"` failure response. Invalid commands produce appropriate failure responses.

---

## Group 5: Integration Testing

**Goal:** Run full integration tests against a real DATA_BROKER. Verify all command processing paths end-to-end.

### Task 5.1: Verify lock command happy path (TS-03-1)

Run the integration test that writes a lock command to DATA_BROKER and verifies `IsLocked = true` and a success response. Fix any issues found.

### Task 5.2: Verify unlock command happy path (TS-03-2)

Run the integration test that writes an unlock command and verifies `IsLocked = false` and a success response. Fix any issues found.

### Task 5.3: Verify safety constraint rejection -- vehicle moving (TS-03-3)

Run the integration test that sets speed > 0, sends a lock command, and verifies the `"vehicle_moving"` failure response. Fix any issues found.

### Task 5.4: Verify safety constraint rejection -- door ajar (TS-03-4)

Run the integration test that sets door open, sends a lock command, and verifies the `"door_ajar"` failure response. Fix any issues found.

### Task 5.5: Verify invalid command handling (TS-03-E1, TS-03-E2, TS-03-E3)

Run integration tests for malformed JSON, missing fields, and invalid action values. Verify appropriate failure responses. Fix any issues found.

### Task 5.6: Verify response format (TS-03-E4)

Run integration tests that validate the exact JSON structure of success and failure responses. Fix any issues found.

### Task 5.7: Verify property-based safety invariants (TS-03-P1, TS-03-P2, TS-03-P3)

Run property tests that verify:
- No state change when any safety constraint is violated.
- Every command produces exactly one response.
- Successful execution implies all safety constraints were satisfied.

Fix any issues found.

**Exit criteria:** All integration tests pass. `cargo clippy -p locking-service` reports no warnings. `cargo test -p locking-service` passes. `cargo test -p locking-service --features integration` passes (with infra running).

---

## Group 6: Checkpoint

**Goal:** Final validation that all requirements are met, all tests pass, and the component is ready for integration with other specs.

### Task 6.1: Full build and test run

Run in sequence:
1. `cd rhivos && cargo build -p locking-service`
2. `cd rhivos && cargo clippy -p locking-service`
3. `cd rhivos && cargo test -p locking-service`
4. `make infra-up`
5. `cd rhivos && cargo test -p locking-service --features integration`

Confirm all steps pass with zero errors and zero warnings.

### Task 6.2: Manual smoke test

1. Start infrastructure: `make infra-up`.
2. Run `cargo run -p locking-service`.
3. Using a DATA_BROKER client (e.g., `databroker-cli` or `grpcurl`):
   a. Set `Vehicle.Speed = 0.0` and `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = false`.
   b. Write a lock command to `Vehicle.Command.Door.Lock`.
   c. Verify `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true`.
   d. Verify `Vehicle.Command.Door.Response` contains a success response.
   e. Set `Vehicle.Speed = 50.0`.
   f. Write another lock command.
   g. Verify the response contains `"vehicle_moving"`.
4. Send SIGTERM to the process and verify it exits with code 0.

### Task 6.3: Requirements coverage review

Verify every requirement in `requirements.md` has at least one passing test in `test_spec.md`.

**Exit criteria:** All build, lint, and test steps pass. Manual smoke test demonstrates end-to-end functionality. Requirements coverage is complete.

---

## Traceability Matrix

| Requirement | Task Group(s) | Test(s) |
|-------------|--------------|---------|
| 03-REQ-1.1 | G2 (2.1, 2.2, 2.3) | TS-03-1, TS-03-2 (preconditions) |
| 03-REQ-1.2 | G2 (2.2) | TS-03-1 (preconditions) |
| 03-REQ-2.1 | G3 (3.1, 3.2) | TS-03-1, TS-03-2, TS-03-E2 |
| 03-REQ-2.2 | G3 (3.2), G4 (4.3) | TS-03-E1, TS-03-E2 |
| 03-REQ-3.1 | G4 (4.1) | TS-03-3, TS-03-P1, TS-03-P3 |
| 03-REQ-3.2 | G4 (4.1) | TS-03-4, TS-03-P1, TS-03-P3 |
| 03-REQ-3.3 | G4 (4.1, 4.3) | TS-03-3, TS-03-4, TS-03-P1 |
| 03-REQ-4.1 | G4 (4.2, 4.3) | TS-03-1, TS-03-P1 |
| 03-REQ-4.2 | G4 (4.2, 4.3) | TS-03-2, TS-03-P1 |
| 03-REQ-5.1 | G3 (3.3), G4 (4.2, 4.3) | TS-03-1, TS-03-2, TS-03-E4, TS-03-P2 |
| 03-REQ-5.2 | G3 (3.3), G4 (4.2, 4.3) | TS-03-3, TS-03-4, TS-03-E1, TS-03-E2, TS-03-E3, TS-03-E4, TS-03-P2 |
| 03-REQ-6.1 | G3 (3.2), G4 (4.3) | TS-03-E3 |
| 03-REQ-7.1 | G2 (2.3) | (manual smoke test) |
| 03-REQ-8.1 | G4 (4.4) | (manual smoke test) |
