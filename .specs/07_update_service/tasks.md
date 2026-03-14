# Implementation Plan: UPDATE_SERVICE

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the UPDATE_SERVICE as a Rust gRPC service in `rhivos/update-service/`. The service manages containerized adapter lifecycle via podman CLI. Task group 1 writes failing tests. Groups 2-3 implement pure-function modules (model, config, state). Group 4 implements the container runtime trait and mock. Group 5 implements gRPC service and main. Group 6 runs integration tests.

Ordering: tests first, then data types and config, then state manager, then container runtime trait, then gRPC handlers and main, then integration validation.

## Test Commands

- Spec tests (unit): `cd rhivos && cargo test -p update-service`
- Spec tests (integration): `cd tests/update-service && go test -v ./...`
- Property tests: `cd rhivos && cargo test -p update-service -- --include-ignored proptest`
- All Rust tests: `cd rhivos && cargo test`
- Linter: `cd rhivos && cargo clippy -p update-service -- -D warnings`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Add dependencies to update-service Cargo.toml
    - Add: tonic, prost, tokio, serde, serde_json, uuid, tracing, tracing-subscriber, async-trait, proptest (dev)
    - Vendor update_service.proto and common.proto into `rhivos/update-service/proto/`
    - Add tonic-build to build.rs for proto code generation
    - _Test Spec: TS-07-1 through TS-07-23_

  - [x] 1.2 Write config and model tests
    - Create `rhivos/update-service/src/config.rs` with test module
    - `test_load_config_from_file` — TS-07-19
    - `test_config_fields` — TS-07-20
    - `test_config_defaults` — TS-07-21
    - `test_config_file_missing` — TS-07-E9
    - `test_config_invalid_json` — TS-07-E10
    - Create `rhivos/update-service/src/model.rs` with test module
    - `test_adapter_id_derivation` — TS-07-5
    - _Test Spec: TS-07-5, TS-07-19, TS-07-20, TS-07-21, TS-07-E9, TS-07-E10_

  - [x] 1.3 Write state manager tests
    - Create `rhivos/update-service/src/state.rs` with test module
    - `test_install_happy_path` — TS-07-1
    - `test_state_transitions_during_install` — TS-07-2
    - `test_checksum_verification` — TS-07-3
    - `test_container_host_networking` — TS-07-4
    - `test_single_adapter_stops_running` — TS-07-6
    - `test_previous_adapter_stopped_state` — TS-07-7
    - _Test Spec: TS-07-1 through TS-07-4, TS-07-6, TS-07-7_

  - [x] 1.4 Write watch, list, remove, and offload tests
    - `test_watch_state_stream` — TS-07-8
    - `test_multiple_watch_subscribers` — TS-07-9
    - `test_state_event_fields` — TS-07-10
    - `test_list_adapters` — TS-07-11
    - `test_get_adapter_status` — TS-07-12
    - `test_remove_adapter` — TS-07-13
    - `test_remove_adapter_transitions` — TS-07-14
    - `test_remove_adapter_events` — TS-07-15
    - `test_automatic_offloading` — TS-07-16
    - `test_configurable_inactivity` — TS-07-17
    - `test_offloading_events` — TS-07-18
    - _Test Spec: TS-07-8 through TS-07-18_

  - [x] 1.5 Write edge case and property tests
    - `test_empty_image_ref` — TS-07-E1
    - `test_pull_failure` — TS-07-E2
    - `test_checksum_mismatch` — TS-07-E3
    - `test_container_start_failure` — TS-07-E4
    - `test_stop_running_fails` — TS-07-E5
    - `test_get_unknown_adapter` — TS-07-E6
    - `test_remove_unknown_adapter` — TS-07-E7
    - `test_container_removal_failure` — TS-07-E8
    - `proptest_state_machine_validity` — TS-07-P1
    - `proptest_single_adapter_constraint` — TS-07-P2
    - `proptest_checksum_gate` — TS-07-P3
    - `proptest_state_event_broadcasting` — TS-07-P4
    - `proptest_adapter_id_derivation` — TS-07-P5
    - `proptest_inactivity_offloading` — TS-07-P6
    - `proptest_config_defaults` — TS-07-P7
    - _Test Spec: TS-07-E1 through TS-07-E8, TS-07-P1 through TS-07-P7_

  - [x] 1.V Verify task group 1
    - [x] All test files compile: `cd rhivos && cargo test -p update-service --no-run`
    - [x] All unit tests FAIL (red): `cd rhivos && cargo test -p update-service 2>&1 | grep FAILED`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`

- [x] 2. Model and config modules
  - [x] 2.1 Implement model module
    - Define `AdapterState` enum with all 7 states
    - Implement `derive_adapter_id(image_ref: &str) -> String`: extract last path segment + tag
    - Implement `generate_job_id() -> String`: UUID v4
    - Define `AdapterInfo` struct with all fields
    - Define `AdapterStateEvent` struct
    - _Requirements: 07-REQ-1.5_

  - [x] 2.2 Implement config module
    - Define `Config` struct with serde Deserialize
    - `load_config(path: &str) -> Result<Config, ConfigError>`: read JSON, apply defaults
    - `default_config() -> Config`: port 50052, timeout 86400, storage path `/var/lib/containers/adapters/`
    - If file not found: return default_config, log warning
    - If invalid JSON: return error
    - _Requirements: 07-REQ-7.1, 07-REQ-7.2, 07-REQ-7.3, 07-REQ-7.E1, 07-REQ-7.E2_

  - [x] 2.V Verify task group 2
    - [x] Config and model tests pass: `cd rhivos && cargo test -p update-service -- config model`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - [x] _Test Spec: TS-07-5, TS-07-19, TS-07-20, TS-07-21, TS-07-E9, TS-07-E10, TS-07-P5, TS-07-P7_

- [x] 3. State manager module
  - [x] 3.1 Implement state manager
    - `StateManager` struct with `Arc<Mutex<HashMap<String, AdapterInfo>>>` and `broadcast::Sender<AdapterStateEvent>`
    - `new() -> Self`: create broadcast channel
    - `create_adapter(adapter_id, image_ref, checksum)`: add to map with DOWNLOADING state
    - `transition(adapter_id, new_state)`: validate transition, update state, emit event
    - `get(adapter_id)`, `list()`, `remove(adapter_id)`
    - `get_running_adapter()`: find adapter in RUNNING state (for single constraint)
    - `get_stopped_expired(timeout_secs)`: find STOPPED adapters past threshold
    - `subscribe()`: return broadcast receiver
    - _Requirements: 07-REQ-1.2, 07-REQ-1.4, 07-REQ-3.1, 07-REQ-3.2, 07-REQ-3.3, 07-REQ-4.1, 07-REQ-4.2_

  - [x] 3.2 Implement state transition validation
    - Define valid transitions as a lookup table
    - Reject invalid transitions with descriptive error
    - Set `stopped_at` timestamp when transitioning to STOPPED
    - _Requirements: 07-REQ-1.2, 07-REQ-5.2_

  - [x] 3.V Verify task group 3
    - [x] State manager tests pass: `cd rhivos && cargo test -p update-service -- state`
    - [x] Property tests pass: `cd rhivos && cargo test -p update-service -- proptest`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - [x] _Test Spec: TS-07-8 through TS-07-12, TS-07-14, TS-07-15, TS-07-16, TS-07-18, TS-07-E6, TS-07-P1, TS-07-P2, TS-07-P4, TS-07-P6_

- [x] 4. Container runtime trait and service logic
  - [x] 4.1 Define ContainerRuntime trait
    - `async fn pull(image_ref)`, `async fn inspect_digest(image_ref)`, `async fn run(image_ref, adapter_id)`, `async fn stop(container_id)`, `async fn remove(container_id)`, `async fn remove_image(image_ref)`
    - Implement `PodmanRuntime` struct: each method calls podman CLI via `tokio::process::Command`
    - `run` uses `--network=host` and `--name={adapter_id}`
    - _Requirements: 07-REQ-1.1, 07-REQ-1.3, 07-REQ-1.4_

  - [x] 4.2 Create MockContainerRuntime
    - Configurable success/failure for each operation
    - Records calls for assertion
    - Configurable digest return value
    - _Test Spec: TS-07-1, TS-07-3, TS-07-4, TS-07-6, TS-07-13, TS-07-E2, TS-07-E3, TS-07-E4, TS-07-E5, TS-07-E8_

  - [x] 4.3 Implement install logic
    - Orchestrate: validate inputs → check single adapter constraint → stop running if needed → create adapter → pull → verify checksum → run
    - Transition through DOWNLOADING → INSTALLING → RUNNING (or ERROR)
    - On checksum mismatch: remove image, transition to ERROR
    - _Requirements: 07-REQ-1.1, 07-REQ-1.2, 07-REQ-1.3, 07-REQ-2.1, 07-REQ-2.2_

  - [x] 4.4 Implement remove and offload logic
    - Remove: stop (if running) → transition OFFLOADING → remove container → remove image → remove from state
    - Offload timer: periodic task checking `get_stopped_expired()`, triggers removal
    - _Requirements: 07-REQ-5.1, 07-REQ-5.2, 07-REQ-5.3, 07-REQ-6.1, 07-REQ-6.3_

  - [x] 4.V Verify task group 4
    - [x] All unit tests pass: `cd rhivos && cargo test -p update-service`
    - [x] All property tests pass: `cd rhivos && cargo test -p update-service -- proptest`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - [x] _Test Spec: TS-07-1 through TS-07-4, TS-07-6, TS-07-7, TS-07-13, TS-07-14, TS-07-15, TS-07-16, TS-07-17, TS-07-18, TS-07-E1 through TS-07-E5, TS-07-E7, TS-07-E8, TS-07-P2, TS-07-P3_

- [x] 5. gRPC server and main
  - [x] 5.1 Implement gRPC service
    - Implement `UpdateService` tonic trait for all 5 RPCs
    - `install_adapter`: delegate to install logic
    - `watch_adapter_states`: create subscriber, stream events
    - `list_adapters`: delegate to state manager
    - `remove_adapter`: delegate to remove logic
    - `get_adapter_status`: delegate to state manager
    - Map errors to appropriate gRPC status codes
    - _Requirements: 07-REQ-1.1, 07-REQ-3.1, 07-REQ-4.1, 07-REQ-4.2, 07-REQ-5.1_

  - [x] 5.2 Implement main
    - Read `CONFIG_PATH` env var, load config
    - Create StateManager, PodmanRuntime, gRPC service
    - Start offload timer as tokio task
    - Start tonic gRPC server on configured port
    - Log version, port, registry URL at startup
    - Handle SIGTERM/SIGINT: stop running adapters, shutdown gRPC server
    - _Requirements: 07-REQ-7.1, 07-REQ-8.1, 07-REQ-8.2_

  - [x] 5.V Verify task group 5
    - [x] Binary compiles: `cd rhivos && cargo build -p update-service`
    - [x] All unit tests still pass: `cd rhivos && cargo test -p update-service`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`

