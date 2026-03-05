# Tasks: PARKING_OPERATOR_ADAPTOR (Spec 08)

> Implementation tasks for the PARKING_OPERATOR_ADAPTOR and mock PARKING_OPERATOR.
> Derived from `.specs/08_parking_operator_adaptor/design.md` and `.specs/08_parking_operator_adaptor/test_spec.md`.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | All groups | 1 | Requires Rust workspace skeleton (`rhivos/Cargo.toml`), proto definitions (`proto/parking_adaptor.proto`), and build system (`Makefile`) |
| 02_data_broker | 4, 6 | 1 | Requires running DATA_BROKER (Kuksa Databroker) with VSS signals configured, including `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` and `Vehicle.Parking.SessionActive` |

## Test Commands

| Scope | Command |
|-------|---------|
| Rust unit tests | `cd rhivos && cargo test -p parking-operator-adaptor` |
| Rust lint | `cd rhivos && cargo clippy -p parking-operator-adaptor` |
| Mock operator unit tests | `cd mock/parking-operator && go test ./... -v` |
| Integration tests | Requires `make infra-up` first; then run integration test binary |

## Group 1: Write Failing Spec Tests

**Goal:** Establish the test harness and write failing tests that define the expected behavior. These tests will pass as subsequent groups implement the functionality.

### Task 1.1: Create Rust crate skeleton for parking-operator-adaptor

- Add `parking-operator-adaptor` to the Rust workspace in `rhivos/Cargo.toml`.
- Create `rhivos/parking-operator-adaptor/Cargo.toml` with dependencies: `tonic`, `prost`, `tokio`, `reqwest`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`.
- Create `rhivos/parking-operator-adaptor/src/main.rs` with a minimal `#[tokio::main]` entry point that compiles.
- Create `rhivos/parking-operator-adaptor/build.rs` for tonic-build proto compilation of `proto/parking_adaptor.proto`.
- Verify: `cd rhivos && cargo build -p parking-operator-adaptor` succeeds.

### Task 1.2: Create proto/parking_adaptor.proto

- Create `proto/parking_adaptor.proto` with the `ParkingAdaptor` service definition as specified in the design document (`StartSession`, `StopSession`, `GetStatus`, `GetRate` RPCs with their request/response message types).
- Verify: `cd rhivos && cargo build -p parking-operator-adaptor` compiles the generated code.

### Task 1.3: Create Go module skeleton for mock parking-operator

- Create `mock/parking-operator/go.mod` with module path `github.com/rhadp/parking-fee-service/mock/parking-operator`.
- Create `mock/parking-operator/main.go` with a minimal HTTP server stub (compiles, listens on port 8080, returns 501 for all routes).
- Verify: `cd mock/parking-operator && go build ./...` succeeds.

### Task 1.4: Write failing unit tests for mock operator

- Create `mock/parking-operator/main_test.go` with tests covering:
  - `TestStartSession_ValidRequest` (TS-08-9 start portion)
  - `TestStopSession_ValidRequest` (TS-08-9 stop portion)
  - `TestStopSession_UnknownSession` (TS-08-E5)
  - `TestStartSession_MalformedBody` (TS-08-E6)
- All tests should fail (handlers not yet implemented).
- Verify: `cd mock/parking-operator && go test ./... -v` runs and tests fail with meaningful messages.

### Task 1.5: Write failing Rust unit tests for session state machine

- Create `rhivos/parking-operator-adaptor/src/session/mod.rs` and `rhivos/parking-operator-adaptor/src/session/state.rs` with a minimal `SessionState` enum and `SessionManager` struct (compiles but has no logic).
- Write `#[cfg(test)]` tests in `state.rs` covering:
  - `test_idle_to_starting_on_lock` (TS-08-P1 state machine portion)
  - `test_starting_to_active_on_operator_ok`
  - `test_active_to_stopping_on_unlock`
  - `test_stopping_to_idle_on_operator_ok`
  - `test_double_lock_ignored` (TS-08-P1)
  - `test_double_unlock_ignored` (TS-08-P2)
  - `test_starting_to_idle_on_operator_error`
- All tests should fail.
- Verify: `cd rhivos && cargo test -p parking-operator-adaptor` runs and tests fail.

### Task 1.6: Write failing Rust unit tests for operator REST client

