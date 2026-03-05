# Tasks: PARKING_OPERATOR_ADAPTOR (Spec 08)

> Implementation tasks for the PARKING_OPERATOR_ADAPTOR.
> Derived from `.specs/08_parking_operator_adaptor/design.md` and `.specs/08_parking_operator_adaptor/test_spec.md`.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | All groups | 1 | Requires Rust workspace skeleton (`rhivos/Cargo.toml`), proto definitions (`proto/parking_adaptor.proto`), and build system (`Makefile`) |
| 02_data_broker | 3, 4 | 1 | Requires running DATA_BROKER (Kuksa Databroker) with VSS signals configured, including `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` and `Vehicle.Parking.SessionActive` |
| 07_update_service | -- | -- | UPDATE_SERVICE manages adaptor container lifecycle (installed/started as container); no build-time dependency |

## Test Commands

| Scope | Command |
|-------|---------|
| Rust unit tests | `cd rhivos && cargo test -p parking-operator-adaptor` |
| Rust lint | `cd rhivos && cargo clippy -p parking-operator-adaptor` |
| Integration tests | Requires `make infra-up` first; then run integration test binary |

## Group 1: Write Failing Spec Tests

**Goal:** Establish the test harness and write failing tests that define the expected behavior. Tests must compile but fail (red phase of red-green-refactor).

### Task 1.1: Create Rust crate skeleton for parking-operator-adaptor

- Add `parking-operator-adaptor` to the Rust workspace in `rhivos/Cargo.toml`.
- Create `rhivos/parking-operator-adaptor/Cargo.toml` with dependencies: `tonic`, `prost`, `tokio`, `reqwest`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`.
- Create `rhivos/parking-operator-adaptor/src/main.rs` with a minimal `#[tokio::main]` entry point that compiles.
- Create `rhivos/parking-operator-adaptor/build.rs` for tonic-build proto compilation of `proto/parking_adaptor.proto`.
- Verify: `cd rhivos && cargo build -p parking-operator-adaptor` succeeds.

**Files:** `rhivos/parking-operator-adaptor/Cargo.toml`, `rhivos/parking-operator-adaptor/src/main.rs`, `rhivos/parking-operator-adaptor/build.rs`

### Task 1.2: Create proto/parking_adaptor.proto

- Create `proto/parking_adaptor.proto` with the `ParkingAdaptor` service definition as specified in the design document (`StartSession`, `StopSession`, `GetStatus`, `GetRate` RPCs with their request/response message types).
- Note: `StopSessionRequest` has no fields (empty message).
- `GetRateResponse` includes `rate_type`, `rate_amount`, `currency`, and `zone_id`.
- Verify: `cd rhivos && cargo build -p parking-operator-adaptor` compiles the generated code.

**Files:** `proto/parking_adaptor.proto`

### Task 1.3: Write failing Rust unit tests for session state machine

- Create `rhivos/parking-operator-adaptor/src/session/mod.rs` and `rhivos/parking-operator-adaptor/src/session/state.rs` with a minimal `SessionState` enum and `SessionManager` struct (compiles but has no logic).
- Write `#[cfg(test)]` tests in `state.rs` covering:
  - `test_idle_to_starting_on_lock` (state machine portion of TS-08-1)
  - `test_starting_to_active_on_operator_ok`
  - `test_active_to_stopping_on_unlock`
  - `test_stopping_to_idle_on_operator_ok`
  - `test_double_lock_ignored` (TS-08-P1)
  - `test_double_unlock_ignored` (TS-08-P2)
  - `test_starting_to_idle_on_operator_error`
- All tests should fail.

**Files:** `rhivos/parking-operator-adaptor/src/session/mod.rs`, `rhivos/parking-operator-adaptor/src/session/state.rs`

### Task 1.4: Write failing Rust unit tests for operator REST client

- Create `rhivos/parking-operator-adaptor/src/operator/mod.rs`, `client.rs`, and `models.rs` with minimal struct stubs.
- Write `#[cfg(test)]` tests in `client.rs` covering:
  - `test_start_session_request_format`
  - `test_stop_session_request_format`
  - `test_start_session_response_parse`
  - `test_stop_session_response_parse`
  - `test_status_query_response_parse`
- All tests should fail.

**Files:** `rhivos/parking-operator-adaptor/src/operator/mod.rs`, `rhivos/parking-operator-adaptor/src/operator/client.rs`, `rhivos/parking-operator-adaptor/src/operator/models.rs`

### Task 1.5: Write failing Rust unit tests for gRPC service

