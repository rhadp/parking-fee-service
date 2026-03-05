# Tasks: CLOUD_GATEWAY_CLIENT (Spec 04)

> Implementation tasks for the CLOUD_GATEWAY_CLIENT component.
> Implements requirements from `.specs/04_cloud_gateway_client/requirements.md`.
> Follows the design in `.specs/04_cloud_gateway_client/design.md`.
> Verified by tests in `.specs/04_cloud_gateway_client/test_spec.md`.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Rust workspace with `cloud-gateway-client` crate skeleton; requires NATS + Kuksa in `docker-compose.yml` via `make infra-up` |
| 02_data_broker | 3 | 1 | Requires running DATA_BROKER with VSS signals configured (standard + custom overlay) |

## Test Commands

| Action | Command |
|--------|---------|
| Unit tests | `cd rhivos && cargo test -p cloud-gateway-client` |
| Integration tests | `make infra-up && cd rhivos && cargo test -p cloud-gateway-client --features integration` |
| Lint | `cd rhivos && cargo clippy -p cloud-gateway-client` |
| Build | `cd rhivos && cargo build -p cloud-gateway-client` |

---

## Group 1: Write Failing Spec Tests

**Goal:** Establish test scaffolding that encodes the expected behavior from the requirements and test spec. All tests should compile but fail (red phase of red-green-refactor).

### Task 1.1: Unit test scaffolding for configuration

Create `src/config.rs` tests:
- Test that `VIN` is parsed from environment.
- Test that missing `VIN` produces an error.
- Test that `NATS_URL` defaults to `nats://localhost:4222` when unset.
- Test that `NATS_TLS_ENABLED` defaults to `false` when unset.
- Test that `DATABROKER_UDS_PATH` defaults to `/tmp/kuksa/databroker.sock` when unset.

**Covers:** TS-04-2, TS-04-3

### Task 1.2: Unit test scaffolding for command validation

Create `src/command.rs` tests:
- Test valid command JSON parses and validates successfully.
- Test malformed JSON returns a parse error.
- Test JSON missing `action` field returns a validation error.
- Test JSON missing `command_id` field returns a validation error.
- Test JSON with invalid `action` value (e.g., `"reboot"`) returns a validation error.
- Test JSON with invalid `command_id` (not a UUID) returns a validation error.

**Covers:** TS-04-E1, TS-04-E2, TS-04-E3

### Task 1.3: Integration test scaffolding

Create `tests/integration.rs` with `#[cfg(feature = "integration")]` gated tests:
- Test NATS connection and command subscription (TS-04-1).
- Test command pipeline: NATS -> DATA_BROKER (TS-04-P1).
- Test response relay: DATA_BROKER -> NATS (TS-04-P2).
- Test telemetry publishing: DATA_BROKER -> NATS (TS-04-P3, TS-04-P4).
- Test full command round-trip (TS-04-P5).
- Test VIN isolation (TS-04-E5).

All tests should assert expected outcomes but fail because the implementation does not exist yet.

**Covers:** TS-04-1, TS-04-P1, TS-04-P2, TS-04-P3, TS-04-P4, TS-04-P5, TS-04-E5

### Task 1.4: Add `integration` feature flag to Cargo.toml

Add a Cargo feature `integration` (no dependencies, used only for `#[cfg(feature = "integration")]` gating).

**Exit criteria:** `cargo test -p cloud-gateway-client` compiles. All unit tests fail. `cargo test -p cloud-gateway-client --features integration` compiles (with infra running). All integration tests fail.

---

## Group 2: NATS Client (Connect and Subscribe to Commands)

**Goal:** Implement NATS connection management, including connecting, subscribing, publishing, and TLS support.

### Task 2.1: Implement `config.rs`

Parse environment variables: `VIN`, `NATS_URL`, `NATS_TLS_ENABLED`, `DATABROKER_UDS_PATH`. Return a `Config` struct. Exit with error if `VIN` is missing. Apply defaults for optional variables.

**Covers:** 04-REQ-1.1 (configuration)

### Task 2.2: Implement `nats_client.rs`

- Connect to NATS server using `async_nats::connect()` (plain) or `async_nats::ConnectOptions` with TLS (when `NATS_TLS_ENABLED=true`).
- Provide methods to subscribe to a subject and to publish to a subject.
- Leverage async-nats built-in reconnection (no custom reconnect logic needed).
- Log connection, disconnection, and reconnection events.