- Create `rhivos/parking-operator-adaptor/src/operator/mod.rs`, `client.rs`, and `models.rs` with minimal struct stubs.
- Write `#[cfg(test)]` tests in `client.rs` covering:
  - `test_start_session_request_format`
  - `test_stop_session_request_format`
  - `test_start_session_response_parse`
  - `test_stop_session_response_parse`
- All tests should fail.
- Verify: `cd rhivos && cargo test -p parking-operator-adaptor` runs.

---

## Group 2: Mock PARKING_OPERATOR (Go REST Service)

**Goal:** Implement the mock PARKING_OPERATOR so that all Go tests pass and the adaptor has a working operator to call.

### Task 2.1: Implement request/response models

- Create `mock/parking-operator/models.go` with Go struct types for:
  - `StartRequest` (`VehicleID`, `ZoneID`, `Timestamp`)
  - `StartResponse` (`SessionID`, `Status`)
  - `StopRequest` (`SessionID`, `Timestamp`)
  - `StopResponse` (`SessionID`, `Duration`, `Fee`, `Status`)
  - `ErrorResponse` (`Error`)
- Add JSON struct tags matching the REST API contract.

### Task 2.2: Implement in-memory session store

- Create `mock/parking-operator/session.go` with:
  - `Session` struct (`SessionID`, `VehicleID`, `ZoneID`, `StartTime`, `Status`).
  - `SessionStore` backed by `sync.Map`.
  - Methods: `Create(vehicleID, zoneID) Session`, `Get(sessionID) (Session, bool)`, `Complete(sessionID) (Session, bool)`.
- Session IDs are generated UUIDs.

### Task 2.3: Implement HTTP route handlers

- Create `mock/parking-operator/handler.go` with:
  - `handleStart(w, r)`: parse body, validate required fields, create session, return 200 with `StartResponse`.
  - `handleStop(w, r)`: parse body, validate required fields, lookup session, calculate duration and fee, return 200 with `StopResponse` or 404.
  - Malformed request body returns 400.
- Fee calculation: `duration_minutes * rate_per_minute` (default 0.05 EUR/min).

### Task 2.4: Wire up main.go

- Update `mock/parking-operator/main.go` to register routes and start the HTTP server.
- Support `-port` flag and `PORT` environment variable for configurable port (default 8080).
- Verify: `cd mock/parking-operator && go test ./... -v` -- all tests from Task 1.4 pass.

---

## Group 3: PARKING_OPERATOR_ADAPTOR Core (Session State Machine, Operator REST Client)

**Goal:** Implement the core session state machine and the REST client for communicating with the PARKING_OPERATOR.

### Task 3.1: Implement session state machine

- Implement `SessionState` enum in `rhivos/parking-operator-adaptor/src/session/state.rs` with variants: `Idle`, `Starting`, `Active`, `Stopping`.
- Implement `SessionManager` with:
  - `state() -> SessionState`
  - `try_start() -> Result<(), SessionError>` (idle -> starting; returns error if not idle)
  - `confirm_start(session_id) -> ()` (starting -> active)
  - `fail_start() -> ()` (starting -> idle)
  - `try_stop() -> Result<String, SessionError>` (active -> stopping; returns session_id; error if not active)
  - `confirm_stop() -> StopResult` (stopping -> idle; returns duration/fee)
  - `fail_stop() -> ()` (stopping -> idle)
  - `session_id() -> Option<String>`
- Protect state with `tokio::sync::Mutex`.
- Verify: `cd rhivos && cargo test -p parking-operator-adaptor` -- state machine tests from Task 1.5 pass.

### Task 3.2: Implement operator REST client models

- Implement `rhivos/parking-operator-adaptor/src/operator/models.rs` with serde structs:
  - `StartRequest { vehicle_id, zone_id, timestamp }`
  - `StartResponse { session_id, status }`
  - `StopRequest { session_id, timestamp }`
  - `StopResponse { session_id, duration, fee, status }`
- Add `Serialize`/`Deserialize` derives.

### Task 3.3: Implement operator REST client

- Implement `rhivos/parking-operator-adaptor/src/operator/client.rs` with:
  - `OperatorClient::new(base_url: String)` -- creates reqwest client with 5s timeout.
  - `start_session(vehicle_id, zone_id) -> Result<StartResponse, OperatorError>`
  - `stop_session(session_id) -> Result<StopResponse, OperatorError>`
