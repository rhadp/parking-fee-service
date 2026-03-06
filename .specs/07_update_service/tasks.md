# Implementation Plan: UPDATE_SERVICE (Spec 07)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the UPDATE_SERVICE adapter lifecycle manager. It provides a gRPC API for installing, monitoring, and removing OCI-based adapter containers via podman, with a state machine governing adapter lifecycle transitions. Task group 1 writes all failing spec tests. Groups 2-6 implement functionality. Group 7 is the final checkpoint.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Rust workspace structure, `rhivos/Cargo.toml` workspace includes `update-service` |

## Test Commands

- Unit tests: `cd rhivos && cargo test -p update-service`
- Lint: `cd rhivos && cargo clippy -p update-service`
- Build: `cd rhivos && cargo build -p update-service`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Initialize Cargo crate
    - Create `rhivos/update-service/Cargo.toml` with dependencies: tonic, prost, tokio (full), uuid, toml, tracing, tracing-subscriber
    - Dev dependencies: mockall, tonic (features: transport), tokio-test
    - Add crate to `rhivos/Cargo.toml` workspace members

  - [x] 1.2 Create proto file
    - Create `proto/update_service/v1/update_service.proto` with the full service definition as specified in design.md
    - Create `rhivos/update-service/build.rs` for tonic-build compilation

  - [x] 1.3 Create type stubs and trait definitions
    - Create minimal stub files so that tests compile:
    - `src/main.rs` -- minimal main with module declarations
    - `src/state.rs` -- `AdapterState` enum stub with `UNKNOWN` variant, `StateMachine` trait stub
    - `src/config.rs` -- `Config` struct stub
    - `src/oci.rs` -- `OciPuller` trait stub
    - `src/container.rs` -- `ContainerRuntime` trait stub
    - `src/manager.rs` -- `AdapterManager` struct stub
    - `src/grpc.rs` -- gRPC service struct stub
    - `src/offload.rs` -- offload timer stub

  - [x] 1.4 Write state machine tests
    - Create `rhivos/update-service/src/state_test.rs` with test functions
    - `test_valid_transitions` -- all valid transitions from 07-REQ-6.1
    - `test_invalid_transitions` -- invalid transitions are rejected
    - `test_all_states_represented` -- all 7 states exist in the enum
    - Tests should compile but fail because the state machine is not yet implemented
    - _Test Spec: TS-07-P1_

  - [x] 1.5 Write manager tests
    - Create `rhivos/update-service/src/manager_test.rs` with test functions
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
    - Tests use mockall-generated mocks for `OciPuller` and `ContainerRuntime` traits; tests should compile but fail
    - _Test Spec: TS-07-1, TS-07-4, TS-07-5, TS-07-6, TS-07-7, TS-07-E1 through TS-07-E6_

  - [x] 1.6 Write gRPC integration tests
    - Create `rhivos/update-service/src/grpc_test.rs` with test functions
    - `test_grpc_install_and_watch` (TS-07-1, TS-07-2)
    - `test_grpc_list_adapters` (TS-07-5)
    - `test_grpc_get_status` (TS-07-6)
    - `test_grpc_remove_adapter` (TS-07-7)
    - Tests start an in-process tonic server with mocked dependencies and use a tonic client; tests should compile but fail
    - _Test Spec: TS-07-1, TS-07-2, TS-07-5, TS-07-6, TS-07-7_

  - [x] 1.V Verify task group 1
    - [x] `cd rhivos && cargo test -p update-service` compiles but all tests fail (or are marked `#[ignore]`)
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p update-service`

- [x] 2. Implement gRPC server and proto definitions
  - [x] 2.1 Finalize proto compilation
    - Ensure `build.rs` compiles the proto file and generated code is importable via `tonic::include_proto!("update_service.v1")`

  - [x] 2.2 Implement gRPC service skeleton
    - Implement `src/grpc.rs` with a struct `UpdateServiceImpl` that implements the tonic-generated `UpdateService` trait
    - Each method returns `Err(Status::unimplemented("..."))` initially
    - Wire the service into `main.rs` with tonic `Server::builder`

  - [x] 2.3 Implement configuration loading
    - Implement `src/config.rs` with `Config` struct: `grpc_port`, `registry_base_url`, `inactivity_timeout_secs`, `storage_path`
    - `Config::load(path: Option<&str>)` function that loads from TOML file or defaults
    - Environment variable overrides with `UPDATE_SERVICE_` prefix
    - _Requirements: 07-REQ-9_

  - [x] 2.V Verify task group 2
    - [x] `cd rhivos && cargo build -p update-service` succeeds
    - [x] Server starts and responds with UNIMPLEMENTED to all RPCs
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p update-service`

