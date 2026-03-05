# Implementation Tasks: UPDATE_SERVICE (Spec 07)

> Task breakdown for implementing the UPDATE_SERVICE adapter lifecycle manager.
> Implements design from `.specs/07_update_service/design.md`.
> Validates against `.specs/07_update_service/test_spec.md`.

## Dependencies

| Spec | Dependency | Relationship |
|------|-----------|--------------|
| 01_project_setup | Group 2 (Rust workspace) | Requires Rust workspace structure, `rhivos/Cargo.toml` workspace includes `update-service` |

## Test Commands

| Action | Command |
|--------|---------|
| Unit tests | `cd rhivos && cargo test -p update-service` |
| Lint | `cd rhivos && cargo clippy -p update-service` |

## Task Groups

### Group 1: Write Failing Spec Tests

**Goal:** Create Rust test files that encode all test specifications. Tests must compile but fail (red phase of red-green-refactor).

#### Task 1.1: Initialize Cargo crate

Create `rhivos/update-service/Cargo.toml` with dependencies: tonic, prost, tokio (full), uuid, toml, tracing, tracing-subscriber. Dev dependencies: mockall, tonic (features: transport), tokio-test. Add crate to `rhivos/Cargo.toml` workspace members.

**Files:** `rhivos/update-service/Cargo.toml`, `rhivos/Cargo.toml`

#### Task 1.2: Create proto file

Create `proto/update_service/v1/update_service.proto` with the full service definition as specified in design.md. Create `rhivos/update-service/build.rs` for tonic-build compilation.

**Files:** `proto/update_service/v1/update_service.proto`, `rhivos/update-service/build.rs`

#### Task 1.3: Create type stubs and trait definitions

Create minimal stub files so that tests compile:

- `src/main.rs` -- minimal main with module declarations
- `src/state.rs` -- `AdapterState` enum stub with `UNKNOWN` variant, `StateMachine` trait stub
- `src/config.rs` -- `Config` struct stub
- `src/oci.rs` -- `OciPuller` trait stub
- `src/container.rs` -- `ContainerRuntime` trait stub
- `src/manager.rs` -- `AdapterManager` struct stub
- `src/grpc.rs` -- gRPC service struct stub
- `src/offload.rs` -- offload timer stub

**Files:** `rhivos/update-service/src/*.rs`

#### Task 1.4: Write state machine tests

Create `rhivos/update-service/src/state_test.rs` with test functions covering:

- `test_valid_transitions` -- all valid transitions from 07-REQ-6.1 (TS-07-P1)
- `test_invalid_transitions` -- invalid transitions are rejected (TS-07-P1)
- `test_all_states_represented` -- all 7 states exist in the enum

Tests should compile but fail because the state machine is not yet implemented.

**Files:** `rhivos/update-service/src/state_test.rs`

#### Task 1.5: Write manager tests

Create `rhivos/update-service/src/manager_test.rs` with test functions covering:

- `test_install_adapter_happy_path` (TS-07-1)
- `test_single_adapter_enforcement` (TS-07-4)
- `test_list_adapters` (TS-07-5)
- `test_get_adapter_status` (TS-07-6)
- `test_remove_adapter` (TS-07-7)
- `test_checksum_mismatch` (TS-07-E1)
- `test_registry_unreachable` (TS-07-E2)
- `test_container_start_failure` (TS-07-E3)
- `test_get_status_unknown` (TS-07-E4)
- `test_remove_unknown` (TS-07-E5)
- `test_install_already_running` (TS-07-E6)

Tests use mockall-generated mocks for `OciPuller` and `ContainerRuntime` traits. Tests should compile but fail.

**Files:** `rhivos/update-service/src/manager_test.rs`

#### Task 1.6: Write gRPC integration tests

Create `rhivos/update-service/src/grpc_test.rs` with test functions covering:

- `test_grpc_install_and_watch` (TS-07-1, TS-07-2)
- `test_grpc_list_adapters` (TS-07-5)
- `test_grpc_get_status` (TS-07-6)
- `test_grpc_remove_adapter` (TS-07-7)

Tests start an in-process tonic server with mocked dependencies and use a tonic client. Tests should compile but fail.

**Files:** `rhivos/update-service/src/grpc_test.rs`

#### Task 1.7: Write offload tests

Create `rhivos/update-service/src/offload_test.rs` with test functions covering:

- `test_offload_after_inactivity` (TS-07-8)
- `test_no_offload_while_running` -- running adapters are not offloaded

Tests use tokio time pausing/advancing. Tests should compile but fail.

**Files:** `rhivos/update-service/src/offload_test.rs`

**Verification:** `cd rhivos && cargo test -p update-service` compiles but all tests fail (or are marked `#[ignore]` with comment "not implemented").

---

### Group 2: Implement gRPC Server and Proto Definitions

**Goal:** Implement the tonic gRPC server skeleton with all RPC methods stubbed.

#### Task 2.1: Finalize proto compilation

Ensure `build.rs` compiles the proto file and generated code is importable via `tonic::include_proto!("update_service.v1")`.