- `OperatorError` enum: `Unreachable`, `Timeout`, `HttpError(status, body)`, `ParseError`.
- Verify: `cd rhivos && cargo test -p parking-operator-adaptor` -- client tests from Task 1.6 pass.

### Task 3.4: Implement configuration module

- Implement `rhivos/parking-operator-adaptor/src/config.rs` with:
  - `Config` struct reading from environment variables with defaults as specified in the design document.
  - Fields: `parking_operator_url`, `data_broker_addr`, `grpc_port`, `vehicle_id`, `zone_id`, `rate_per_minute`.

---

## Group 4: DATA_BROKER Integration (Subscribe Lock Events, Publish SessionActive)

**Goal:** Connect the adaptor to DATA_BROKER for subscribing to lock/unlock events and publishing session state.

### Task 4.1: Implement DATA_BROKER subscriber

- Implement `rhivos/parking-operator-adaptor/src/broker/subscriber.rs` with:
  - `BrokerSubscriber::new(addr: String)` -- connects to Kuksa Databroker via gRPC (network TCP).
  - `subscribe_lock_events() -> impl Stream<Item = bool>` -- subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` and yields boolean values.
- Use the Kuksa Databroker `val.proto` / `broker.proto` gRPC API for `Subscribe`.
- Handle connection errors by logging and retrying with backoff.

### Task 4.2: Implement DATA_BROKER publisher

- Implement `rhivos/parking-operator-adaptor/src/broker/publisher.rs` with:
  - `BrokerPublisher::new(addr: String)` -- connects to Kuksa Databroker via gRPC.
  - `set_session_active(active: bool) -> Result<(), BrokerError>` -- writes `Vehicle.Parking.SessionActive` to DATA_BROKER.
- Use the Kuksa Databroker `SetRequest` gRPC call.

### Task 4.3: Write unit tests for broker modules

- Add `#[cfg(test)]` tests for subscriber and publisher modules.
- Use mock gRPC server or test against a real Kuksa Databroker instance (integration test).
- Tests:
  - `test_subscriber_connects` -- verifies connection attempt.
  - `test_publisher_sets_session_active_true`
  - `test_publisher_sets_session_active_false`

---

## Group 5: gRPC Service (StartSession, StopSession, GetStatus, GetRate)

**Goal:** Implement the gRPC service that PARKING_APP uses to manually override sessions and query status.

### Task 5.1: Implement ParkingAdaptor gRPC service

- Implement `rhivos/parking-operator-adaptor/src/grpc/service.rs` with a struct that implements the `ParkingAdaptor` tonic trait.
- The service holds shared references to `SessionManager`, `OperatorClient`, and `BrokerPublisher`.
- Implement RPCs:
  - `StartSession`: acquire session lock, call `try_start`, call operator `start_session`, call `confirm_start` or `fail_start`, publish `SessionActive`.
  - `StopSession`: acquire session lock, call `try_stop`, call operator `stop_session`, call `confirm_stop` or `fail_stop`, publish `SessionActive`.
  - `GetStatus`: read current state and session_id from `SessionManager`.
  - `GetRate`: return configured rate, currency, and zone_id.
- Map errors to appropriate gRPC status codes (ALREADY_EXISTS, NOT_FOUND, UNAVAILABLE, FAILED_PRECONDITION, INTERNAL).

### Task 5.2: Write unit tests for gRPC service

- Add `#[cfg(test)]` tests for the gRPC service:
  - `test_start_session_success` (TS-08-7)
  - `test_stop_session_success` (TS-08-8)
  - `test_get_status_idle` (TS-08-5)
  - `test_get_status_active` (TS-08-5)
  - `test_get_rate` (TS-08-6)
  - `test_start_session_already_active` (TS-08-E3)
  - `test_stop_session_no_active` (TS-08-E4)

### Task 5.3: Wire gRPC server in main.rs

- Update `rhivos/parking-operator-adaptor/src/main.rs` to:
  - Read configuration.
  - Create `SessionManager`, `OperatorClient`, `BrokerPublisher`.
  - Start the tonic gRPC server on the configured port (default 50052).
- Verify: `cd rhivos && cargo build -p parking-operator-adaptor` compiles.
- Verify: `cd rhivos && cargo clippy -p parking-operator-adaptor` passes with no warnings.

