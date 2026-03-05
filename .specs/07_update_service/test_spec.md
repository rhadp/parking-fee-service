# Test Specification: UPDATE_SERVICE (Spec 07)

> Test specifications for the UPDATE_SERVICE adapter lifecycle manager.
> Validates requirements from `.specs/07_update_service/requirements.md`.

## Test ID Convention

| Prefix  | Category           |
|---------|--------------------|
| TS-07-  | Functional tests   |
| TS-07-P | Property tests     |
| TS-07-E | Error/edge tests   |

## Test Environment

- **Test framework:** Rust `#[tokio::test]` with mockall for trait mocking
- **gRPC testing:** tonic test client against in-process server
- **Test location:** `rhivos/update-service/src/*_test.rs`
- **Run command:** `cd rhivos && cargo test -p update-service`
- **Lint command:** `cd rhivos && cargo clippy -p update-service`
- **Mocking strategy:** Podman CLI interactions are mocked via a `ContainerRuntime` trait. OCI registry interactions are mocked via an `OciPuller` trait.

## Functional Tests

### TS-07-1: Install Adapter Happy Path

**Requirement:** 07-REQ-1, 07-REQ-2, 07-REQ-6

**Description:** Calling `InstallAdapter` with a valid image reference and correct checksum pulls the image, verifies the checksum, installs the container, and transitions through DOWNLOADING -> INSTALLING -> RUNNING.

**Preconditions:** No adapters are currently installed. Podman and registry mocks return success.

**Steps:**

1. Configure mock OCI puller to return a known manifest digest for `test-image:v1.0`.
2. Configure mock container runtime to succeed on `run`.
3. Call `InstallAdapter(image_ref="test-image:v1.0", checksum_sha256="sha256:<valid>")`.
4. Assert response contains a non-empty `job_id`, non-empty `adapter_id`, and `state` = DOWNLOADING.
5. Wait for the adapter to reach RUNNING state (poll via `GetAdapterStatus`).
6. Assert `GetAdapterStatus` returns `state` = RUNNING.

**Expected result:** Adapter transitions DOWNLOADING -> INSTALLING -> RUNNING. Response contains valid job_id and adapter_id.

### TS-07-2: State Transition Streaming

**Requirement:** 07-REQ-3

**Description:** A `WatchAdapterStates` subscriber receives all state transition events during an adapter installation.

**Preconditions:** No adapters installed. Mocks configured for successful install.

**Steps:**

1. Open a `WatchAdapterStates` stream.
2. Call `InstallAdapter` with valid parameters.
3. Collect events from the stream until RUNNING state is reached.
4. Assert the following events were received in order:
   - `{old_state: UNKNOWN, new_state: DOWNLOADING}`
   - `{old_state: DOWNLOADING, new_state: INSTALLING}`
   - `{old_state: INSTALLING, new_state: RUNNING}`
5. Assert each event has a valid `adapter_id` and `timestamp > 0`.

**Expected result:** Three state transition events received in correct order.

### TS-07-3: Checksum Verification -- Valid Checksum

**Requirement:** 07-REQ-2

**Description:** When the computed checksum matches the provided checksum, installation proceeds to INSTALLING and then RUNNING.

**Preconditions:** Mock OCI puller returns a manifest digest whose SHA-256 matches the provided checksum.

**Steps:**

1. Configure mock with a known digest and matching checksum.
2. Call `InstallAdapter` with matching `checksum_sha256`.
3. Wait for RUNNING state.
4. Assert adapter reached RUNNING state.

**Expected result:** Adapter reaches RUNNING state successfully.

### TS-07-4: Single Adapter Enforcement

**Requirement:** 07-REQ-7

**Description:** When installing a new adapter while another is RUNNING, the currently running adapter is stopped before the new one starts.

**Preconditions:** One adapter (`adapter-A`) is already in RUNNING state. Mocks configured for successful operations.

**Steps:**

1. Install `adapter-A` and wait for RUNNING state.
2. Call `InstallAdapter` for a different image (`adapter-B`).
3. Wait for `adapter-B` to reach RUNNING state.
4. Call `ListAdapters`.
5. Assert `adapter-A` is in STOPPED state.
6. Assert `adapter-B` is in RUNNING state.
7. Assert exactly one adapter is in RUNNING state.

