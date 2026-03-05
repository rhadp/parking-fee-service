# Implementation Tasks: LOCKING_SERVICE (Spec 03)

> Task breakdown for implementing the LOCKING_SERVICE and mock sensor CLI tools.
> Implements the design defined in `.specs/03_locking_service/design.md`.

## Dependencies

| Spec | Dependency | What Is Needed |
|------|-----------|----------------|
| 01_project_setup | Group 2 (Rust workspace) | Rust workspace at `rhivos/Cargo.toml` with workspace member structure. Proto definitions at `proto/`. Top-level Makefile with `make build`, `make test`, `make lint` targets. |
| 02_data_broker | Group 1 (Kuksa deployment) | Eclipse Kuksa Databroker running locally (port 55556 or UDS) with all VSS signals configured (standard + custom signals from the PRD). `make infra-up` starts the broker. |

## Test Commands

| What | Command |
|------|---------|
| Unit tests (locking-service) | `cd rhivos && cargo test -p locking-service` |
| Unit tests (mock-sensors) | `cd rhivos && cargo test -p mock-sensors` |
| All unit tests | `cd rhivos && cargo test` |
| Integration tests | `cd rhivos && cargo test -p locking-service --test integration` |
| Lint (locking-service) | `cd rhivos && cargo clippy -p locking-service -- -D warnings` |
| Lint (mock-sensors) | `cd rhivos && cargo clippy -p mock-sensors -- -D warnings` |
| Format check | `cd rhivos && cargo fmt -- --check` |

---

## Task Group 1: Write Failing Spec Tests

**Goal:** Write unit tests and integration test stubs that encode the requirements and test specifications. All tests should fail initially (red phase of red-green-refactor).

### Task 1.1: Set Up Crate Skeletons

Create the crate structure within the Rust workspace:

- Add `locking-service` crate at `rhivos/locking-service/` with `Cargo.toml` and `src/main.rs`.
- Add `mock-sensors` crate at `rhivos/mock-sensors/` with `Cargo.toml` and `src/lib.rs`.
- Register both crates as workspace members in `rhivos/Cargo.toml`.
- Add dependencies: `tokio`, `tonic`, `prost`, `serde`, `serde_json`, `clap`, `tracing`, `tracing-subscriber`.

**Verify:** `cd rhivos && cargo build -p locking-service -p mock-sensors` compiles (skeleton only).

### Task 1.2: Write Unit Tests for Command Parsing

Create `rhivos/locking-service/src/command.rs` with type stubs and tests:

- Test: valid JSON payload deserializes into `LockCommand` struct with all fields.
- Test: JSON with `"action": "lock"` parses to `Action::Lock`.
- Test: JSON with `"action": "unlock"` parses to `Action::Unlock`.
- Test: JSON missing `command_id` field returns parse error.
- Test: JSON missing `action` field returns parse error.
- Test: JSON with unknown `action` value (e.g., `"open"`) returns parse error.
- Test: completely invalid JSON (not valid JSON syntax) returns parse error.

**Traces to:** 03-REQ-1.1, 03-REQ-4.2, TS-03-E1

### Task 1.3: Write Unit Tests for Safety Validation

Create `rhivos/locking-service/src/validator.rs` with trait stubs and tests:

- Test: lock command with speed = 0.0 and door closed passes validation.
- Test: lock command with speed = 5.0 returns `VehicleMoving` rejection.
- Test: lock command with speed = 0.5 (at threshold) passes validation.
- Test: lock command with speed = 0.6 (above threshold) returns `VehicleMoving` rejection.
- Test: lock command with door open returns `DoorAjar` rejection.
- Test: lock command with speed > 0.5 AND door open returns `VehicleMoving` (speed checked first).
- Test: unlock command with door open passes validation (no door-ajar check for unlock).
- Test: unlock command with speed = 5.0 returns `VehicleMoving` rejection.

Use a mock/trait-based approach for signal reads so tests do not require DATA_BROKER.

**Traces to:** 03-REQ-2.1, 03-REQ-2.2, TS-03-1, TS-03-P1, TS-03-P2, TS-03-P3