- Create `rhivos/parking-operator-adaptor/src/grpc/mod.rs` and `rhivos/parking-operator-adaptor/src/grpc/service.rs` with minimal stubs.
- Write `#[cfg(test)]` tests covering:
  - `test_get_status_idle` (TS-08-5)
  - `test_get_rate` (TS-08-6)
  - `test_start_session_already_active` (TS-08-E3)
  - `test_stop_session_no_active` (TS-08-E4)
- All tests should fail.

**Files:** `rhivos/parking-operator-adaptor/src/grpc/mod.rs`, `rhivos/parking-operator-adaptor/src/grpc/service.rs`

**Verification:** `cd rhivos && cargo test -p parking-operator-adaptor` compiles but all tests fail.

---

## Group 2: Implement gRPC Server (Proto + Service Stub)

**Goal:** Implement the gRPC service definition and server startup so that the adaptor listens on port 50052 and responds to RPCs.

### Task 2.1: Implement configuration module

- Create `rhivos/parking-operator-adaptor/src/config.rs` with a `Config` struct reading from environment variables with defaults:
  - `PARKING_OPERATOR_URL` (default: `http://localhost:8080`)
  - `DATA_BROKER_ADDR` (default: `http://localhost:55556`)
  - `GRPC_PORT` (default: `50052`)
  - `VEHICLE_ID` (default: `DEMO-VIN-001`)
  - `ZONE_ID` (default: `zone-demo-1`)

**Files:** `rhivos/parking-operator-adaptor/src/config.rs`

### Task 2.2: Implement session state machine

- Implement `SessionState` enum in `rhivos/parking-operator-adaptor/src/session/state.rs` with variants: `Idle`, `Starting`, `Active`, `Stopping`.
- Implement `SessionManager` with:
  - `state() -> SessionState`
  - `try_start() -> Result<(), SessionError>` (idle -> starting; error if not idle)
  - `confirm_start(session_id) -> ()` (starting -> active)
  - `fail_start() -> ()` (starting -> idle)
  - `try_stop() -> Result<(), SessionError>` (active -> stopping; error if not active)
  - `confirm_stop() -> ()` (stopping -> idle)
  - `fail_stop() -> ()` (stopping -> idle)
  - `session_id() -> Option<String>`
- Protect state with `tokio::sync::Mutex`.

**Files:** `rhivos/parking-operator-adaptor/src/session/state.rs`

**Verification:** Session state machine tests from Task 1.3 pass.

### Task 2.3: Implement ParkingAdaptor gRPC service

- Implement `rhivos/parking-operator-adaptor/src/grpc/service.rs` with a struct that implements the `ParkingAdaptor` tonic trait.
- The service holds shared references to `SessionManager`, `OperatorClient`, and `BrokerPublisher` (use trait-based or optional dependencies for testability).
- Implement RPCs:
  - `StartSession`: acquire session lock, call `try_start`, call operator `start_session`, call `confirm_start` or `fail_start`, publish `SessionActive`.
  - `StopSession`: acquire session lock, call `try_stop`, call operator `stop_session`, call `confirm_stop` or `fail_stop`, publish `SessionActive`.
  - `GetStatus`: read current state and session_id from `SessionManager`.
  - `GetRate`: return rate information (rate_type, rate_amount, currency, zone_id). Return `FAILED_PRECONDITION` if no zone configured.
- Map errors to appropriate gRPC status codes (`ALREADY_EXISTS`, `NOT_FOUND`, `UNAVAILABLE`, `FAILED_PRECONDITION`, `INTERNAL`).

**Files:** `rhivos/parking-operator-adaptor/src/grpc/service.rs`

### Task 2.4: Wire gRPC server in main.rs

- Update `rhivos/parking-operator-adaptor/src/main.rs` to:
  - Read configuration.
  - Create `SessionManager`.
  - Start the tonic gRPC server on the configured port (default 50052).
  - Log startup message.
- Verify: `cd rhivos && cargo build -p parking-operator-adaptor` compiles.
- Verify: gRPC tests from Task 1.5 pass.

**Files:** `rhivos/parking-operator-adaptor/src/main.rs`

---

## Group 3: Implement DATA_BROKER Subscription and State Writing

**Goal:** Connect the adaptor to DATA_BROKER for subscribing to lock/unlock events and publishing session state.

### Task 3.1: Implement DATA_BROKER subscriber

