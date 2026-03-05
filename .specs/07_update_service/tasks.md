# Tasks: UPDATE_SERVICE (Spec 07)

> Implementation tasks for the UPDATE_SERVICE, organized into sequential task groups with dependency tracking.

## References

- Requirements: `.specs/07_update_service/requirements.md`
- Design: `.specs/07_update_service/design.md`
- Test Specification: `.specs/07_update_service/test_spec.md`

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Rust workspace skeleton, proto definitions, and build system |

## Test Commands

- **Unit tests:** `cd rhivos && cargo test -p update-service`
- **Lint:** `cd rhivos && cargo clippy -p update-service -- -D warnings`
- **Format check:** `cd rhivos && cargo fmt -p update-service -- --check`

---

## Task Group 1: Write Failing Spec Tests

**Goal:** Establish the test harness and write all spec tests as failing tests that define the expected behavior. Tests will be made to pass by subsequent task groups.

**Depends on:** 01_project_setup (group 2 -- Rust workspace and proto definitions)

### Task 1.1: Create update-service crate skeleton

Create the `rhivos/update-service/` crate within the Rust workspace:

- `Cargo.toml` with dependencies: `tonic`, `prost`, `tokio` (full features), `sha2`, `uuid` (v4), `tracing`, `tracing-subscriber`.
- Dev dependencies: `tonic` (for test client), `tokio` (test features).
- `build.rs` with `tonic-build` to compile `proto/update_service.proto`.
- `src/main.rs` with a minimal stub (empty main).
- `src/lib.rs` re-exporting module stubs.
- Add `update-service` to the workspace `Cargo.toml` members list.

**Verify:** `cd rhivos && cargo build -p update-service` compiles without errors.

### Task 1.2: Create proto definition

Create `proto/update_service.proto` with the full service definition as specified in the design document:

- `AdapterState` enum (UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR, OFFLOADING).
- All request/response message types.
- `UpdateService` service with all five RPCs.

**Verify:** `cd rhivos && cargo build -p update-service` compiles the proto and generates Rust code.

### Task 1.3: Write failing unit tests for state machine

Create `rhivos/update-service/src/state.rs` with the `AdapterState` enum stub and write tests:

- `test_valid_transitions`: All 10 valid transitions return `true` from `can_transition_to`. (TS-07-P3)
- `test_invalid_transitions`: Invalid transitions (e.g., `DOWNLOADING -> RUNNING`, `UNKNOWN -> RUNNING`) return `false`. (TS-07-P1)

**Verify:** `cd rhivos && cargo test -p update-service -- state` -- tests compile but fail.

### Task 1.4: Write failing unit tests for adapter ID determinism

Create `rhivos/update-service/src/store.rs` with a stub `adapter_id_from_image_ref` function and write tests:

- `test_deterministic_adapter_id`: Same `image_ref` produces same `adapter_id`. (TS-07-P2)
- `test_different_image_ref_different_id`: Different `image_ref` produces different `adapter_id`.

**Verify:** `cd rhivos && cargo test -p update-service -- store` -- tests compile but fail.

### Task 1.5: Write failing integration tests for gRPC RPCs

Create `rhivos/update-service/tests/integration_tests.rs` with integration tests that start the server in-process:

- `test_install_adapter_returns_initial_state` (TS-07-1)
- `test_state_transitions_lifecycle` (TS-07-2)
- `test_watch_adapter_states_streams_events` (TS-07-3)
- `test_list_adapters_returns_all` (TS-07-4)
- `test_remove_adapter_stops_and_removes` (TS-07-5)
- `test_get_adapter_status` (TS-07-6)
- `test_checksum_mismatch_rejects_install` (TS-07-E1)
- `test_duplicate_install_returns_already_exists` (TS-07-E2)
- `test_get_status_not_found` (TS-07-E3)
- `test_remove_not_found` (TS-07-E4)
- `test_install_empty_image_ref` (TS-07-E5)

**Verify:** `cd rhivos && cargo test -p update-service` -- tests compile but fail.

