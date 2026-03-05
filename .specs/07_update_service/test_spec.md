# Test Specification: UPDATE_SERVICE (Spec 07)

> Test specifications for the UPDATE_SERVICE gRPC service managing containerized adapter lifecycle in the RHIVOS QM partition.

## References

- Requirements: `.specs/07_update_service/requirements.md`
- Design: `.specs/07_update_service/design.md`

## Test Naming Convention

- **TS-07-N:** Happy-path / positive tests
- **TS-07-PN:** Property-based / state machine tests
- **TS-07-EN:** Error / edge-case tests

## Test Environment

- Tests are written in Rust and located in `rhivos/update-service/tests/` (integration) and inline in source modules (unit).
- Integration tests start the gRPC server in-process using `tonic` and connect via a loopback client.
- The simulated container manager uses short delays (10-50ms) in test mode to keep tests fast.
- The offload timer is configured to a short duration (1-2 seconds) for offloading tests.

## Test Commands

- **Run all tests:** `cd rhivos && cargo test -p update-service`
- **Run a specific test:** `cd rhivos && cargo test -p update-service -- <test_name>`
- **Lint:** `cd rhivos && cargo clippy -p update-service -- -D warnings`

---

## Happy-Path Tests

### TS-07-1: InstallAdapter returns correct initial state

**Covers:** 07-REQ-1.1

**Scenario:** A client calls `InstallAdapter` with a valid `image_ref` and matching `checksum_sha256`.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Call `InstallAdapter` with `image_ref = "registry.example.com/adapters/demo:v1.0"` and a valid `checksum_sha256`.
3. Verify the response.

**Expected result:**
- Response contains a non-empty `job_id` in UUID format.
- Response contains a non-empty `adapter_id`.
- Response `state` is `DOWNLOADING`.

---

### TS-07-2: State transitions follow valid lifecycle path

**Covers:** 07-REQ-2.1, 07-REQ-6.1

**Scenario:** After calling `InstallAdapter`, the adapter progresses through the complete happy-path lifecycle: `DOWNLOADING -> INSTALLING -> RUNNING`.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Call `WatchAdapterStates` to open a streaming connection.
3. Call `InstallAdapter` with a valid `image_ref` and matching `checksum_sha256`.
4. Collect state transition events from the stream until `RUNNING` is reached.

**Expected result:**
- Event 1: `old_state = DOWNLOADING`, `new_state = INSTALLING`.
- Event 2: `old_state = INSTALLING`, `new_state = RUNNING`.
- No other state transitions occur for this adapter during this sequence.

---

### TS-07-3: WatchAdapterStates streams state changes to multiple clients

**Covers:** 07-REQ-3.1

**Scenario:** Two clients subscribe to `WatchAdapterStates` concurrently; both receive the same events.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Open two `WatchAdapterStates` streams from two separate client connections.
3. Call `InstallAdapter` with a valid request.
4. Collect events from both streams until `RUNNING` is reached.

**Expected result:**
- Both streams receive the same `AdapterStateEvent` messages (same `adapter_id`, `old_state`, `new_state`).
- Events arrive in the same order on both streams.

---

### TS-07-4: ListAdapters returns all installed adapters

**Covers:** 07-REQ-4.1

**Scenario:** Multiple adapters are installed; `ListAdapters` returns all of them.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Install adapter A with `image_ref = "registry.example.com/adapters/alpha:v1.0"`.
3. Wait for adapter A to reach `RUNNING`.
4. Install adapter B with `image_ref = "registry.example.com/adapters/beta:v1.0"`.
5. Wait for adapter B to reach `RUNNING`.
6. Call `ListAdapters`.

**Expected result:**
- Response contains exactly 2 adapters.
- Both adapters have `state = RUNNING`.
- The `adapter_id` and `image_ref` fields match the installed adapters.

---

### TS-07-5: RemoveAdapter stops and removes an adapter

**Covers:** 07-REQ-5.1

**Scenario:** A running adapter is removed via `RemoveAdapter`.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Install an adapter and wait for it to reach `RUNNING`.
3. Open a `WatchAdapterStates` stream.
4. Call `RemoveAdapter` with the adapter's `adapter_id`.
5. Collect state transition events from the stream.
6. Call `ListAdapters`.