**Covers:** 04-REQ-1.1, 04-REQ-1.2

### Task 2.3: Implement `main.rs` startup for NATS

- Load configuration.
- Connect to NATS.
- Subscribe to `vehicles.{VIN}.commands`.
- Log "CLOUD_GATEWAY_CLIENT started for VIN={VIN}".

**Covers:** 04-REQ-1.1, 04-REQ-7.1

**Exit criteria:** `cargo build -p cloud-gateway-client` succeeds. Unit tests for config pass. The binary connects to NATS (with `make infra-up`) and subscribes to the command subject.

---

## Group 3: Command Validation and DATA_BROKER Write

**Goal:** Implement the inbound command pipeline: receive from NATS, validate, write to DATA_BROKER.

### Task 3.1: Implement `command.rs`

Define `Command` struct with serde deserialization. Implement validation: required fields (`command_id`, `action`, `doors`, `source`, `vin`, `timestamp`), valid `action` values (`"lock"` or `"unlock"`), valid UUID for `command_id`. Return structured validation errors.

**Covers:** 04-REQ-2.1

### Task 3.2: Implement `databroker_client.rs`

- Create a tonic gRPC client that connects to DATA_BROKER via Unix Domain Socket.
- Implement `set_signal(path, value)` to write a string signal (for `Vehicle.Command.Door.Lock`).
- Implement `subscribe_signal(paths)` to subscribe to one or more signals and return a stream of updates.
- Handle connection errors with retry and exponential backoff (1s, 2s, 4s, ..., max 30s).

**Covers:** 04-REQ-5.1

### Task 3.3: Implement `command_processor.rs`

- Read messages from the NATS subscription stream.
- Deserialize and validate each message using `command.rs`.
- On valid command: write the JSON to `Vehicle.Command.Door.Lock` on DATA_BROKER.
- On invalid command: log warning with details and discard.
- Handle DATA_BROKER write failures: log error, discard command (no retry to avoid reordering).
- Handle DATA_BROKER unreachable: log error and discard (per 04-REQ-5.1 edge case).

**Covers:** 04-REQ-2.1, 04-REQ-5.1

### Task 3.4: Wire command processor into `main.rs`

Spawn the command processor as a tokio task in the main startup sequence.

**Exit criteria:** Unit tests for command validation pass. With infra running, publishing a valid command on NATS results in the command appearing on `Vehicle.Command.Door.Lock` in DATA_BROKER. Malformed commands are logged and discarded. TS-04-P1, TS-04-E1, TS-04-E2, TS-04-E3 pass.

---

## Group 4: Telemetry Publishing and Response Relay

**Goal:** Implement the outbound pipelines: DATA_BROKER subscriptions to NATS publishing for both command responses and telemetry.

### Task 4.1: Implement `response_relay.rs`

- Subscribe to `Vehicle.Command.Door.Response` on DATA_BROKER via `databroker_client.subscribe_signal()`.
- On each response update, read the JSON string value.
- Publish the response JSON to `vehicles.{VIN}.command_responses` on NATS.
- Handle DATA_BROKER stream errors: log and attempt reconnection.
- Handle unparseable response JSON from DATA_BROKER: log warning and skip.

**Covers:** 04-REQ-3.1

### Task 4.2: Implement `telemetry.rs`