---

## Task Group 2: Adapter State Machine and In-Memory Store

**Goal:** Implement the core state machine and adapter store. Unit tests from group 1 should pass after this group.

**Depends on:** Task Group 1

### Task 2.1: Implement AdapterState enum and transitions

In `rhivos/update-service/src/state.rs`:

- Define the `AdapterState` enum matching the proto `AdapterState`.
- Implement `can_transition_to(&self, target: &AdapterState) -> bool` with all 10 valid transitions.
- Implement `Display` for logging.

**Verify:** `cd rhivos && cargo test -p update-service -- state` -- all state machine unit tests pass.

### Task 2.2: Implement AdapterStore

In `rhivos/update-service/src/store.rs`:

- Define `AdapterRecord` struct with fields: `adapter_id`, `image_ref`, `checksum_sha256`, `state`, `job_id`, `last_activity`.
- Define `AdapterStore` wrapping `Arc<RwLock<HashMap<String, AdapterRecord>>>`.
- Implement `adapter_id_from_image_ref(image_ref: &str) -> String` (deterministic, e.g., stable hash).
- Implement `insert`, `get`, `list`, `remove` operations.
- Implement `transition(adapter_id, new_state) -> Result<(AdapterState, AdapterState), Error>` that validates via `can_transition_to` and returns `(old_state, new_state)`.
- Wire up a `tokio::sync::broadcast::Sender<AdapterStateEvent>` to emit events on successful transitions.

**Verify:** `cd rhivos && cargo test -p update-service -- store` -- all store unit tests pass.

### Task 2.3: Implement error types

In `rhivos/update-service/src/error.rs`:

- Define `UpdateServiceError` enum covering: `AdapterNotFound`, `AdapterAlreadyExists`, `InvalidTransition`, `InvalidArgument`, `ChecksumMismatch`, `Internal`.
- Implement `From<UpdateServiceError> for tonic::Status` mapping each variant to the appropriate gRPC status code.

**Verify:** `cd rhivos && cargo build -p update-service` compiles without errors.

---

## Task Group 3: gRPC Service Implementation

**Goal:** Implement the five gRPC RPCs. Most integration tests from group 1 should pass after this group (except streaming and offloading tests).

**Depends on:** Task Group 2

### Task 3.1: Implement gRPC service struct

In `rhivos/update-service/src/grpc.rs`:

- Define `UpdateServiceImpl` struct holding `AdapterStore` and `broadcast::Sender`.
- Implement the `UpdateService` tonic trait.

### Task 3.2: Implement InstallAdapter RPC

In `rhivos/update-service/src/grpc.rs`:

- Validate `image_ref` and `checksum_sha256` are non-empty; return `INVALID_ARGUMENT` if empty.
- Compute `adapter_id` from `image_ref`.
- Check for duplicate: if adapter already exists and is not in terminal state, return `ALREADY_EXISTS`.
- Generate a UUID `job_id`.
- Insert a new `AdapterRecord` with state `DOWNLOADING`.
- Spawn a background task to run the simulated container lifecycle (see Task Group 5).
- Return `InstallAdapterResponse { job_id, adapter_id, state: DOWNLOADING }`.

**Verify:** `cd rhivos && cargo test -p update-service -- test_install_adapter` passes.

### Task 3.3: Implement GetAdapterStatus RPC

- Look up `adapter_id` in the store.
- Return `GetAdapterStatusResponse` with current state and `image_ref`.
- Return `NOT_FOUND` if adapter does not exist.

**Verify:** `cd rhivos && cargo test -p update-service -- test_get_adapter_status` and `test_get_status_not_found` pass.

### Task 3.4: Implement ListAdapters RPC

- Return all adapters from the store as `ListAdaptersResponse`.

**Verify:** `cd rhivos && cargo test -p update-service -- test_list_adapters` passes.

### Task 3.5: Implement RemoveAdapter RPC

- Look up `adapter_id`; return `NOT_FOUND` if missing.
- Based on current state, transition through the appropriate path to `OFFLOADING`.
- Remove the adapter from the store.
- Return `RemoveAdapterResponse`.