**Expected result:** Old adapter stopped, new adapter running. Only one RUNNING adapter at any time.

### TS-07-5: List Adapters

**Requirement:** 07-REQ-4

**Description:** `ListAdapters` returns all known adapters with their current states.

**Preconditions:** Two adapters have been installed (one RUNNING, one STOPPED).

**Steps:**

1. Install `adapter-A`, wait for RUNNING.
2. Install `adapter-B`, wait for RUNNING (this stops `adapter-A`).
3. Call `ListAdapters`.
4. Assert response contains two adapters.
5. Assert one is RUNNING and one is STOPPED.
6. Assert each adapter has a valid `adapter_id` and `image_ref`.

**Expected result:** Both adapters listed with correct states.

### TS-07-6: Get Adapter Status

**Requirement:** 07-REQ-4

**Description:** `GetAdapterStatus` returns the current state and metadata of a specific adapter.

**Preconditions:** One adapter is installed and RUNNING.

**Steps:**

1. Install an adapter and wait for RUNNING.
2. Call `GetAdapterStatus(adapter_id)` with the adapter's ID.
3. Assert response contains correct `adapter_id`, `image_ref`, and `state` = RUNNING.

**Expected result:** Correct adapter status returned.

### TS-07-7: Remove Adapter

**Requirement:** 07-REQ-5

**Description:** `RemoveAdapter` stops and removes an adapter.

**Preconditions:** One adapter is installed and RUNNING. Mock container runtime succeeds on stop and remove.

**Steps:**

1. Install an adapter and wait for RUNNING.
2. Call `RemoveAdapter(adapter_id)`.
3. Assert response `success` = true.
4. Call `ListAdapters`.
5. Assert the removed adapter is no longer in the list (or is in OFFLOADING state transitioning to removal).

**Expected result:** Adapter is stopped, removed, and no longer listed.

### TS-07-8: Offloading After Inactivity

**Requirement:** 07-REQ-8

**Description:** A stopped adapter is automatically offloaded after the configured inactivity timeout expires.

**Preconditions:** Inactivity timeout configured to a short duration (e.g., 1 second for testing). One adapter is installed.

**Steps:**

1. Configure inactivity timeout to 1 second.
2. Install an adapter and wait for RUNNING.
3. Stop the adapter (via installing a different one, or RemoveAdapter followed by reinstall to STOPPED).
4. Use `tokio::time::advance` (or equivalent) to advance time past the inactivity timeout.
5. Call `ListAdapters`.
6. Assert the stopped adapter has been offloaded (no longer in the list).

**Expected result:** Adapter automatically removed after inactivity timeout.

## Error and Edge Case Tests

### TS-07-E1: Checksum Verification -- Invalid Checksum

**Requirement:** 07-REQ-2

**Description:** When the computed checksum does not match the provided checksum, the adapter transitions to ERROR and the image is cleaned up.

**Steps:**

1. Configure mock OCI puller to return a digest that does NOT match the provided checksum.
2. Call `InstallAdapter` with a mismatched `checksum_sha256`.
3. Assert gRPC status is INVALID_ARGUMENT.
4. Assert error message contains "checksum mismatch".
5. Call `GetAdapterStatus` for the adapter.
6. Assert state is ERROR.

**Expected result:** INVALID_ARGUMENT error with adapter in ERROR state.

### TS-07-E2: Registry Unreachable

**Requirement:** 07-REQ-1, 07-REQ-10

**Description:** When the OCI registry is unreachable, InstallAdapter returns UNAVAILABLE and the adapter transitions to ERROR.

**Steps:**

1. Configure mock OCI puller to fail with a connection error.
2. Call `InstallAdapter` with valid parameters.
3. Assert gRPC status is UNAVAILABLE.
4. Assert error message describes the connectivity failure.

**Expected result:** UNAVAILABLE error returned.

### TS-07-E3: Container Start Failure

**Requirement:** 07-REQ-1, 07-REQ-10