- [x] 3. Implement OCI pull and checksum verification
  - [x] 3.1 Define OciPuller trait
    - Define the `OciPuller` trait in `src/oci.rs` with `pull_image` and `remove_image` methods
    - Add `#[automock]` attribute for mockall

  - [x] 3.2 Implement PodmanOciPuller
    - `pull_image`: Runs `podman pull <image_ref>`, then `podman inspect` to extract the digest
    - `remove_image`: Runs `podman rmi <image_ref>`
    - Maps podman errors to `OciError` variants
    - _Requirements: 07-REQ-2_

  - [x] 3.3 Implement checksum verification
    - Add `verify_checksum(digest, expected) -> Result<(), ChecksumError>` function
    - Computes SHA-256, formats as `sha256:<hex>`, compares with expected
    - Returns `Ok(())` on match, `Err(ChecksumError::Mismatch{...})` otherwise
    - _Requirements: 07-REQ-2_

  - [x] 3.V Verify task group 3
    - [x] OCI-related tests (TS-07-3, TS-07-E1) pass with mocked podman
    - [x] All existing tests still pass: `cd rhivos && cargo test -p update-service`
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p update-service`

- [x] 4. Implement container lifecycle management (podman)
  - [x] 4.1 Define ContainerRuntime trait
    - Define the `ContainerRuntime` trait in `src/container.rs` with `run`, `stop`, `remove`, `status` methods
    - Add `#[automock]` attribute

  - [x] 4.2 Implement PodmanRuntime
    - `run`: Executes `podman run -d --name <name> <image_ref>`
    - `stop`: Executes `podman stop <name>`
    - `remove`: Executes `podman rm -f <name>`
    - `status`: Executes `podman inspect <name> --format '{{.State.Status}}'`, parses output
    - All commands via `tokio::process::Command`
    - _Requirements: 07-REQ-1_

  - [x] 4.V Verify task group 4
    - [x] Container runtime tests pass with mocked commands; TS-07-E3 passes
    - [x] All existing tests still pass: `cd rhivos && cargo test -p update-service`
    - [x] No linter warnings introduced: `cd rhivos && cargo clippy -p update-service`

- [ ] 5. Implement state machine and streaming
  - [ ] 5.1 Implement state machine
    - `AdapterState` enum with all 7 states
    - `AdapterState::can_transition_to(&self, target: AdapterState) -> bool` method encoding valid transitions from 07-REQ-6.1
    - `AdapterRecord` struct holding `adapter_id`, `image_ref`, `state`, `last_activity` timestamp
    - _Requirements: 07-REQ-6_

  - [ ] 5.2 Implement AdapterManager
    - `AdapterManager` struct holding a `HashMap<String, AdapterRecord>`, a `broadcast::Sender<AdapterStateEvent>`, and references to `OciPuller` and `ContainerRuntime`
    - `install_adapter(image_ref, checksum)` -- full flow: check single-adapter constraint, pull, verify, install, start
    - `remove_adapter(adapter_id)`, `list_adapters()`, `get_adapter_status(adapter_id)`
    - `subscribe_state_events()` returning a `broadcast::Receiver`
    - Private `transition_state(adapter_id, new_state)` that validates transitions and emits events
    - _Requirements: 07-REQ-1, 07-REQ-4, 07-REQ-5, 07-REQ-7_

  - [ ] 5.3 Wire gRPC methods to AdapterManager
    - Update `src/grpc.rs` to delegate all RPC calls to `AdapterManager`
    - `InstallAdapter` -> `manager.install_adapter()`
    - `WatchAdapterStates` -> `manager.subscribe_state_events()` wrapped in a tonic streaming response
    - `ListAdapters` -> `manager.list_adapters()`
    - `RemoveAdapter` -> `manager.remove_adapter()`
    - `GetAdapterStatus` -> `manager.get_adapter_status()`
    - _Requirements: 07-REQ-1, 07-REQ-3, 07-REQ-4, 07-REQ-5, 07-REQ-10_

  - [ ] 5.V Verify task group 5
    - [ ] State machine tests (TS-07-P1) pass
    - [ ] Manager tests (TS-07-1, TS-07-4, TS-07-5, TS-07-6, TS-07-7, TS-07-E1 through TS-07-E6) pass
    - [ ] Streaming tests (TS-07-2) pass
    - [ ] gRPC integration tests pass
    - [ ] All existing tests still pass: `cd rhivos && cargo test -p update-service`
    - [ ] No linter warnings introduced: `cd rhivos && cargo clippy -p update-service`