**Verify:** `cd rhivos && cargo test -p update-service -- test_remove` passes.

### Task 3.6: Implement server startup

In `rhivos/update-service/src/main.rs`:

- Initialize tracing subscriber.
- Create `AdapterStore` and broadcast channel.
- Build `UpdateServiceImpl`.
- Start tonic server on `0.0.0.0:50051`.
- Log startup message.

**Verify:** `cd rhivos && cargo run -p update-service` starts and listens on port 50051.

---

## Task Group 4: WatchAdapterStates Streaming and Event Broadcasting

**Goal:** Implement the server-streaming RPC and event broadcasting. Streaming-related integration tests should pass after this group.

**Depends on:** Task Group 3

### Task 4.1: Implement WatchAdapterStates RPC

In `rhivos/update-service/src/grpc.rs`:

- Create a new `broadcast::Receiver` from the store's broadcast sender.
- Return a `ReceiverStream` that converts broadcast events to gRPC `AdapterStateEvent` messages.
- Handle `RecvError::Lagged` by logging a warning and continuing.
- The stream ends when the client disconnects.

**Verify:** `cd rhivos && cargo test -p update-service -- test_watch_adapter_states` and `test_state_transitions_lifecycle` pass.

### Task 4.2: Verify multi-client streaming

Ensure the broadcast channel supports multiple concurrent subscribers.

**Verify:** `cd rhivos && cargo test -p update-service -- test_watch_adapter_states_streams_events` passes (multi-client test from TS-07-3).

---

## Task Group 5: Simulated Container Lifecycle

**Goal:** Implement the simulated container download, checksum verification, and installation. Checksum-related tests should pass after this group.

**Depends on:** Task Group 3

### Task 5.1: Implement container manager

In `rhivos/update-service/src/container.rs`:

- Define `ContainerManager` struct with configurable delays.
- Implement `simulate_download(image_ref) -> Vec<u8>`: returns a simulated manifest digest (deterministic from `image_ref`).
- Implement `verify_checksum(manifest_digest, expected_checksum) -> bool`: computes SHA-256 and compares.
- Implement `simulate_install()`: sleep to simulate extraction.
- Implement `simulate_start()`: sleep to simulate container startup.
- Implement `simulate_stop()`: sleep to simulate container shutdown.

### Task 5.2: Wire container lifecycle into InstallAdapter

Update the background task spawned by `InstallAdapter` (from Task 3.2):

1. Call `simulate_download` with delay.
2. Call `verify_checksum`; if mismatch, transition to `ERROR` and return.
3. Transition `DOWNLOADING -> INSTALLING`.
4. Call `simulate_install`.
5. Call `simulate_start`.
6. Transition `INSTALLING -> RUNNING`.

Each transition emits a broadcast event.

**Verify:** `cd rhivos && cargo test -p update-service -- test_checksum_mismatch` and `test_state_transitions_lifecycle` pass.

### Task 5.3: Wire container stop into RemoveAdapter

Update `RemoveAdapter` to call `simulate_stop` before transitioning a `RUNNING` adapter to `STOPPED`.

**Verify:** `cd rhivos && cargo test -p update-service -- test_remove_adapter` passes.

---

## Task Group 6: Offloading Logic

**Goal:** Implement the inactivity timer and automatic offloading. Offloading tests should pass after this group.

**Depends on:** Task Group 5

### Task 6.1: Implement offload timer background task

In `rhivos/update-service/src/offload.rs`:

- Define configuration: `offload_timeout` and `check_interval` (from environment variables with defaults).
- Implement `start_offload_task(store: AdapterStore)`: spawns a `tokio` task that runs on `check_interval`.
- On each tick: iterate all adapters in `STOPPED` or `ERROR` state; if `now - last_activity > offload_timeout`, transition to `OFFLOADING` and remove from store.

### Task 6.2: Wire offload task into server startup

In `main.rs`, spawn the offload background task after creating the store.