---

## Group 6: Autonomous Session Management (Event-Driven Start/Stop)

**Goal:** Wire the DATA_BROKER subscription to the session state machine and operator client, enabling fully autonomous session management.

### Task 6.1: Implement autonomous event loop

- In `main.rs` (or a dedicated module), spawn a tokio task that:
  - Subscribes to lock events via `BrokerSubscriber`.
  - For each `IsLocked = true` event: if state is idle, call `try_start` -> operator `start_session` -> `confirm_start` / `fail_start` -> publish `SessionActive`.
  - For each `IsLocked = false` event: if state is active, call `try_stop` -> operator `stop_session` -> `confirm_stop` / `fail_stop` -> publish `SessionActive`.
- Ensure the autonomous event loop and gRPC handlers share the same `SessionManager` (via `Arc<Mutex<...>>`).

### Task 6.2: Integration test: full lock/unlock cycle

- Write an integration test (can be in `tests/` directory or as a separate test binary) that:
  - Starts DATA_BROKER, mock operator, and adaptor.
  - Writes `IsLocked = true` to DATA_BROKER.
  - Asserts `SessionActive = true` on DATA_BROKER and `GetStatus` returns active.
  - Writes `IsLocked = false` to DATA_BROKER.
  - Asserts `SessionActive = false` and `GetStatus` returns idle.
- Covers: TS-08-1, TS-08-2, TS-08-3, TS-08-4.

### Task 6.3: Integration test: double lock / double unlock

- Write an integration test covering:
  - Double lock: two consecutive `IsLocked = true` events, verify only one operator start call (TS-08-P1).
  - Double unlock: two consecutive `IsLocked = false` events, verify only one operator stop call (TS-08-P2).

### Task 6.4: Integration test: operator unreachable

- Write an integration test covering:
  - Stop the mock operator.
  - Write `IsLocked = true` to DATA_BROKER.
  - Verify session remains idle and `SessionActive` is not set (TS-08-E1).

### Task 6.5: Integration test: manual override followed by autonomous event

- Write an integration test covering:
  - Call `StartSession` via gRPC.
  - Write `IsLocked = true` to DATA_BROKER (should be ignored, session already active).
  - Write `IsLocked = false` to DATA_BROKER (should trigger stop).
  - Verify session is idle (TS-08-P3).

---

## Group 7: Checkpoint

**Goal:** Verify all tests pass and the system is fully functional.

### Task 7.1: Run all unit tests

- Run: `cd rhivos && cargo test -p parking-operator-adaptor`
- Run: `cd mock/parking-operator && go test ./... -v`
- All tests must pass.

### Task 7.2: Run lint

- Run: `cd rhivos && cargo clippy -p parking-operator-adaptor`
- No warnings or errors.

### Task 7.3: Run integration tests

- Start infrastructure: `make infra-up`
- Run all integration tests.
- Verify all tests pass.
- Stop infrastructure: `make infra-down`

### Task 7.4: Verify Definition of Done

- Verify all 10 items from the Definition of Done in `design.md` are satisfied.
- Document any deviations or known limitations.

## Traceability Matrix

| Requirement | Test Spec | Task Group |
|-------------|-----------|------------|
| 08-REQ-1.1 (Autonomous session start) | TS-08-1, TS-08-P1 | Group 4 (subscriber), Group 6 (event loop) |
| 08-REQ-1.2 (Autonomous session stop) | TS-08-2, TS-08-P2 | Group 4 (subscriber), Group 6 (event loop) |
| 08-REQ-2.1 (SessionActive publication) | TS-08-3, TS-08-4, TS-08-P4 | Group 4 (publisher), Group 6 (event loop) |
| 08-REQ-3.1 (gRPC override interface) | TS-08-5, TS-08-6, TS-08-7, TS-08-8, TS-08-E3, TS-08-E4 | Group 5 (gRPC service) |
| 08-REQ-4.1 (Operator REST client) | TS-08-9 | Group 3 (client) |
| 08-REQ-5.1 (Mock PARKING_OPERATOR) | TS-08-9, TS-08-E5, TS-08-E6 | Group 2 (mock operator) |
| 08-REQ-6.1 (Operator unreachable resilience) | TS-08-E1, TS-08-E2 | Group 3 (client), Group 6 (event loop) |