- [ ] 6. Implement offloading and single-adapter constraint
  - [ ] 6.1 Implement offload timer
    - `OffloadTimer` struct that runs as a tokio background task
    - Periodically checks all adapters in STOPPED state
    - If `now - last_activity > inactivity_timeout`, transitions the adapter to OFFLOADING and removes it via `AdapterManager`
    - Check interval: `inactivity_timeout / 10` (minimum 60 seconds)
    - _Requirements: 07-REQ-8_

  - [ ] 6.2 Wire offload timer into main
    - Start the `OffloadTimer` as a tokio spawn in `main.rs`, passing a shared `Arc<AdapterManager>`

  - [ ] 6.3 Verify single-adapter constraint end-to-end
    - Ensure the single-adapter constraint works through the full gRPC path
    - Installing adapter B while adapter A is RUNNING stops A first
    - After the sequence, only B is RUNNING
    - _Requirements: 07-REQ-7_

  - [ ] 6.V Verify task group 6
    - [ ] Offload tests (TS-07-8) pass
    - [ ] Property test TS-07-P2 passes
    - [ ] All existing tests still pass: `cd rhivos && cargo test -p update-service`
    - [ ] No linter warnings introduced: `cd rhivos && cargo clippy -p update-service`

- [ ] 7. Checkpoint
  - [ ] 7.1 Run full test suite
    - `cd rhivos && cargo test -p update-service` -- confirm all tests pass

  - [ ] 7.2 Run linter
    - `cd rhivos && cargo clippy -p update-service -- -D warnings` -- confirm no warnings

  - [ ] 7.3 Verify proto compilation
    - `cd rhivos && cargo build -p update-service` -- confirm proto compilation succeeds

  - [ ] 7.4 Review Definition of Done
    - gRPC server starts on configured port
    - InstallAdapter flow works end-to-end (with mocked podman)
    - WatchAdapterStates streams events
    - ListAdapters and GetAdapterStatus return correct data
    - RemoveAdapter stops and removes containers
    - Checksum verification rejects mismatches
    - Single adapter constraint enforced
    - Automatic offloading works
    - All state transitions follow the defined state machine
    - All tests pass, clippy clean, proto compiles

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 07-REQ-1 | TS-07-1, TS-07-E3 | 4.2, 5.2, 5.3 | Manager + gRPC tests |
| 07-REQ-2 | TS-07-3, TS-07-E1 | 3.2, 3.3 | OCI + checksum tests |
| 07-REQ-3 | TS-07-2 | 5.3 | gRPC streaming tests |
| 07-REQ-4 | TS-07-5 | 5.2, 5.3 | Manager + gRPC tests |
| 07-REQ-5 | TS-07-6, TS-07-E4 | 5.2, 5.3 | Manager + gRPC tests |
| 07-REQ-6 | TS-07-P1 | 5.1 | State machine tests |
| 07-REQ-7 | TS-07-4, TS-07-P2 | 5.2, 6.3 | Manager + property tests |
| 07-REQ-8 | TS-07-8 | 6.1 | Offload tests |
| 07-REQ-9 | -- | 2.3 | Config loading verified at startup |
| 07-REQ-10 | TS-07-7, TS-07-E5 | 5.2, 5.3 | Manager + gRPC tests |