- Subscribe to the following DATA_BROKER signals:
  - `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
  - `Vehicle.CurrentLocation.Latitude`
  - `Vehicle.CurrentLocation.Longitude`
  - `Vehicle.Parking.SessionActive`
- On each signal change, construct a telemetry JSON message with `vin`, `signal`, `value`, and `timestamp`.
- Publish to `vehicles.{VIN}.telemetry` on NATS.
- Only publish on actual value changes, not on periodic schedule.

**Covers:** 04-REQ-4.1

### Task 4.3: Wire response relay and telemetry into `main.rs`

Spawn `response_relay` and `telemetry` as tokio tasks alongside `command_processor`. Ensure all three tasks run concurrently. Add shutdown signal handling (SIGTERM/SIGINT) that closes NATS and DATA_BROKER connections. If any task exits with an error, log and attempt restart.

**Covers:** 04-REQ-7.1

**Exit criteria:** Writing a response to `Vehicle.Command.Door.Response` on DATA_BROKER results in the response appearing on `vehicles.{VIN}.command_responses` in NATS. Writing telemetry signals to DATA_BROKER results in telemetry messages on `vehicles.{VIN}.telemetry` in NATS. TS-04-P2, TS-04-P3, TS-04-P4, TS-04-P5 pass.

---

## Group 5: Integration Tests

**Goal:** Run full integration tests against real NATS and DATA_BROKER infrastructure. Verify all pipelines end-to-end and all error handling.

### Task 5.1: Verify command pipeline end-to-end (TS-04-P1)

Run the integration test that publishes a command via NATS and verifies it appears on DATA_BROKER. Fix any issues found.

### Task 5.2: Verify response relay end-to-end (TS-04-P2)

Run the integration test that writes a response to DATA_BROKER and verifies it appears on NATS. Fix any issues found.

### Task 5.3: Verify telemetry pipeline end-to-end (TS-04-P3, TS-04-P4)

Run the integration tests that write telemetry signals to DATA_BROKER and verify they appear on NATS. Fix any issues found.

### Task 5.4: Verify full command round-trip (TS-04-P5)

Run the integration test that exercises the complete command -> response flow. Fix any issues found.

### Task 5.5: Verify error handling (TS-04-E1, TS-04-E2, TS-04-E3, TS-04-E6, TS-04-E7)

Run integration tests for malformed commands, missing fields, invalid action values, DATA_BROKER unavailability, and invalid tokens. Fix any issues found.

### Task 5.6: Verify VIN isolation (TS-04-E5)

Run the integration test that confirms commands for other VINs are not processed. Fix any issues found.

### Task 5.7: Verify NATS reconnection (TS-04-E4)

Run the integration test that stops and restarts NATS, then confirms the client recovers. Fix any issues found.

**Exit criteria:** All integration tests pass. `cargo clippy -p cloud-gateway-client` reports no warnings. `cargo test -p cloud-gateway-client` passes. `cargo test -p cloud-gateway-client --features integration` passes (with infra running).

---

## Checkpoint

**Goal:** Final validation that all requirements are met, all tests pass, and the component is ready for integration with other specs.

### Task C.1: Full build and test run

Run in sequence:
1. `cd rhivos && cargo build -p cloud-gateway-client`
2. `cd rhivos && cargo clippy -p cloud-gateway-client`
3. `cd rhivos && cargo test -p cloud-gateway-client`
4. `make infra-up`
5. `cd rhivos && cargo test -p cloud-gateway-client --features integration`

Confirm all steps pass with zero errors and zero warnings.

### Task C.2: Manual smoke test

1. Start infrastructure: `make infra-up`.
2. Run `VIN=SMOKE_TEST_VIN cargo run -p cloud-gateway-client`.
3. Publish a lock command to `vehicles.SMOKE_TEST_VIN.commands` using `nats pub` or a test script.
4. Verify the command appears on `Vehicle.Command.Door.Lock` in DATA_BROKER.
5. Write a response to `Vehicle.Command.Door.Response` in DATA_BROKER.
6. Verify the response appears on `vehicles.SMOKE_TEST_VIN.command_responses` in NATS.
7. Write a lock state change to DATA_BROKER.
8. Verify telemetry appears on `vehicles.SMOKE_TEST_VIN.telemetry` in NATS.

### Task C.3: Requirements coverage review

Verify every requirement in `requirements.md` has at least one passing test in `test_spec.md`.

**Exit criteria:** All build, lint, and test steps pass. Manual smoke test demonstrates end-to-end functionality. Requirements coverage is complete.

---

## Traceability Matrix

| Requirement | Task Group(s) | Test(s) |
|-------------|--------------|---------|
| 04-REQ-1.1 | G2 (2.1, 2.2, 2.3) | TS-04-1, TS-04-2, TS-04-3 |
| 04-REQ-1.2 | G2 (2.2) | TS-04-E4 |
| 04-REQ-2.1 | G3 (3.1, 3.3) | TS-04-P1, TS-04-E1, TS-04-E2, TS-04-E3, TS-04-E7 |
| 04-REQ-3.1 | G4 (4.1) | TS-04-P2, TS-04-P5 |
| 04-REQ-4.1 | G4 (4.2) | TS-04-P3, TS-04-P4 |
| 04-REQ-5.1 | G3 (3.2) | TS-04-3, TS-04-E6 |
| 04-REQ-6.1 | G2 (2.3), G3 (3.3) | TS-04-E5 |
| 04-REQ-7.1 | G2 (2.3), G4 (4.3) | TS-04-1 |