- Implement `rhivos/parking-operator-adaptor/src/broker/subscriber.rs` with:
  - `BrokerSubscriber::new(addr: String)` -- connects to Kuksa Databroker via gRPC (network TCP).
  - `subscribe_lock_events() -> impl Stream<Item = bool>` -- subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` and yields boolean values.
- Use the Kuksa Databroker `Subscribe` gRPC API.
- Handle connection errors by logging and retrying with backoff.

**Files:** `rhivos/parking-operator-adaptor/src/broker/mod.rs`, `rhivos/parking-operator-adaptor/src/broker/subscriber.rs`

### Task 3.2: Implement DATA_BROKER publisher

- Implement `rhivos/parking-operator-adaptor/src/broker/publisher.rs` with:
  - `BrokerPublisher::new(addr: String)` -- connects to Kuksa Databroker via gRPC.
  - `set_session_active(active: bool) -> Result<(), BrokerError>` -- writes `Vehicle.Parking.SessionActive` to DATA_BROKER using `SetRequest`.

**Files:** `rhivos/parking-operator-adaptor/src/broker/publisher.rs`

### Task 3.3: Write unit tests for broker modules

- Add `#[cfg(test)]` tests for subscriber and publisher modules.
- Tests:
  - `test_subscriber_connects` -- verifies connection attempt.
  - `test_publisher_sets_session_active_true`
  - `test_publisher_sets_session_active_false`

**Files:** `rhivos/parking-operator-adaptor/src/broker/subscriber.rs`, `rhivos/parking-operator-adaptor/src/broker/publisher.rs`

---

## Group 4: Implement Autonomous Session Management (Lock/Unlock Events)

**Goal:** Wire the DATA_BROKER subscription to the session state machine and operator client, enabling fully autonomous session management.

### Task 4.1: Implement operator REST client models

- Implement `rhivos/parking-operator-adaptor/src/operator/models.rs` with serde structs:
  - `StartRequest { vehicle_id, zone_id, timestamp }`
  - `StartResponse { session_id, status }`
  - `StopRequest { session_id, timestamp }`
  - `StopResponse { session_id, duration, fee, status }`
  - `StatusResponse { session_id, status, rate_type, rate_amount, currency }`
- Add `Serialize`/`Deserialize` derives.

**Files:** `rhivos/parking-operator-adaptor/src/operator/models.rs`

### Task 4.2: Implement operator REST client

- Implement `rhivos/parking-operator-adaptor/src/operator/client.rs` with:
  - `OperatorClient::new(base_url: String)` -- creates reqwest client with 5s timeout.
  - `start_session(vehicle_id, zone_id) -> Result<StartResponse, OperatorError>`
  - `stop_session(session_id) -> Result<StopResponse, OperatorError>`
  - `get_status(session_id) -> Result<StatusResponse, OperatorError>`
- `OperatorError` enum: `Unreachable`, `Timeout`, `HttpError(status, body)`, `ParseError`.

**Files:** `rhivos/parking-operator-adaptor/src/operator/client.rs`

**Verification:** Operator client tests from Task 1.4 pass.

### Task 4.3: Implement autonomous event loop

- In `main.rs` (or a dedicated module), spawn a tokio task that:
  - Subscribes to lock events via `BrokerSubscriber`.
  - For each `IsLocked = true` event: if state is idle, call `try_start` -> operator `start_session` -> `confirm_start` / `fail_start` -> publish `SessionActive`.
  - For each `IsLocked = false` event: if state is active, call `try_stop` -> operator `stop_session` -> `confirm_stop` / `fail_stop` -> publish `SessionActive`.
- Ensure the autonomous event loop and gRPC handlers share the same `SessionManager` (via `Arc<Mutex<...>>`).

**Files:** `rhivos/parking-operator-adaptor/src/main.rs`

### Task 4.4: Integration test: full lock/unlock cycle

- Write an integration test that:
  - Starts DATA_BROKER, mock operator, and adaptor.
  - Writes `IsLocked = true` to DATA_BROKER.
  - Asserts `SessionActive = true` on DATA_BROKER and `GetStatus` returns active.
  - Writes `IsLocked = false` to DATA_BROKER.
  - Asserts `SessionActive = false` and `GetStatus` returns idle.
- Covers: TS-08-1, TS-08-2, TS-08-3, TS-08-4.

---

## Group 5: Implement PARKING_OPERATOR REST Client

**Goal:** Ensure the REST client is fully tested against the mock PARKING_OPERATOR, including the status query endpoint.

### Task 5.1: Integration test: REST start/stop cycle

- Write an integration test that directly calls the mock PARKING_OPERATOR REST endpoints (without going through the adaptor) to validate the REST contract:
  - `POST /parking/start` returns session_id and status.
  - `POST /parking/stop` returns session_id, duration, fee, and status.
  - `GET /parking/status/{session_id}` returns current session status with rate information.
- Covers: TS-08-9, TS-08-10.

### Task 5.2: Integration test: operator unreachable

- Write an integration test covering:
  - Stop the mock operator.
  - Write `IsLocked = true` to DATA_BROKER.
  - Verify session remains idle and `SessionActive` is not set (TS-08-E1).
  - Start a session, then stop the mock operator, then unlock.
  - Verify adaptor transitions to idle but `SessionActive` remains stale (TS-08-E2).

---