### Task 1.4: Write Unit Tests for Response Serialization

Create response serialization tests in `rhivos/locking-service/src/command.rs`:

- Test: success response serializes to JSON with `command_id`, `status: "success"`, `timestamp`.
- Test: failure response serializes to JSON with `command_id`, `status: "failed"`, `reason`, `timestamp`.
- Test: `command_id` in response matches input command's `command_id`.

**Traces to:** 03-REQ-4.1, TS-03-E2

### Task 1.5: Write Integration Test Stubs

Create `rhivos/locking-service/tests/integration.rs` with test function stubs that will require a running DATA_BROKER:

- Test stub: lock with valid preconditions (TS-03-1).
- Test stub: unlock regardless of door state (TS-03-2).
- Test stub: lock rejected when vehicle moving (TS-03-P1).
- Test stub: lock rejected when door ajar (TS-03-P2).
- Test stub: concurrent command handling (TS-03-E4).
- Test stub: mock sensors write correct values (TS-03-E3).

Mark integration tests with `#[ignore]` initially (they need DATA_BROKER running).

**Verify:** `cd rhivos && cargo test -p locking-service` runs unit tests (integration tests are ignored). All unit tests fail (no implementation yet).

---

## Task Group 2: LOCKING_SERVICE Core Implementation

**Goal:** Implement the core business logic: command parsing, safety validation, and state management.

### Task 2.1: Implement Command Types and Parsing

Implement `rhivos/locking-service/src/command.rs`:

- `LockCommand` struct with serde deserialization.
- `Action` enum (`Lock`, `Unlock`) with serde deserialization.
- `CommandResponse` struct with serde serialization.
- `parse_command(json: &str) -> Result<LockCommand, CommandError>` function.
- `CommandError` enum with variants for parse failures.

**Verify:** `cd rhivos && cargo test -p locking-service command` -- all command parsing tests pass.

### Task 2.2: Implement Safety Validator

Implement `rhivos/locking-service/src/validator.rs`:

- `SignalReader` trait with methods `read_speed()` and `read_door_open()`.
- `validate(command: &LockCommand, signals: &dyn SignalReader) -> Result<(), RejectionReason>` function.
- `RejectionReason` enum: `VehicleMoving`, `DoorAjar`.
- Speed threshold constant (0.5 m/s) in `config.rs`.

**Verify:** `cd rhivos && cargo test -p locking-service validator` -- all validation tests pass.

### Task 2.3: Implement Configuration Module

Implement `rhivos/locking-service/src/config.rs`:

- `Config` struct with fields: `databroker_uds_path`, `databroker_address`, `speed_threshold`, `max_retries`.
- Load from environment variables with sensible defaults.
- Signal path constants for all VSS signals used.

### Task 2.4: Implement Error Types

Implement `rhivos/locking-service/src/error.rs`:

- Unified error type covering: parse errors, validation rejections, gRPC errors, signal read failures.

**Verify:** `cd rhivos && cargo test -p locking-service` -- all unit tests pass.

---

## Task Group 3: LOCKING_SERVICE DATA_BROKER Integration

**Goal:** Implement the gRPC client for DATA_BROKER communication and the main command processing loop.

### Task 3.1: Add Kuksa Protobuf Definitions

- Place Kuksa Databroker `.proto` files in `proto/kuksa/` (or reference them via git submodule/vendoring).
- Configure `tonic-build` in `rhivos/locking-service/build.rs` to generate Rust code from the protos.
- Verify generated code compiles.

### Task 3.2: Implement Broker Client

Implement `rhivos/locking-service/src/broker.rs`:

- `BrokerClient` struct wrapping a tonic gRPC channel (UDS or TCP).
- `connect(config: &Config) -> Result<Self>` constructor with retry logic.
- `subscribe_commands() -> Result<Streaming<SubscribeResponse>>` method.
- `read_speed() -> Result<f32>` method (reads `Vehicle.Speed`).
- `read_door_open() -> Result<bool>` method (reads `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`).
- `write_lock_state(locked: bool) -> Result<()>` method.
- `write_response(response: &CommandResponse) -> Result<()>` method.
- Implement the `SignalReader` trait for `BrokerClient`.