### Task 6.3: Write and verify offloading integration test

Implement or update `test_offloading_after_inactivity` (TS-07-E6):

- Configure short offload timeout (2 seconds) and check interval (1 second).
- Install adapter, wait for `RUNNING`.
- Simulate stop (or use an internal method to transition to `STOPPED`).
- Wait for offload timeout + check interval.
- Verify adapter is removed from `ListAdapters`.
- Verify `WatchAdapterStates` emits `STOPPED -> OFFLOADING` event.

**Verify:** `cd rhivos && cargo test -p update-service -- test_offloading` passes.

---

## Task Group 7: Checkpoint

**Goal:** Final verification that all tests pass, code is clean, and the service is ready.

**Depends on:** Task Groups 1-6

### Task 7.1: Run full test suite

Run all tests and verify they pass:

```bash
cd rhivos && cargo test -p update-service
```

**Expected:** All tests pass (0 failures).

### Task 7.2: Run linter

```bash
cd rhivos && cargo clippy -p update-service -- -D warnings
```

**Expected:** No warnings or errors.

### Task 7.3: Run format check

```bash
cd rhivos && cargo fmt -p update-service -- --check
```

**Expected:** No formatting issues.

### Task 7.4: Manual smoke test

Start the service and verify basic operation:

1. `cd rhivos && cargo run -p update-service`
2. Use `grpcurl` or a test client to call `InstallAdapter`.
3. Verify the response contains `job_id`, `adapter_id`, and `state=DOWNLOADING`.
4. Call `ListAdapters` and verify the adapter appears.
5. Call `GetAdapterStatus` and verify the state.

### Task 7.5: Review Definition of Done

Verify all items from the design document's Definition of Done checklist are satisfied.

---

## Traceability Table

| Task | Requirement(s) | Test(s) |
|------|----------------|---------|
| 1.1 | 07-REQ-8.1 | -- (build verification) |
| 1.2 | 07-REQ-1.1, 07-REQ-3.1, 07-REQ-4.1, 07-REQ-5.1 | -- (proto definition) |
| 1.3 | 07-REQ-2.1 | TS-07-P1, TS-07-P3 |
| 1.4 | 07-REQ-1.1 | TS-07-P2 |
| 1.5 | 07-REQ-1.1 through 07-REQ-8.1 | TS-07-1 through TS-07-E5 |
| 2.1 | 07-REQ-2.1 | TS-07-P1, TS-07-P3 |
| 2.2 | 07-REQ-1.1, 07-REQ-2.1, 07-REQ-4.1 | TS-07-P2, TS-07-4, TS-07-6 |
| 2.3 | 07-REQ-1.1, 07-REQ-4.1, 07-REQ-5.1 | TS-07-E2, TS-07-E3, TS-07-E4, TS-07-E5 |
| 3.1 | 07-REQ-8.1 | -- (infrastructure) |
| 3.2 | 07-REQ-1.1 | TS-07-1, TS-07-E2, TS-07-E5 |
| 3.3 | 07-REQ-4.1 | TS-07-6, TS-07-E3 |
| 3.4 | 07-REQ-4.1 | TS-07-4 |
| 3.5 | 07-REQ-5.1 | TS-07-5, TS-07-E4 |
| 3.6 | 07-REQ-8.1 | -- (manual verification) |
| 4.1 | 07-REQ-3.1 | TS-07-2, TS-07-3 |
| 4.2 | 07-REQ-3.1 | TS-07-3 |
| 5.1 | 07-REQ-6.1 | TS-07-E1 |
| 5.2 | 07-REQ-2.1, 07-REQ-6.1 | TS-07-2, TS-07-E1 |
| 5.3 | 07-REQ-5.1 | TS-07-5 |
| 6.1 | 07-REQ-7.1 | TS-07-E6 |
| 6.2 | 07-REQ-7.1 | TS-07-E6 |
| 6.3 | 07-REQ-7.1 | TS-07-E6 |
| 7.1-7.5 | All | All |