## Group 6: Implement Manual Override Logic

**Goal:** Validate that manual gRPC overrides work correctly alongside autonomous operations.

### Task 6.1: Integration test: manual start and stop

- Write an integration test covering:
  - Call `StartSession` via gRPC. Verify session active. Verify `SessionActive = true` on DATA_BROKER.
  - Call `StopSession` via gRPC. Verify session idle. Verify `SessionActive = false`.
- Covers: TS-08-7, TS-08-8.

### Task 6.2: Integration test: double lock / double unlock

- Write an integration test covering:
  - Double lock: two consecutive `IsLocked = true` events, verify only one operator start call (TS-08-P1).
  - Double unlock: two consecutive `IsLocked = false` events, verify only one operator stop call (TS-08-P2).

### Task 6.3: Integration test: manual start then autonomous unlock

- Write an integration test covering:
  - Call `StartSession` via gRPC.
  - Write `IsLocked = true` to DATA_BROKER (should be ignored, session already active -- TS-08-P5).
  - Write `IsLocked = false` to DATA_BROKER (should trigger stop -- TS-08-P3).
  - Verify session is idle.

### Task 6.4: Integration test: state-signal consistency

- Write an integration test covering:
  - Full cycle of lock/start, verify `SessionActive`, unlock/stop, verify `SessionActive`.
  - At every step, `Vehicle.Parking.SessionActive` on DATA_BROKER matches the adaptor's internal state.
- Covers: TS-08-P4.

### Task 6.5: Integration test: error gRPC calls

- Write integration tests covering:
  - `StartSession` when session already active returns `ALREADY_EXISTS` (TS-08-E3).
  - `StopSession` when no session active returns `NOT_FOUND` (TS-08-E4).
  - `GetRate` with no zone returns `FAILED_PRECONDITION` (TS-08-E6).

---

## Group 7: Checkpoint

**Goal:** Verify all tests pass and the system is fully functional.

### Task 7.1: Run all unit tests

- Run: `cd rhivos && cargo test -p parking-operator-adaptor`
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

---

## Traceability Matrix

| Requirement | Test Spec | Task Group |
|-------------|-----------|------------|
| 08-REQ-1.1 (gRPC interface) | TS-08-5, TS-08-6 | Group 2 (gRPC service) |
| 08-REQ-1.2 (Manual StartSession) | TS-08-7 | Group 2, Group 6 |
| 08-REQ-1.3 (Manual StopSession) | TS-08-8 | Group 2, Group 6 |
| 08-REQ-1.E1 (StartSession already active) | TS-08-E3 | Group 2, Group 6 |
| 08-REQ-1.E2 (StopSession no session) | TS-08-E4 | Group 2, Group 6 |
| 08-REQ-2.1 (DATA_BROKER subscription) | TS-08-1, TS-08-2 | Group 3 (subscriber) |
| 08-REQ-2.E1 (DATA_BROKER disconnect) | TS-08-E5 | Group 3 |
| 08-REQ-3.1 (Autonomous start on lock) | TS-08-1 | Group 4 (event loop) |
| 08-REQ-3.E1 (Double lock ignored) | TS-08-P1 | Group 4, Group 6 |
| 08-REQ-4.1 (Autonomous stop on unlock) | TS-08-2 | Group 4 (event loop) |
| 08-REQ-4.E1 (Double unlock ignored) | TS-08-P2 | Group 4, Group 6 |
| 08-REQ-5.1 (Override consistency) | TS-08-7, TS-08-8, TS-08-P3 | Group 6 |
| 08-REQ-5.2 (Lock ignored after manual start) | TS-08-P5 | Group 6 |
| 08-REQ-6.1 (SessionActive on start) | TS-08-3 | Group 3 (publisher), Group 4 |
| 08-REQ-6.2 (SessionActive on stop) | TS-08-4 | Group 3 (publisher), Group 4 |
| 08-REQ-6.E1 (DATA_BROKER write failure) | TS-08-E5 | Group 3 |
| 08-REQ-7.1 (REST start) | TS-08-9 | Group 4, Group 5 |
| 08-REQ-7.2 (REST stop) | TS-08-9 | Group 4, Group 5 |
| 08-REQ-7.3 (REST status query) | TS-08-10 | Group 5 |
| 08-REQ-7.E1 (Operator unreachable) | TS-08-E1, TS-08-E2 | Group 5 |
| 08-REQ-7.E2 (Operator non-200) | TS-08-E1 | Group 5 |
| 08-REQ-8.1 (Configuration) | -- | Group 2 (config module) |
| 08-REQ-9.1 (Rate information) | TS-08-6 | Group 2 (gRPC GetRate) |
| 08-REQ-9.E1 (No zone configured) | TS-08-E6 | Group 6 |