**Description:** When the container fails to start after installation, the adapter transitions to ERROR.

**Steps:**

1. Configure mock OCI puller to succeed.
2. Configure mock container runtime to fail on `run` (non-zero exit code).
3. Call `InstallAdapter` with valid parameters.
4. Wait for state transition.
5. Assert adapter state is ERROR.
6. Assert gRPC error status is INTERNAL.

**Expected result:** ERROR state with INTERNAL gRPC status.

### TS-07-E4: Get Status for Unknown Adapter

**Requirement:** 07-REQ-4

**Description:** Querying status for a non-existent adapter returns NOT_FOUND.

**Steps:**

1. Call `GetAdapterStatus("nonexistent-adapter")`.
2. Assert gRPC status is NOT_FOUND.

**Expected result:** NOT_FOUND error.

### TS-07-E5: Remove Unknown Adapter

**Requirement:** 07-REQ-5

**Description:** Removing a non-existent adapter returns NOT_FOUND.

**Steps:**

1. Call `RemoveAdapter("nonexistent-adapter")`.
2. Assert gRPC status is NOT_FOUND.

**Expected result:** NOT_FOUND error.

### TS-07-E6: Install Already Running Adapter (Same Image)

**Requirement:** 07-REQ-1

**Description:** Installing an adapter that is already running with the same image_ref returns ALREADY_EXISTS.

**Steps:**

1. Install an adapter and wait for RUNNING.
2. Call `InstallAdapter` again with the same `image_ref`.
3. Assert gRPC status is ALREADY_EXISTS.
4. Assert the adapter is still RUNNING (not restarted).

**Expected result:** ALREADY_EXISTS error, adapter unchanged.

## Property Tests

### TS-07-P1: State Machine Transition Validity

**Requirement:** 07-REQ-6

**Description:** Every attempted state transition is either a valid transition (accepted) or an invalid transition (rejected). No undefined behavior.

**Properties tested:**

1. For each valid transition pair in the state machine, `transition(from, to)` succeeds.
2. For each invalid transition pair, `transition(from, to)` returns an error.
3. The set of valid transitions exactly matches the specification in 07-REQ-6.1.

**Implementation:** Enumerate all (state, state) pairs and verify each produces the expected accept/reject outcome.

**Expected result:** All valid transitions accepted, all invalid transitions rejected.

### TS-07-P2: Single Adapter Invariant

**Requirement:** 07-REQ-7

**Description:** After any sequence of InstallAdapter and RemoveAdapter operations, at most one adapter is in RUNNING state.

**Properties tested:**

1. After each operation in a sequence of random install/remove calls, `ListAdapters` shows at most one RUNNING adapter.

**Implementation:** Randomized sequence of 10-20 install/remove operations with assertions after each step.

**Expected result:** Invariant holds after every operation.

## Traceability

| Test ID   | Requirement(s)             | Category    |
|-----------|----------------------------|-------------|
| TS-07-1   | 07-REQ-1, 07-REQ-2, 07-REQ-6 | Functional  |
| TS-07-2   | 07-REQ-3                   | Functional  |
| TS-07-3   | 07-REQ-2                   | Functional  |
| TS-07-4   | 07-REQ-7                   | Functional  |
| TS-07-5   | 07-REQ-4                   | Functional  |
| TS-07-6   | 07-REQ-4                   | Functional  |
| TS-07-7   | 07-REQ-5                   | Functional  |
| TS-07-8   | 07-REQ-8                   | Functional  |
| TS-07-E1  | 07-REQ-2                   | Error/Edge  |
| TS-07-E2  | 07-REQ-1, 07-REQ-10        | Error/Edge  |
| TS-07-E3  | 07-REQ-1, 07-REQ-10        | Error/Edge  |
| TS-07-E4  | 07-REQ-4                   | Error/Edge  |
| TS-07-E5  | 07-REQ-5                   | Error/Edge  |
| TS-07-E6  | 07-REQ-1                   | Error/Edge  |
| TS-07-P1  | 07-REQ-6                   | Property    |
| TS-07-P2  | 07-REQ-7                   | Property    |