**Expected result:**
- State transition events include `RUNNING -> STOPPED` and `STOPPED -> OFFLOADING`.
- `ListAdapters` returns an empty list (or does not include the removed adapter).

---

### TS-07-6: GetAdapterStatus returns current state

**Covers:** 07-REQ-4.1

**Scenario:** An adapter is installed and queried at different lifecycle stages.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Install an adapter.
3. Call `GetAdapterStatus` with the `adapter_id` from the install response.
4. Wait for the adapter to reach `RUNNING`.
5. Call `GetAdapterStatus` again.

**Expected result:**
- The first call returns a state that is one of `DOWNLOADING`, `INSTALLING`, or `RUNNING` (depending on timing).
- After waiting for `RUNNING`, `GetAdapterStatus` returns `state = RUNNING`.
- The `image_ref` matches the installed image reference.

---

## Property Tests

### TS-07-P1: State machine rejects invalid transitions

**Covers:** 07-REQ-2.1

**Scenario:** Verify that the state machine rejects transitions that are not in the valid transition set.

**Steps:**
1. For each pair `(from_state, to_state)` NOT in the valid transition table, call `can_transition_to(from_state, to_state)`.

**Expected result:**
- All invalid transitions return `false`.
- Specifically verify: `DOWNLOADING -> RUNNING` (must skip `INSTALLING`), `RUNNING -> DOWNLOADING`, `UNKNOWN -> RUNNING`, `OFFLOADING -> RUNNING`.

---

### TS-07-P2: Deterministic adapter ID from image_ref

**Covers:** 07-REQ-1.1 (deterministic adapter_id)

**Scenario:** The same `image_ref` always produces the same `adapter_id`.

**Steps:**
1. Compute `adapter_id` from `image_ref = "registry.example.com/adapters/demo:v1.0"` twice.
2. Compute `adapter_id` from a different `image_ref`.

**Expected result:**
- The two computations from the same `image_ref` produce identical `adapter_id` values.
- The computation from a different `image_ref` produces a different `adapter_id`.

---

### TS-07-P3: All valid transitions are accepted

**Covers:** 07-REQ-2.1

**Scenario:** Verify that every transition in the valid transition table is accepted.

**Steps:**
1. For each pair `(from_state, to_state)` in the valid transition table, call `can_transition_to(from_state, to_state)`.

**Expected result:**
- All valid transitions return `true`.
- Verify all 10 valid transitions listed in 07-REQ-2.1.

---

## Error / Edge-Case Tests

### TS-07-E1: Checksum mismatch rejects installation

**Covers:** 07-REQ-6.1

**Scenario:** `InstallAdapter` is called with a `checksum_sha256` that does not match the downloaded image.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Open a `WatchAdapterStates` stream.
3. Call `InstallAdapter` with a valid `image_ref` but an incorrect `checksum_sha256` (e.g., `"0000000000000000000000000000000000000000000000000000000000000000"`).
4. Collect state transition events from the stream.

**Expected result:**
- The adapter transitions from `DOWNLOADING` to `ERROR`.
- A `WatchAdapterStates` event is emitted with `old_state = DOWNLOADING`, `new_state = ERROR`.
- The adapter does NOT reach `INSTALLING` or `RUNNING`.

---

### TS-07-E2: Duplicate install returns ALREADY_EXISTS

**Covers:** 07-REQ-1.1

**Scenario:** `InstallAdapter` is called twice with the same `image_ref` while the first is still in progress or running.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Call `InstallAdapter` with `image_ref = "registry.example.com/adapters/demo:v1.0"`.
3. Immediately call `InstallAdapter` again with the same `image_ref`.

**Expected result:**
- The second call returns gRPC status `ALREADY_EXISTS`.
- The error details include the existing `adapter_id`.

---

### TS-07-E3: GetAdapterStatus with invalid adapter_id returns NOT_FOUND

**Covers:** 07-REQ-4.1

**Scenario:** `GetAdapterStatus` is called with an `adapter_id` that does not exist in the store.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Call `GetAdapterStatus` with `adapter_id = "nonexistent-adapter-id"`.

**Expected result:**
- The RPC returns gRPC status `NOT_FOUND`.
- The error message contains the requested `adapter_id`.

