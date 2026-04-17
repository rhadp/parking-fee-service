# Implementation Plan: UPDATE_SERVICE

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the UPDATE_SERVICE as a Rust gRPC server in `rhivos/update-service/`. The service manages containerized PARKING_OPERATOR_ADAPTOR lifecycle: pulling OCI images, verifying checksums, installing/running/stopping/removing containers via podman CLI, and automatically offloading unused adapters. Task group 1 writes failing tests. Groups 2-4 implement core modules (config, adapter, state, podman, offload, monitor). Group 5 implements the gRPC service layer and main. Group 6 runs wiring verification.

Ordering: tests first (TDD), then pure-function modules (config, adapter ID derivation), then state management and podman executor, then async background tasks (offload, monitor), then gRPC handlers and main, then wiring verification.

## Test Commands

- Spec tests (unit): `cd rhivos && cargo test -p update-service`
- Property tests: `cd rhivos && cargo test -p update-service -- --include-ignored proptest`
- Integration tests: `cd tests/update-service && go test -v ./...`
- All Rust tests: `cd rhivos && cargo test`
- Linter: `cd rhivos && cargo clippy -p update-service -- -D warnings`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Add dependencies to update-service Cargo.toml
    - Add: tonic, prost, tokio (full), serde, serde_json, uuid, tracing, tracing-subscriber, async-trait
    - Add dev: proptest, tokio-test
    - Add tonic-build to build-dependencies
    - Vendor or reference update_service.proto and common.proto from `proto/` directory
    - Add build.rs for proto code generation
    - _Test Spec: TS-07-1 through TS-07-18_

  - [x] 1.2 Write config and adapter ID unit tests
    - Create `rhivos/update-service/src/config.rs` with test module
    - Create `rhivos/update-service/src/adapter.rs` with test module
    - `test_load_config_from_file` -- TS-07-14
    - `test_config_file_missing_defaults` -- TS-07-E13
    - `test_config_invalid_json` -- TS-07-E14
    - `test_derive_adapter_id` -- TS-07-6 (three cases)
    - _Test Spec: TS-07-6, TS-07-14, TS-07-E13, TS-07-E14_

  - [x] 1.3 Write state manager and event tests
    - Create `rhivos/update-service/src/state.rs` with test module
    - `test_create_and_get_adapter` -- TS-07-11
    - `test_list_adapters_empty` -- TS-07-E9
    - `test_list_adapters_multiple` -- TS-07-10
    - `test_get_unknown_adapter` -- TS-07-E8
    - `test_remove_adapter` -- TS-07-12
    - `test_remove_unknown_adapter` -- TS-07-E10
    - `test_state_transition_emits_event` -- TS-07-8
    - `test_no_historical_replay` -- TS-07-9
    - `test_no_subscribers_no_error` -- TS-07-E15
    - `test_subscriber_disconnect` -- TS-07-E7
    - _Test Spec: TS-07-8, TS-07-9, TS-07-10, TS-07-11, TS-07-12, TS-07-E7, TS-07-E8, TS-07-E9, TS-07-E10, TS-07-E15_

  - [x] 1.4 Write podman executor and install flow tests
    - Create `rhivos/update-service/src/podman.rs` with mock and test module
    - `test_install_calls_podman_pull` -- TS-07-2
    - `test_install_verifies_checksum` -- TS-07-3
    - `test_install_runs_with_network_host` -- TS-07-4
    - `test_install_reaches_running` -- TS-07-5
    - `test_install_response_immediate` -- TS-07-1
    - `test_install_empty_image_ref` -- TS-07-E1
    - `test_install_empty_checksum` -- TS-07-E2
    - `test_pull_failure_error_state` -- TS-07-E3
    - `test_checksum_mismatch_error` -- TS-07-E4
    - `test_run_failure_error_state` -- TS-07-E5
    - _Test Spec: TS-07-1, TS-07-2, TS-07-3, TS-07-4, TS-07-5, TS-07-E1, TS-07-E2, TS-07-E3, TS-07-E4, TS-07-E5_

  - [x] 1.5 Write single-adapter, offload, and monitor tests
    - `test_single_adapter_stops_running` -- TS-07-7
    - `test_stop_failure_install_proceeds` -- TS-07-E6
    - `test_offload_after_timeout` -- TS-07-13
    - `test_offload_failure_error` -- TS-07-E12
    - `test_container_exit_nonzero_error` -- TS-07-15
    - `test_container_exit_zero_stopped` -- TS-07-16
    - `test_podman_wait_failure_error` -- TS-07-E16
    - `test_removal_failure_internal` -- TS-07-E11
    - _Test Spec: TS-07-7, TS-07-13, TS-07-15, TS-07-16, TS-07-E6, TS-07-E11, TS-07-E12, TS-07-E16_

  - [x] 1.6 Write property tests
    - `proptest_adapter_id_determinism` -- TS-07-P1
    - `proptest_single_adapter_invariant` -- TS-07-P2
    - `proptest_state_transition_validity` -- TS-07-P3
    - `proptest_event_delivery_completeness` -- TS-07-P4
    - `proptest_checksum_verification_soundness` -- TS-07-P5
    - `proptest_offload_timing_correctness` -- TS-07-P6
    - _Test Spec: TS-07-P1 through TS-07-P6_

  - [x] 1.V Verify task group 1
    - [x] All test files compile: `cd rhivos && cargo test -p update-service --no-run`
    - [x] All spec tests FAIL (red): `cd rhivos && cargo test -p update-service 2>&1 | grep FAILED`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`

- [x] 2. Config, adapter, and data types
  - [x] 2.1 Implement adapter module
    - `derive_adapter_id(image_ref: &str) -> String`: extract last path segment, replace `:` with `-`
    - Define `AdapterState` enum with all states
    - Define `AdapterEntry` struct
    - Define `AdapterStateEvent` struct
    - _Requirements: 07-REQ-1.6_

  - [x] 2.2 Implement config module
    - `load_config(path: &str) -> Result<Config, ConfigError>`: read JSON file, deserialize
    - `default_config() -> Config`: defaults (port 50052, timeout 86400s, storage path)
    - If file not found: return default_config, log warning
    - If invalid JSON: return error
    - Support `CONFIG_PATH` env var in main
    - _Requirements: 07-REQ-7.1, 07-REQ-7.2, 07-REQ-7.E1, 07-REQ-7.E2_

  - [x] 2.V Verify task group 2
    - [x] Config and adapter tests pass: `cd rhivos && cargo test -p update-service -- config adapter`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - [x] _Test Spec: TS-07-6, TS-07-14, TS-07-E13, TS-07-E14, TS-07-P1_

- [x] 3. State manager and podman executor
  - [x] 3.1 Implement state manager
    - `StateManager::new(broadcaster)`: create with broadcast channel
    - `create_adapter`, `transition`, `get_adapter`, `list_adapters`, `remove_adapter`
    - `get_running_adapter`: returns the adapter in RUNNING state (if any)
    - `get_offload_candidates(timeout)`: returns STOPPED adapters past timeout
    - Thread-safe with `Arc<Mutex<HashMap>>` or `DashMap`
    - State transitions emit events via broadcast channel
    - _Requirements: 07-REQ-3.1, 07-REQ-3.2, 07-REQ-3.3, 07-REQ-3.4, 07-REQ-4.1, 07-REQ-4.2, 07-REQ-8.1, 07-REQ-8.2, 07-REQ-8.3_

  - [x] 3.2 Implement podman executor trait and real implementation
    - Define `PodmanExecutor` async trait (pull, inspect_digest, run, stop, rm, rmi, wait)
    - Implement `RealPodmanExecutor` using `tokio::process::Command`
    - `pull`: `podman pull <image_ref>`
    - `inspect_digest`: `podman image inspect --format '{{.Digest}}' <image_ref>`
    - `run`: `podman run -d --name <adapter_id> --network=host <image_ref>`
    - `stop`: `podman stop <adapter_id>`
    - `rm`: `podman rm <adapter_id>`
    - `rmi`: `podman rmi <image_ref>`
    - `wait`: `podman wait <adapter_id>`
    - Implement `MockPodmanExecutor` for testing
    - _Requirements: 07-REQ-1.2, 07-REQ-1.3, 07-REQ-1.4_

  - [x] 3.3 Implement install flow orchestration
    - Validate inputs (non-empty image_ref, checksum)
    - Derive adapter_id
    - Check for running adapter, stop if needed (single adapter constraint)
    - Create adapter entry, transition to DOWNLOADING
    - Spawn async task: pull, inspect, compare checksum, run
    - Handle errors at each step (transition to ERROR)
    - _Requirements: 07-REQ-1.1 through 07-REQ-1.5, 07-REQ-2.1, 07-REQ-2.2_

  - [x] 3.V Verify task group 3
    - [x] State and podman tests pass: `cd rhivos && cargo test -p update-service -- state podman install`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - [x] _Test Spec: TS-07-1 through TS-07-5, TS-07-7 through TS-07-12, TS-07-E1 through TS-07-E11, TS-07-E15, TS-07-P2 through TS-07-P5_

- [x] 4. Background tasks: offload timer and container monitor
  - [x] 4.1 Implement offload timer
    - Background tokio task that runs on a periodic interval (configurable, e.g., every 60s)
    - Checks `state_mgr.get_offload_candidates(inactivity_timeout)`
    - For each candidate: transition to OFFLOADING, call podman rm + rmi, remove from state
    - Handle cleanup failures (transition to ERROR)
    - _Requirements: 07-REQ-6.1, 07-REQ-6.2, 07-REQ-6.3, 07-REQ-6.4, 07-REQ-6.E1_

  - [x] 4.2 Implement container monitor
    - After `podman run` succeeds, spawn a task that calls `podman wait adapter_id`
    - When wait completes: read exit code
    - Exit 0: transition to STOPPED, record `stopped_at` for offload timer
    - Exit non-zero: transition to ERROR
    - Handle wait failures (transition to ERROR)
    - _Requirements: 07-REQ-9.1, 07-REQ-9.2, 07-REQ-9.E1_

  - [x] 4.V Verify task group 4
    - [x] Offload and monitor tests pass: `cd rhivos && cargo test -p update-service -- offload monitor container_exit`
    - [x] All existing tests still pass: `cd rhivos && cargo test`
    - [x] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - [x] _Test Spec: TS-07-13, TS-07-15, TS-07-16, TS-07-E12, TS-07-E16, TS-07-P6_

- [ ] 5. gRPC service and main
  - [ ] 5.1 Implement gRPC service
    - Implement `UpdateService` tonic trait:
      - `install_adapter`: validate inputs, delegate to install orchestration, return response
      - `watch_adapter_states`: subscribe to broadcast channel, stream events to client
      - `list_adapters`: delegate to state manager
      - `remove_adapter`: stop (if running) + rm + rmi via podman executor, remove from state
      - `get_adapter_status`: delegate to state manager
    - Map errors to gRPC status codes (INVALID_ARGUMENT, NOT_FOUND, INTERNAL)
    - _Requirements: 07-REQ-1.E1, 07-REQ-1.E2, 07-REQ-4.E1, 07-REQ-4.E2, 07-REQ-5.E1, 07-REQ-5.E2_

  - [ ] 5.2 Implement main
    - Read `CONFIG_PATH` env var, load config
    - Create broadcast channel, state manager, podman executor
    - Spawn offload timer task
    - Build tonic server with `UpdateService` implementation
    - Listen on `0.0.0.0:<grpc_port>`
    - Log configuration and ready message at startup
    - Handle SIGTERM/SIGINT via `tokio::signal` with 10s drain timeout
    - _Requirements: 07-REQ-7.3, 07-REQ-10.1, 07-REQ-10.2, 07-REQ-10.E1_

  - [ ] 5.V Verify task group 5
    - [ ] All unit tests pass: `cd rhivos && cargo test -p update-service`
    - [ ] Property tests pass: `cd rhivos && cargo test -p update-service -- --include-ignored proptest`
    - [ ] Binary builds: `cd rhivos && cargo build -p update-service`
    - [ ] All existing tests still pass: `cd rhivos && cargo test`
    - [ ] No linter warnings: `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - [ ] _Test Spec: TS-07-17, TS-07-18, TS-07-P1 through TS-07-P6_