**Files:** `rhivos/update-service/build.rs`

#### Task 2.2: Implement gRPC service skeleton

Implement `src/grpc.rs` with a struct `UpdateServiceImpl` that implements the tonic-generated `UpdateService` trait. Each method returns `Err(Status::unimplemented("..."))` initially. Wire the service into `main.rs` with tonic `Server::builder`.

**Files:** `rhivos/update-service/src/grpc.rs`, `rhivos/update-service/src/main.rs`

#### Task 2.3: Implement configuration loading

Implement `src/config.rs` with:

- `Config` struct with fields: `grpc_port`, `registry_base_url`, `inactivity_timeout_secs`, `storage_path`
- `Config::load(path: Option<&str>)` function that loads from TOML file or defaults
- Environment variable overrides with `UPDATE_SERVICE_` prefix

**Files:** `rhivos/update-service/src/config.rs`

**Verification:** `cd rhivos && cargo build -p update-service` succeeds. Server starts and responds with UNIMPLEMENTED to all RPCs.

---

### Group 3: Implement OCI Pull and Checksum Verification

**Goal:** Implement the OCI image puller that shells out to podman and verifies SHA-256 checksums.

#### Task 3.1: Define OciPuller trait

Define the `OciPuller` trait in `src/oci.rs`:

```rust
#[async_trait]
pub trait OciPuller: Send + Sync {
    async fn pull_image(&self, image_ref: &str) -> Result<String, OciError>;  // returns digest
    async fn remove_image(&self, image_ref: &str) -> Result<(), OciError>;
}
```

Add `#[automock]` attribute for mockall.

**Files:** `rhivos/update-service/src/oci.rs`

#### Task 3.2: Implement PodmanOciPuller

Implement `PodmanOciPuller` struct that implements `OciPuller`:

- `pull_image`: Runs `podman pull <image_ref>`, then `podman inspect <image_ref> --format '{{.Digest}}'` to extract the digest.
- `remove_image`: Runs `podman rmi <image_ref>`.
- Maps podman errors to `OciError` variants.

**Files:** `rhivos/update-service/src/oci.rs`

#### Task 3.3: Implement checksum verification

Add a standalone function `verify_checksum(digest: &str, expected: &str) -> Result<(), ChecksumError>` that:

1. Computes SHA-256 of the digest string.
2. Formats as `sha256:<hex>`.
3. Compares with the expected checksum.
4. Returns `Ok(())` on match, `Err(ChecksumError::Mismatch{...})` otherwise.

**Files:** `rhivos/update-service/src/oci.rs`

**Verification:** OCI-related tests (TS-07-3, TS-07-E1) pass with mocked podman.

---

### Group 4: Implement Container Lifecycle Management (Podman)

**Goal:** Implement the container runtime abstraction that manages adapter containers via podman CLI.

#### Task 4.1: Define ContainerRuntime trait

Define the `ContainerRuntime` trait in `src/container.rs`:

```rust
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    async fn run(&self, name: &str, image_ref: &str) -> Result<(), ContainerError>;
    async fn stop(&self, name: &str) -> Result<(), ContainerError>;
    async fn remove(&self, name: &str) -> Result<(), ContainerError>;
    async fn status(&self, name: &str) -> Result<ContainerStatus, ContainerError>;
}
```

Add `#[automock]` attribute.

**Files:** `rhivos/update-service/src/container.rs`

#### Task 4.2: Implement PodmanRuntime

Implement `PodmanRuntime` struct:

- `run`: Executes `podman run -d --name <name> <image_ref>`.
- `stop`: Executes `podman stop <name>`.
- `remove`: Executes `podman rm -f <name>`.
- `status`: Executes `podman inspect <name> --format '{{.State.Status}}'`, parses output.
- All commands via `tokio::process::Command`.

**Files:** `rhivos/update-service/src/container.rs`

**Verification:** Container runtime tests pass with mocked commands. TS-07-E3 passes.

---

### Group 5: Implement State Machine and Streaming

**Goal:** Implement the adapter state machine, the AdapterManager, and the state event broadcasting.

#### Task 5.1: Implement state machine

Implement `src/state.rs` with:

- `AdapterState` enum with all 7 states.
- `AdapterState::can_transition_to(&self, target: AdapterState) -> bool` method encoding valid transitions from 07-REQ-6.1.
- `AdapterRecord` struct holding `adapter_id`, `image_ref`, `state`, `last_activity` timestamp.

**Files:** `rhivos/update-service/src/state.rs`

#### Task 5.2: Implement AdapterManager

Implement `src/manager.rs` with:

- `AdapterManager` struct holding a `HashMap<String, AdapterRecord>`, a `broadcast::Sender<AdapterStateEvent>`, and references to `OciPuller` and `ContainerRuntime` (via trait objects / generics).
- `install_adapter(image_ref, checksum)` method implementing the full flow: check single-adapter constraint, pull, verify, install, start.
- `remove_adapter(adapter_id)` method.
- `list_adapters()` method.
- `get_adapter_status(adapter_id)` method.
- `subscribe_state_events()` method returning a `broadcast::Receiver`.
- Private `transition_state(adapter_id, new_state)` method that validates transitions and emits events.