---

### TS-07-E4: RemoveAdapter with invalid adapter_id returns NOT_FOUND

**Covers:** 07-REQ-5.1

**Scenario:** `RemoveAdapter` is called with an `adapter_id` that does not exist.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Call `RemoveAdapter` with `adapter_id = "nonexistent-adapter-id"`.

**Expected result:**
- The RPC returns gRPC status `NOT_FOUND`.

---

### TS-07-E5: InstallAdapter with empty image_ref returns INVALID_ARGUMENT

**Covers:** 07-REQ-1.1, 07-REQ-8.1

**Scenario:** `InstallAdapter` is called with an empty `image_ref`.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server.
2. Call `InstallAdapter` with `image_ref = ""` and a non-empty `checksum_sha256`.

**Expected result:**
- The RPC returns gRPC status `INVALID_ARGUMENT`.
- The error message indicates that `image_ref` must not be empty.

---

### TS-07-E6: Offloading after inactivity timeout

**Covers:** 07-REQ-7.1

**Scenario:** A stopped adapter is automatically offloaded after the inactivity timeout expires.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server with a short offload timeout (e.g., 2 seconds) and short check interval (e.g., 1 second).
2. Install an adapter and wait for it to reach `RUNNING`.
3. Open a `WatchAdapterStates` stream.
4. Stop the adapter (via `RemoveAdapter` transitioning to `STOPPED`, or a simulated stop that leaves it in `STOPPED` without full removal).
5. Wait for the offload timeout to expire (e.g., sleep 3 seconds).
6. Collect events from the `WatchAdapterStates` stream.
7. Call `ListAdapters`.

**Expected result:**
- A `WatchAdapterStates` event is emitted with `old_state = STOPPED`, `new_state = OFFLOADING`.
- After offloading completes, the adapter does not appear in `ListAdapters`.

---

### TS-07-E7: RemoveAdapter cancels in-progress download

**Covers:** 07-REQ-5.1

**Scenario:** `RemoveAdapter` is called while an adapter is in `DOWNLOADING` state.

**Steps:**
1. Start the UPDATE_SERVICE gRPC server with a long simulated download time (e.g., 5 seconds).
2. Call `InstallAdapter` to begin downloading.
3. Immediately call `RemoveAdapter` with the `adapter_id` from the install response.
4. Collect state transition events.

**Expected result:**
- The adapter transitions through `ERROR` and then `OFFLOADING`.
- The adapter is removed from the system.
- `ListAdapters` does not include the adapter.

---

## Traceability Matrix

| Test ID | Requirement | Description |
|---------|-------------|-------------|
| TS-07-1 | 07-REQ-1.1 | InstallAdapter returns correct initial state |
| TS-07-2 | 07-REQ-2.1, 07-REQ-6.1 | State transitions follow valid lifecycle |
| TS-07-3 | 07-REQ-3.1 | WatchAdapterStates streams to multiple clients |
| TS-07-4 | 07-REQ-4.1 | ListAdapters returns all adapters |
| TS-07-5 | 07-REQ-5.1 | RemoveAdapter stops and removes adapter |
| TS-07-6 | 07-REQ-4.1 | GetAdapterStatus returns current state |
| TS-07-P1 | 07-REQ-2.1 | Invalid state transitions rejected |
| TS-07-P2 | 07-REQ-1.1 | Deterministic adapter ID |
| TS-07-P3 | 07-REQ-2.1 | All valid transitions accepted |
| TS-07-E1 | 07-REQ-6.1 | Checksum mismatch rejects installation |
| TS-07-E2 | 07-REQ-1.1 | Duplicate install returns ALREADY_EXISTS |
| TS-07-E3 | 07-REQ-4.1 | Invalid adapter_id returns NOT_FOUND |
| TS-07-E4 | 07-REQ-5.1 | RemoveAdapter with invalid ID returns NOT_FOUND |
| TS-07-E5 | 07-REQ-1.1 | Empty image_ref returns INVALID_ARGUMENT |
| TS-07-E6 | 07-REQ-7.1 | Offloading after inactivity timeout |
| TS-07-E7 | 07-REQ-5.1 | RemoveAdapter cancels in-progress download |