### Task 3.3: Implement Main Command Loop

Implement `rhivos/locking-service/src/main.rs`:

- Initialize tracing/logging.
- Load configuration.
- Connect to DATA_BROKER with retry.
- Subscribe to `Vehicle.Command.Door.Lock`.
- For each incoming command:
  1. Parse the JSON payload.
  2. Validate safety constraints.
  3. On success: write lock state, publish success response.
  4. On failure: publish failure response (do not modify lock state).
  5. Handle invalid commands per 03-REQ-4.2.
- Commands processed sequentially (single-threaded command loop ensures serialization per 03-REQ-4.3).

**Verify:**
- `cd rhivos && cargo build -p locking-service` compiles.
- `cd rhivos && cargo clippy -p locking-service -- -D warnings` passes.

---

## Task Group 4: Mock Sensor CLIs

**Goal:** Implement the three mock sensor CLI tools.

### Task 4.1: Implement Shared Broker Client for Sensors

Implement `rhivos/mock-sensors/src/broker_client.rs`:

- Shared gRPC client for writing signals to DATA_BROKER.
- `connect()` function (same UDS/TCP connection logic as LOCKING_SERVICE).
- `write_signal(path: &str, value: DataValue) -> Result<()>` generic write method.

### Task 4.2: Implement LOCATION_SENSOR CLI

Implement `rhivos/mock-sensors/src/bin/location_sensor.rs`:

- CLI arguments: `--latitude <f64>`, `--longitude <f64>` (using clap derive).
- Connect to DATA_BROKER.
- Write `Vehicle.CurrentLocation.Latitude` and `Vehicle.CurrentLocation.Longitude`.
- Exit 0 on success, non-zero on failure with error message to stderr.

### Task 4.3: Implement SPEED_SENSOR CLI

Implement `rhivos/mock-sensors/src/bin/speed_sensor.rs`:

- CLI arguments: `--speed <f32>` (using clap derive).
- Connect to DATA_BROKER.
- Write `Vehicle.Speed`.
- Exit 0 on success, non-zero on failure.

### Task 4.4: Implement DOOR_SENSOR CLI

Implement `rhivos/mock-sensors/src/bin/door_sensor.rs`:

- CLI arguments: `--open <true|false>` (using clap derive).
- Connect to DATA_BROKER.
- Write `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.
- Exit 0 on success, non-zero on failure.

**Verify:**
- `cd rhivos && cargo build -p mock-sensors` compiles.
- `cd rhivos && cargo clippy -p mock-sensors -- -D warnings` passes.
- `cd rhivos && cargo test -p mock-sensors` passes (if any unit tests exist for argument parsing).

---

## Task Group 5: Integration Testing with DATA_BROKER

**Goal:** Run the full integration test suite against a live DATA_BROKER instance.

### Task 5.1: Create Integration Test Harness

- Write a script or test configuration that:
  1. Starts DATA_BROKER via `make infra-up` (or docker-compose).
  2. Waits for DATA_BROKER to be ready (health check or port probe).
  3. Starts LOCKING_SERVICE in the background.
  4. Runs integration tests.
  5. Stops LOCKING_SERVICE and DATA_BROKER.

### Task 5.2: Implement Integration Tests

Complete the integration test stubs from Task 1.5 in `rhivos/locking-service/tests/integration.rs`:

- **TS-03-1:** Set speed=0, door=closed. Send lock command. Assert IsLocked=true and success response.
- **TS-03-2:** Set speed=0, door=open. Send unlock command. Assert IsLocked=false and success response.
- **TS-03-P1:** Set speed=5.0. Send lock command. Assert IsLocked unchanged and failed response with "vehicle_moving".
- **TS-03-P2:** Set speed=0, door=open. Send lock command. Assert IsLocked unchanged and failed response with "door_ajar".
- **TS-03-E4:** Send three commands rapidly. Assert three responses received in order with correct command_ids.

### Task 5.3: Implement Mock Sensor Integration Tests

Test mock sensor CLIs by running them as subprocesses:

- **TS-03-E3:** Run each sensor CLI with known values, then read the corresponding signal from DATA_BROKER and verify the value.

**Verify:**
- `cd rhivos && cargo test -p locking-service --test integration` passes (with DATA_BROKER running).
- All integration tests in TS-03 pass.

---

## Task Group 6: Checkpoint

**Goal:** Final verification that all requirements are met and the implementation is complete.

### Task 6.1: Full Test Suite

Run the complete test suite:

```bash
cd rhivos && cargo test
cd rhivos && cargo clippy -p locking-service -p mock-sensors -- -D warnings
cd rhivos && cargo fmt -- --check
```

All must pass with zero warnings and zero failures.

### Task 6.2: Requirement Traceability Verification

Verify every requirement has at least one passing test:

| Requirement | Test(s) | Status |
|-------------|---------|--------|
| 03-REQ-1.1 | TS-03-1, TS-03-2 (subscription exercised) | |
| 03-REQ-2.1 | TS-03-1, TS-03-P1, TS-03-P3, unit tests | |
| 03-REQ-2.2 | TS-03-1, TS-03-2, TS-03-P2, unit tests | |
| 03-REQ-3.1 | TS-03-1, TS-03-2 | |
| 03-REQ-4.1 | TS-03-1, TS-03-2, TS-03-P1, TS-03-P2, TS-03-E2 | |
| 03-REQ-4.2 | TS-03-E1, unit tests | |
| 03-REQ-4.3 | TS-03-E4 | |
| 03-REQ-5.1 | TS-03-E3 | |
| 03-REQ-5.2 | TS-03-E3 | |
| 03-REQ-5.3 | TS-03-E3 | |

### Task 6.3: Definition of Done Checklist

- [ ] All requirements in `requirements.md` have passing tests.
- [ ] `cd rhivos && cargo test -p locking-service` passes.
- [ ] `cd rhivos && cargo test -p mock-sensors` passes.
- [ ] Integration tests pass with DATA_BROKER running.
- [ ] `cd rhivos && cargo clippy -p locking-service -p mock-sensors -- -D warnings` passes.
- [ ] `cd rhivos && cargo fmt -- --check` passes.
- [ ] Public functions and modules have doc comments.
- [ ] All safety validation edge cases have dedicated tests.

---

## Traceability: Tasks to Requirements

| Task | Requirements | Test Specs |
|------|-------------|------------|
| 1.2 | 03-REQ-1.1, 03-REQ-4.2 | TS-03-E1 |
| 1.3 | 03-REQ-2.1, 03-REQ-2.2 | TS-03-1, TS-03-P1, TS-03-P2, TS-03-P3 |
| 1.4 | 03-REQ-4.1 | TS-03-E2 |
| 2.1 | 03-REQ-1.1, 03-REQ-4.2 | TS-03-E1 |
| 2.2 | 03-REQ-2.1, 03-REQ-2.2 | TS-03-1, TS-03-P1, TS-03-P2, TS-03-P3 |
| 3.2 | 03-REQ-1.1, 03-REQ-3.1, 03-REQ-4.1 | TS-03-1, TS-03-2, TS-03-E2 |
| 3.3 | 03-REQ-1.1, 03-REQ-4.3 | TS-03-E4 |
| 4.2 | 03-REQ-5.1 | TS-03-E3 |
| 4.3 | 03-REQ-5.2 | TS-03-E3 |
| 4.4 | 03-REQ-5.3 | TS-03-E3 |
| 5.2 | 03-REQ-2.1, 03-REQ-2.2, 03-REQ-3.1, 03-REQ-4.1, 03-REQ-4.3 | TS-03-1, TS-03-2, TS-03-P1, TS-03-P2, TS-03-E4 |
| 5.3 | 03-REQ-5.1, 03-REQ-5.2, 03-REQ-5.3 | TS-03-E3 |