- [ ] 6. Wiring verification
  - [ ] 6.1 Run full unit test suite
    - `cd rhivos && cargo test -p update-service`
    - All tests pass (no failures, no ignored tests except proptest)
  - [ ] 6.2 Run property tests
    - `cd rhivos && cargo test -p update-service -- --include-ignored proptest`
    - All property tests pass
  - [ ] 6.3 Run clippy with strict warnings
    - `cd rhivos && cargo clippy -p update-service -- -D warnings`
    - No warnings or errors
  - [ ] 6.4 Binary smoke test
    - `cd rhivos && cargo build -p update-service`
    - Start binary, verify gRPC port is listening, send SIGTERM, verify clean exit
  - [ ] 6.5 Cross-crate regression check
    - `cd rhivos && cargo test`
    - update-service: all tests pass; parking-operator-adaptor has pre-existing failures (todo!() stubs from spec 08, unrelated to this spec)
  - [ ] 6.V Verify task group 6
    - [ ] All checks from 6.1-6.5 pass
    - [ ] _Test Spec: TS-07-SMOKE-1, TS-07-SMOKE-2_ (implemented in tests/update-service/ Go module)

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
| 07-REQ-1.1 | TS-07-1 | 3.3 | podman::test_install_response_immediate |
| 07-REQ-1.2 | TS-07-2 | 3.2, 3.3 | podman::test_install_calls_podman_pull |
| 07-REQ-1.3 | TS-07-3 | 3.2, 3.3 | podman::test_install_verifies_checksum |
| 07-REQ-1.4 | TS-07-4 | 3.2, 3.3 | podman::test_install_runs_with_network_host |
| 07-REQ-1.5 | TS-07-5 | 3.3 | podman::test_install_reaches_running |
| 07-REQ-1.6 | TS-07-6 | 2.1 | adapter::test_derive_adapter_id |
| 07-REQ-1.E1 | TS-07-E1 | 5.1 | grpc::test_install_empty_image_ref |
| 07-REQ-1.E2 | TS-07-E2 | 5.1 | grpc::test_install_empty_checksum |
| 07-REQ-1.E3 | TS-07-E3 | 3.3 | podman::test_pull_failure_error_state |
| 07-REQ-1.E4 | TS-07-E4 | 3.3 | podman::test_checksum_mismatch_error |
| 07-REQ-1.E5 | TS-07-E5 | 3.3 | podman::test_run_failure_error_state |
| 07-REQ-2.1 | TS-07-7 | 3.3 | podman::test_single_adapter_stops_running |
| 07-REQ-2.2 | TS-07-7 | 3.3 | podman::test_single_adapter_stops_running |
| 07-REQ-2.E1 | TS-07-E6 | 3.3 | podman::test_stop_failure_install_proceeds |
| 07-REQ-3.1 | TS-07-8 | 5.1 | grpc::test_watch_adapter_states |
| 07-REQ-3.2 | TS-07-8 | 3.1 | state::test_state_transition_emits_event |
| 07-REQ-3.3 | TS-07-8 | 3.1 | state::test_state_transition_emits_event |
| 07-REQ-3.4 | TS-07-9 | 3.1 | state::test_no_historical_replay |
| 07-REQ-3.E1 | TS-07-E7 | 3.1 | state::test_subscriber_disconnect |
| 07-REQ-4.1 | TS-07-10 | 3.1 | state::test_list_adapters_multiple |
| 07-REQ-4.2 | TS-07-11 | 3.1 | state::test_create_and_get_adapter |
| 07-REQ-4.E1 | TS-07-E8 | 5.1 | grpc::test_get_unknown_adapter |
| 07-REQ-4.E2 | TS-07-E9 | 3.1 | state::test_list_adapters_empty |
| 07-REQ-5.1 | TS-07-12 | 5.1 | grpc::test_remove_adapter |
| 07-REQ-5.2 | TS-07-12 | 5.1 | grpc::test_remove_adapter |
| 07-REQ-5.E1 | TS-07-E10 | 5.1 | grpc::test_remove_unknown_adapter |
| 07-REQ-5.E2 | TS-07-E11 | 5.1 | grpc::test_removal_failure_internal |
| 07-REQ-6.1 | TS-07-13 | 4.1 | offload::test_offload_after_timeout |
| 07-REQ-6.2 | TS-07-13 | 4.1 | offload::test_offload_after_timeout |
| 07-REQ-6.3 | TS-07-13 | 4.1 | offload::test_offload_after_timeout |
| 07-REQ-6.4 | TS-07-13 | 4.1 | offload::test_offload_after_timeout |
| 07-REQ-6.E1 | TS-07-E12 | 4.1 | offload::test_offload_failure_error |
| 07-REQ-7.1 | TS-07-14 | 2.2 | config::test_load_config_from_file |
| 07-REQ-7.2 | TS-07-14 | 2.2 | config::test_load_config_from_file |
| 07-REQ-7.3 | TS-07-17 | 5.2 | integration::test_startup_logging |
| 07-REQ-7.E1 | TS-07-E13 | 2.2 | config::test_config_file_missing_defaults |
| 07-REQ-7.E2 | TS-07-E14 | 2.2 | config::test_config_invalid_json |
| 07-REQ-8.1 | TS-07-8 | 3.1 | state::test_state_transition_emits_event |
| 07-REQ-8.2 | TS-07-8 | 3.1 | state::test_state_transition_emits_event |
| 07-REQ-8.3 | TS-07-8 | 3.1 | state::test_state_transition_emits_event |
| 07-REQ-8.E1 | TS-07-E15 | 3.1 | state::test_no_subscribers_no_error |
| 07-REQ-9.1 | TS-07-15 | 4.2 | monitor::test_container_exit_nonzero_error |
| 07-REQ-9.2 | TS-07-16 | 4.2 | monitor::test_container_exit_zero_stopped |
| 07-REQ-9.E1 | TS-07-E16 | 4.2 | monitor::test_podman_wait_failure_error |
| 07-REQ-10.1 | TS-07-17 | 5.2 | integration::test_startup_logging |
| 07-REQ-10.2 | TS-07-18 | 5.2 | integration::test_graceful_shutdown |
| 07-REQ-10.E1 | TS-07-18 | 5.2 | integration::test_graceful_shutdown |
| Property 1 | TS-07-P1 | 2.1 | adapter::proptest_adapter_id_determinism |
| Property 2 | TS-07-P2 | 3.3 | podman::proptest_single_adapter_invariant |
| Property 3 | TS-07-P3 | 3.1 | state::proptest_state_transition_validity |
| Property 4 | TS-07-P4 | 3.1 | state::proptest_event_delivery_completeness |
| Property 5 | TS-07-P5 | 3.3 | podman::proptest_checksum_verification_soundness |
| Property 6 | TS-07-P6 | 4.1 | offload::proptest_offload_timing_correctness |

## Notes

- The UPDATE_SERVICE uses a `PodmanExecutor` trait to abstract container operations. Unit tests use `MockPodmanExecutor`; the real implementation shells out to podman CLI via `tokio::process::Command`.
- State is in-memory only (no persistence). On service restart, all adapter state is lost. This is a demo simplification per C6.
- The broadcast channel (`tokio::sync::broadcast`) handles fan-out to multiple WatchAdapterStates subscribers. Lagged receivers are handled gracefully.
- Registry authentication is out of scope (C5). The host is assumed to have podman configured with appropriate credentials.
- Integration tests (TS-07-17, TS-07-18, TS-07-SMOKE-1, TS-07-SMOKE-2) require starting the binary as a subprocess. They are in the Go test harness at `tests/update-service/`.
- The offload timer checks for candidates periodically (e.g., every 60 seconds). For unit tests, a short inactivity timeout (1-2 seconds) is used to avoid long test durations.