- [ ] 6. Integration test validation
  - [ ] 6.1 Create integration test module
    - Create `tests/update-service/` Go module
    - Shared helpers: start/stop service, gRPC client helpers
    - Add `go.work` entry for `./tests/update-service`
    - _Test Spec: TS-07-22, TS-07-23_

  - [ ] 6.2 Write and run integration tests
    - `TestStartupLogging` — TS-07-22
    - `TestGracefulShutdown` — TS-07-23
    - `TestInstallAdapterGRPC` — end-to-end with mock registry (optional)
    - `TestListAdaptersGRPC` — end-to-end query
    - _Test Spec: TS-07-22, TS-07-23_

  - [ ] 6.V Verify task group 6
    - [ ] All integration tests pass: `cd tests/update-service && go test -v ./...`
    - [ ] All unit tests still pass: `cd rhivos && cargo test -p update-service`
    - [ ] All existing tests still pass: `make test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - [ ] All requirements 07-REQ-1 through 07-REQ-8 acceptance criteria met

- [ ] 7. Checkpoint - All Tests Green
  - All unit, property, and integration tests pass
  - Binary starts, serves gRPC requests, manages containers, shuts down cleanly
  - Ask the user if questions arise

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 07-REQ-1.1 | TS-07-1 | 4.3 | update-service::state::test_install_happy_path |
| 07-REQ-1.2 | TS-07-2 | 4.3 | update-service::state::test_state_transitions_during_install |
| 07-REQ-1.3 | TS-07-3 | 4.3 | update-service::state::test_checksum_verification |
| 07-REQ-1.4 | TS-07-4 | 4.1 | update-service::state::test_container_host_networking |
| 07-REQ-1.5 | TS-07-5 | 2.1 | update-service::model::test_adapter_id_derivation |
| 07-REQ-1.E1 | TS-07-E1 | 4.3 | update-service::test_empty_image_ref |
| 07-REQ-1.E2 | TS-07-E2 | 4.3 | update-service::test_pull_failure |
| 07-REQ-1.E3 | TS-07-E3 | 4.3 | update-service::test_checksum_mismatch |
| 07-REQ-1.E4 | TS-07-E4 | 4.3 | update-service::test_container_start_failure |
| 07-REQ-2.1 | TS-07-6 | 4.3 | update-service::state::test_single_adapter_stops_running |
| 07-REQ-2.2 | TS-07-7 | 4.3 | update-service::state::test_previous_adapter_stopped_state |
| 07-REQ-2.E1 | TS-07-E5 | 4.3 | update-service::test_stop_running_fails |
| 07-REQ-3.1 | TS-07-8 | 3.1 | update-service::state::test_watch_state_stream |
| 07-REQ-3.2 | TS-07-9 | 3.1 | update-service::state::test_multiple_watch_subscribers |
| 07-REQ-3.3 | TS-07-10 | 3.1 | update-service::state::test_state_event_fields |
| 07-REQ-4.1 | TS-07-11 | 3.1 | update-service::state::test_list_adapters |
| 07-REQ-4.2 | TS-07-12 | 3.1 | update-service::state::test_get_adapter_status |
| 07-REQ-4.E1 | TS-07-E6 | 3.1 | update-service::test_get_unknown_adapter |
| 07-REQ-5.1 | TS-07-13 | 4.4 | update-service::test_remove_adapter |
| 07-REQ-5.2 | TS-07-14 | 4.4 | update-service::test_remove_adapter_transitions |
| 07-REQ-5.3 | TS-07-15 | 4.4 | update-service::test_remove_adapter_events |
| 07-REQ-5.E1 | TS-07-E7 | 4.4 | update-service::test_remove_unknown_adapter |
| 07-REQ-5.E2 | TS-07-E8 | 4.4 | update-service::test_container_removal_failure |
| 07-REQ-6.1 | TS-07-16 | 4.4 | update-service::test_automatic_offloading |
| 07-REQ-6.2 | TS-07-17 | 2.2 | update-service::config::test_configurable_inactivity |
| 07-REQ-6.3 | TS-07-18 | 4.4 | update-service::test_offloading_events |
| 07-REQ-7.1 | TS-07-19 | 2.2 | update-service::config::test_load_config_from_file |
| 07-REQ-7.2 | TS-07-20 | 2.2 | update-service::config::test_config_fields |
| 07-REQ-7.3 | TS-07-21 | 2.2 | update-service::config::test_config_defaults |
| 07-REQ-7.E1 | TS-07-E9 | 2.2 | update-service::config::test_config_file_missing |
| 07-REQ-7.E2 | TS-07-E10 | 2.2 | update-service::config::test_config_invalid_json |
| 07-REQ-8.1 | TS-07-22 | 5.2 | tests/update-service::TestStartupLogging |
| 07-REQ-8.2 | TS-07-23 | 5.2 | tests/update-service::TestGracefulShutdown |
| Property 1 | TS-07-P1 | 3.2 | update-service::proptest_state_machine_validity |
| Property 2 | TS-07-P2 | 4.3 | update-service::proptest_single_adapter_constraint |
| Property 3 | TS-07-P3 | 4.3 | update-service::proptest_checksum_gate |
| Property 4 | TS-07-P4 | 3.1 | update-service::proptest_state_event_broadcasting |
| Property 5 | TS-07-P5 | 2.1 | update-service::proptest_adapter_id_derivation |
| Property 6 | TS-07-P6 | 4.4 | update-service::proptest_inactivity_offloading |
| Property 7 | TS-07-P7 | 2.2 | update-service::proptest_config_defaults |

## Notes

- The UPDATE_SERVICE shares patterns with other Rust services (spec 03, 04): tonic gRPC, tokio async, proptest for property tests. However, this service manages container lifecycle via podman CLI, which is unique.
- The ContainerRuntime trait enables unit testing without a real podman installation. Integration tests requiring podman skip when it is unavailable.
- Proto files are vendored per-crate into `rhivos/update-service/proto/`. `tonic-build` in `build.rs` generates the Rust code.
- The state manager uses `tokio::sync::broadcast` for the WatchAdapterStates streaming pattern. This allows multiple subscribers and handles slow consumers by dropping old events.
- The offload timer runs as a tokio background task checking `get_stopped_expired()` periodically (e.g., every 60 seconds).
- Integration tests in `tests/update-service/` require the compiled binary. They test gRPC connectivity, startup logging, and graceful shutdown. Full adapter lifecycle tests with podman are optional and skip when podman is not available.