**Files:** `rhivos/update-service/src/manager.rs`

#### Task 5.3: Wire gRPC methods to AdapterManager

Update `src/grpc.rs` to delegate all RPC calls to `AdapterManager`:

- `InstallAdapter` -> `manager.install_adapter()`
- `WatchAdapterStates` -> `manager.subscribe_state_events()` wrapped in a tonic streaming response
- `ListAdapters` -> `manager.list_adapters()`
- `RemoveAdapter` -> `manager.remove_adapter()`
- `GetAdapterStatus` -> `manager.get_adapter_status()`

**Files:** `rhivos/update-service/src/grpc.rs`

**Verification:** State machine tests (TS-07-P1), manager tests (TS-07-1, TS-07-4, TS-07-5, TS-07-6, TS-07-7, TS-07-E1 through TS-07-E6), streaming tests (TS-07-2), and gRPC integration tests pass.

---

### Group 6: Implement Offloading and Single-Adapter Constraint

**Goal:** Implement the background offload timer and finalize the single-adapter constraint.

#### Task 6.1: Implement offload timer

Implement `src/offload.rs` with:

- `OffloadTimer` struct that runs as a tokio background task.
- Periodically checks all adapters in STOPPED state.
- If `now - last_activity > inactivity_timeout`, transitions the adapter to OFFLOADING and removes it via `AdapterManager`.
- Check interval: `inactivity_timeout / 10` (minimum 60 seconds).

**Files:** `rhivos/update-service/src/offload.rs`

#### Task 6.2: Wire offload timer into main

Start the `OffloadTimer` as a tokio spawn in `main.rs`, passing a shared `Arc<AdapterManager>`.

**Files:** `rhivos/update-service/src/main.rs`

#### Task 6.3: Verify single-adapter constraint end-to-end

Ensure the single-adapter constraint works through the full gRPC path. Specifically:

- Installing adapter B while adapter A is RUNNING stops A first.
- After the sequence, only B is RUNNING.
- Property test TS-07-P2 passes.

**Files:** `rhivos/update-service/src/manager_test.rs`

**Verification:** Offload tests (TS-07-8) pass. Property test TS-07-P2 passes. All tests pass.

---

### Checkpoint

**Goal:** Final validation that all requirements are met and all tests pass.

#### Checkpoint 1: Run full test suite

Run `cd rhivos && cargo test -p update-service` and confirm all tests pass.

#### Checkpoint 2: Run linter

Run `cd rhivos && cargo clippy -p update-service -- -D warnings` and confirm no warnings.

#### Checkpoint 3: Verify proto compilation

Run `cd rhivos && cargo build -p update-service` and confirm proto compilation succeeds.

#### Checkpoint 4: Review Definition of Done

Confirm all items in the design.md Definition of Done are satisfied:

1. gRPC server starts on configured port.
2. InstallAdapter flow works end-to-end (with mocked podman).
3. WatchAdapterStates streams events.
4. ListAdapters and GetAdapterStatus return correct data.
5. RemoveAdapter stops and removes containers.
6. Checksum verification rejects mismatches.
7. Single adapter constraint enforced.
8. Automatic offloading works.
9. All state transitions follow the defined state machine.
10. All tests pass.
11. Clippy clean.
12. Proto compiles.

---

## Traceability

| Task | Requirement(s) | Test(s) |
|------|----------------|---------|
| 1.2 | All | All (proto definitions) |
| 1.4 | 07-REQ-6 | TS-07-P1 |
| 1.5 | 07-REQ-1, 07-REQ-2, 07-REQ-4, 07-REQ-5, 07-REQ-7 | TS-07-1, TS-07-4, TS-07-5, TS-07-6, TS-07-7, TS-07-E1 through TS-07-E6 |
| 1.6 | 07-REQ-1, 07-REQ-3, 07-REQ-4, 07-REQ-5 | TS-07-1, TS-07-2, TS-07-5, TS-07-6, TS-07-7 |
| 1.7 | 07-REQ-8 | TS-07-8 |
| 2.2, 5.3 | 07-REQ-1, 07-REQ-3, 07-REQ-4, 07-REQ-5, 07-REQ-10 | TS-07-1, TS-07-2, TS-07-5, TS-07-6, TS-07-7 |
| 2.3 | 07-REQ-9 | (Config loading verified at startup) |
| 3.2, 3.3 | 07-REQ-2 | TS-07-3, TS-07-E1 |
| 4.2 | 07-REQ-1 | TS-07-1, TS-07-E3 |
| 5.1 | 07-REQ-6 | TS-07-P1 |
| 5.2 | 07-REQ-1, 07-REQ-4, 07-REQ-5, 07-REQ-7 | TS-07-1, TS-07-4, TS-07-5, TS-07-6, TS-07-7, TS-07-P2 |
| 6.1 | 07-REQ-8 | TS-07-8 |
| 6.3 | 07-REQ-7 | TS-07-4, TS-07-P2 |
