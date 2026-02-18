# Implementation Plan: LOCKING_SERVICE + DATA_BROKER + Mock Sensors

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
- Kuksa Databroker must be configured and running before LOCKING_SERVICE can be tested
- The safety validation module (safety.rs) is pure and testable without Kuksa
- Integration tests require `make infra-up` to start Kuksa
-->

## Overview

This plan implements the RHIVOS safety-partition core in dependency order:

1. Kuksa VSS overlay and infrastructure config (DATA_BROKER must have custom
   signals before anything can use them).
2. Kuksa proto vendoring and client library (shared dependency for
   LOCKING_SERVICE and mock-sensors).
3. Safety validation module (pure logic, testable without Kuksa).
4. LOCKING_SERVICE command handler (connects logic to Kuksa).
5. Mock sensors real implementation (testing tool).
6. Integration tests (end-to-end verification).

## Test Commands

- Unit tests: `cd rhivos && cargo test --workspace`
- Unit tests (locking-service only): `cd rhivos && cargo test -p locking-service`
- Unit tests (safety module): `cd rhivos && cargo test -p locking-service -- safety`
- Integration tests: `cd rhivos && cargo test --workspace -- --ignored`
  (integration tests are `#[ignore]` by default, require `make infra-up`)
- All tests: `make test`
- Rust linter: `cd rhivos && cargo clippy --workspace -- -D warnings`
- All linters: `make lint`
- Build all: `make build`
- Start infrastructure: `make infra-up`

## Tasks

- [x] 1. Kuksa VSS Configuration
  - [x] 1.1 Create VSS overlay file
    - Create `infra/config/kuksa/vss_overlay.json` with custom signal
      definitions: `Vehicle.Command.Door.Lock` (bool actuator),
      `Vehicle.Command.Door.LockResult` (string sensor),
      `Vehicle.Parking.SessionActive` (bool sensor)
    - Follow Kuksa's overlay JSON schema
    - _Requirements: 02-REQ-1.1, 02-REQ-1.2, 02-REQ-1.3, 02-REQ-1.5_

  - [x] 1.2 Update infrastructure compose file
    - Update `infra/compose.yaml` to mount `vss_overlay.json` into the Kuksa
      container and pass it via command-line flag
    - Verify the correct Kuksa CLI flag for VSS overlay loading (may be
      `--vss`, `--overlays`, or similar — check Kuksa documentation)
    - _Requirements: 02-REQ-1.4_

  - [x] 1.3 Verify Kuksa starts with custom signals
    - Run `make infra-up`
    - Use `grpcurl` or a test script to verify that custom signals are
      accessible (Get/Set on `Vehicle.Command.Door.Lock` succeeds)
    - **Property 6: Signal Availability**
    - _Requirements: 02-REQ-1.4_

  - [x] 1.V Verify task group 1
    - [x] `make infra-up` starts Kuksa with overlay loaded
    - [x] Custom signals are readable and writable via gRPC
    - [x] `make infra-down` cleans up
    - [x] Requirements 02-REQ-1.1–1.5 acceptance criteria met

- [x] 2. Kuksa Proto Integration
  - [x] 2.1 Vendor Kuksa proto files
    - Download `kuksa.val.v2` proto files from the Eclipse Kuksa Databroker
      repository
    - Place in `proto/vendor/kuksa/val/v2/` (preserving package paths)
    - Include any transitive proto dependencies (e.g., `google/protobuf/`
      types if not already available)
    - _Requirements: 02-REQ-2.1 (prerequisite)_

  - [x] 2.2 Extend parking-proto build.rs
    - Add Kuksa proto files to the `tonic_build` compilation list
    - Ensure generated Kuksa types are re-exported from `parking-proto`
    - Verify `cargo build -p parking-proto` succeeds with both parking and
      Kuksa protos
    - _Requirements: 02-REQ-2.1 (prerequisite)_

  - [x] 2.3 Create Kuksa client helper module
    - Add a `kuksa_client` module to `parking-proto` (or create a separate
      `rhivos/kuksa-client/` workspace crate)
    - Implement: `connect`, `get_bool`, `get_f32`, `get_f64`, `get_string`,
      `set_bool`, `set_f32`, `set_f64`, `set_string`, `subscribe_bool`
    - See design document for the `KuksaClient` interface
    - _Requirements: 02-REQ-2.1, 02-REQ-6.5_

  - [x] 2.4 Write Kuksa client unit tests
    - Test client construction and error handling (connection failure → error)
    - Integration test (`#[ignore]`): connect to real Kuksa, set a value,
      get it back, verify match
    - **Property 5: Mock Sensor Signal Accuracy** (partial)
    - _Requirements: 02-REQ-2.E1_

  - [x] 2.V Verify task group 2
    - [x] `cargo build -p parking-proto` compiles with Kuksa bindings
    - [x] `cargo test -p parking-proto` passes unit tests
    - [x] Integration test passes with `make infra-up` running
    - [x] No clippy warnings

- [ ] 3. Checkpoint — Kuksa Infrastructure Ready
  - Kuksa starts with custom signals, Rust client library compiles and
    connects successfully
  - Commit and verify clean state

- [ ] 4. LOCKING_SERVICE Safety Validation
  - [ ] 4.1 Create `safety.rs` module
    - Implement `LockResult` enum: `Success`, `RejectedSpeed`,
      `RejectedDoorOpen`
    - Implement `validate_lock(command_is_lock, speed_kmh, door_is_open,
      max_speed_kmh) -> LockResult` as a pure function
    - Implement `Display` trait for `LockResult` returning the string values
      (`"SUCCESS"`, `"REJECTED_SPEED"`, `"REJECTED_DOOR_OPEN"`)
    - _Requirements: 02-REQ-3.1, 02-REQ-3.2, 02-REQ-3.3, 02-REQ-3.4_

  - [ ] 4.2 Create `config.rs` module
    - Define `Config` struct with `databroker_addr` and `max_speed_kmh` fields
    - Parse from CLI args (`clap`) and environment variables
    - Default values: `localhost:55555` and `1.0`
    - _Requirements: 02-REQ-2.3_

  - [ ] 4.3 Write property-based tests for `validate_lock`
    - Use `proptest` crate to generate random inputs
    - Assert: speed >= threshold → RejectedSpeed (regardless of door state)
    - Assert: speed < threshold AND lock AND door_open → RejectedDoorOpen
    - Assert: speed < threshold AND lock AND door_closed → Success
    - Assert: speed < threshold AND unlock → Success (regardless of door)
    - Boundary test: speed exactly at threshold (1.0 → rejected)
    - Boundary test: speed just below threshold (0.99 → accepted)
    - **Property 4: Safety Function Purity**
    - **Validates: Requirements 02-REQ-3.1, 02-REQ-3.2, 02-REQ-3.3, 02-REQ-3.4**

  - [ ] 4.V Verify task group 4
    - [ ] `cargo test -p locking-service -- safety` passes all property tests
    - [ ] `cargo clippy -p locking-service -- -D warnings` clean
    - [ ] Requirements 02-REQ-3.1–3.4 acceptance criteria met

- [ ] 5. LOCKING_SERVICE Command Handler
  - [ ] 5.1 Create `lock_handler.rs` module
    - Implement `run_lock_handler(client: KuksaClient, config: Config)`
    - Subscribe to `Vehicle.Command.Door.Lock`
    - On each signal change: read speed and door state, validate, execute if
      valid, report result
    - Use safe defaults for missing signals (02-REQ-3.E1, 02-REQ-3.E2)
    - _Requirements: 02-REQ-2.2, 02-REQ-4.1, 02-REQ-4.2, 02-REQ-5.1,
      02-REQ-5.2, 02-REQ-5.3, 02-REQ-5.4_

  - [ ] 5.2 Define VSS signal path constants
    - Create a `signals` module (in `parking-proto` or `locking-service`) with
      all VSS path constants from the design document
    - _Requirements: 02-REQ-2.1 (supporting)_

  - [ ] 5.3 Update `main.rs`
    - Replace spec 01 skeleton with real implementation
    - Parse config, connect to Kuksa, run lock handler
    - Handle SIGINT/SIGTERM for graceful shutdown
    - Implement connection retry with exponential backoff (02-REQ-2.E1)
    - Implement subscription re-establishment on stream interruption (02-REQ-2.E2)
    - _Requirements: 02-REQ-2.1, 02-REQ-2.3, 02-REQ-2.E1, 02-REQ-2.E2_

  - [ ] 5.4 Write unit tests for lock_handler
    - Use a trait-based mock for the Kuksa client
    - Test: lock command with safe conditions → IsLocked written, LockResult
      = SUCCESS
    - Test: lock command with high speed → IsLocked NOT written, LockResult
      = REJECTED_SPEED
    - Test: lock command with door open → IsLocked NOT written, LockResult
      = REJECTED_DOOR_OPEN
    - Test: unlock command with safe conditions → IsLocked = false, LockResult
      = SUCCESS
    - Test: missing speed signal → treated as 0.0
    - Test: missing door signal → treated as closed
    - **Property 1: Command-Lock Invariant**
    - **Property 2: Safety Rejection Guarantee**
    - **Property 3: Result Completeness**
    - **Property 7: Default-Safe Behavior**
    - **Validates: Requirements 02-REQ-3.E1, 02-REQ-3.E2, 02-REQ-4.1,
      02-REQ-4.2, 02-REQ-5.1–5.4**

  - [ ] 5.V Verify task group 5
    - [ ] `cargo test -p locking-service` passes all tests
    - [ ] `cargo clippy -p locking-service -- -D warnings` clean
    - [ ] `cargo build -p locking-service` produces binary
    - [ ] Requirements 02-REQ-2.1–2.3, 02-REQ-3.E1–E2, 02-REQ-4.1–4.2,
      02-REQ-5.1–5.4 acceptance criteria met

- [ ] 6. Checkpoint — LOCKING_SERVICE Complete
  - Safety validation tested, command handler tested with mocks
  - Commit and verify clean state before proceeding to mock sensors

- [ ] 7. Mock Sensors Implementation
  - [ ] 7.1 Implement mock-sensors subcommands
    - Replace spec 01 skeleton with real Kuksa gRPC writes
    - `set-location <lat> <lon>`: write to
      `Vehicle.CurrentLocation.{Latitude,Longitude}`
    - `set-speed <km/h>`: write to `Vehicle.Speed`
    - `set-door <open|closed>`: write bool to
      `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`
    - `lock-command <lock|unlock>`: write bool to `Vehicle.Command.Door.Lock`
    - All subcommands: connect to Kuksa, write, print confirmation, exit
    - _Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4,
      02-REQ-6.5_

  - [ ] 7.2 Add error handling
    - Unreachable DATA_BROKER → error message + non-zero exit
    - Invalid arguments → clap error message + non-zero exit
    - _Requirements: 02-REQ-6.E1_

  - [ ] 7.3 Write mock-sensors tests
    - Unit tests: argument parsing for each subcommand
    - Integration test (`#[ignore]`): write value via mock-sensors, read back
      from Kuksa client, verify match (for each signal type)
    - **Property 5: Mock Sensor Signal Accuracy**
    - **Validates: Requirements 02-REQ-6.1–6.5**

  - [ ] 7.V Verify task group 7
    - [ ] `cargo test -p mock-sensors` passes unit tests
    - [ ] `cargo test -p mock-sensors -- --ignored` passes integration tests
      (with `make infra-up`)
    - [ ] `mock-sensors --help` shows all subcommands
    - [ ] `cargo clippy -p mock-sensors -- -D warnings` clean
    - [ ] Requirements 02-REQ-6.1–6.5 acceptance criteria met

- [ ] 8. Integration Tests
  - [ ] 8.1 Create integration test module
    - Create `tests/integration/` directory or `locking-service/tests/`
      directory for integration tests
    - Test harness: start LOCKING_SERVICE as a background process connected to
      real Kuksa, use Kuksa client to set signals and verify results
    - Tests are `#[ignore]` by default; require `make infra-up`
    - _Requirements: 02-REQ-7.E1_

  - [ ] 8.2 Test happy path: lock and unlock
    - Set speed = 0.0, door = closed
    - Write lock command → verify IsLocked = true, LockResult = "SUCCESS"
    - Write unlock command → verify IsLocked = false, LockResult = "SUCCESS"
    - **Property 1: Command-Lock Invariant**
    - _Requirements: 02-REQ-7.1, 02-REQ-7.4_

  - [ ] 8.3 Test safety rejections
    - Set speed = 50.0 → lock command → verify IsLocked unchanged,
      LockResult = "REJECTED_SPEED"
    - Set speed = 0.0, door = open → lock command → verify IsLocked unchanged,
      LockResult = "REJECTED_DOOR_OPEN"
    - **Property 2: Safety Rejection Guarantee**
    - **Property 3: Result Completeness**
    - _Requirements: 02-REQ-7.2, 02-REQ-7.3_

  - [ ] 8.4 Test unlock with door open (allowed)
    - Set speed = 0.0, door = open, IsLocked = true
    - Write unlock command → verify IsLocked = false, LockResult = "SUCCESS"
    - Validates that door-ajar check is lock-only
    - _Requirements: 02-REQ-3.4_

  - [ ] 8.V Verify task group 8
    - [ ] All integration tests pass with `make infra-up` running
    - [ ] Tests skip cleanly when DATA_BROKER is unavailable
    - [ ] Requirements 02-REQ-7.1–7.4 acceptance criteria met

- [ ] 9. Final Verification and Documentation
  - [ ] 9.1 Run full test suite
    - `make build && make test && make lint`
    - Verify no regressions in spec 01 tests
    - _Requirements: all_

  - [ ] 9.2 Run integration tests
    - `make infra-up && cd rhivos && cargo test --workspace -- --ignored`
    - Verify all integration tests pass

  - [ ] 9.3 Update documentation
    - Update README if needed (no new Makefile targets expected, but verify
      mock-sensors documentation)
    - Document the VSS overlay signals in `docs/vss-signals.md` or similar

  - [ ] 9.V Verify task group 9
    - [ ] `make build` succeeds
    - [ ] `make test` passes (unit tests)
    - [ ] `make lint` clean
    - [ ] Integration tests pass
    - [ ] No regressions from spec 01
    - [ ] All 02-REQ requirements verified

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Implemented By Task | Verified By Test |
|-------------|---------------------|------------------|
| 02-REQ-1.1 | 1.1 | `tests` in 1.3 (signal accessible) |
| 02-REQ-1.2 | 1.1 | `tests` in 1.3 (signal accessible) |
| 02-REQ-1.3 | 1.1 | `tests` in 1.3 (signal accessible) |
| 02-REQ-1.4 | 1.2 | `make infra-up` + 1.3 verification |
| 02-REQ-1.5 | 1.1 | File exists at expected path |
| 02-REQ-1.E1 | — | Standard Kuksa behavior |
| 02-REQ-2.1 | 5.3 | Integration tests (8.2, 8.3) |
| 02-REQ-2.2 | 5.1 | Unit tests (5.4), integration tests (8.2) |
| 02-REQ-2.3 | 4.2, 5.3 | Unit test (config parsing) |
| 02-REQ-2.E1 | 5.3 | Unit test (mock connection failure) |
| 02-REQ-2.E2 | 5.1, 5.3 | Unit test (mock stream interruption) |
| 02-REQ-3.1 | 4.1 | Property tests (4.3) |
| 02-REQ-3.2 | 4.1 | Property tests (4.3) |
| 02-REQ-3.3 | 4.1 | Property tests (4.3) |
| 02-REQ-3.4 | 4.1 | Property tests (4.3), integration test (8.4) |
| 02-REQ-3.E1 | 5.1 | Unit test (5.4, missing speed) |
| 02-REQ-3.E2 | 5.1 | Unit test (5.4, missing door) |
| 02-REQ-4.1 | 5.1 | Unit tests (5.4), integration tests (8.2) |
| 02-REQ-4.2 | 5.1 | Unit tests (5.4), integration tests (8.3) |
| 02-REQ-4.E1 | 5.1 | Unit test (5.4, write failure) |
| 02-REQ-5.1 | 5.1 | Unit tests (5.4), integration tests (8.2) |
| 02-REQ-5.2 | 5.1 | Unit tests (5.4), integration tests (8.3) |
| 02-REQ-5.3 | 5.1 | Unit tests (5.4), integration tests (8.3) |
| 02-REQ-5.4 | 5.1 | Unit tests (5.4) |
| 02-REQ-5.E1 | 5.1 | Unit test (5.4, write failure) |
| 02-REQ-6.1 | 7.1 | Integration test (7.3) |
| 02-REQ-6.2 | 7.1 | Integration test (7.3) |
| 02-REQ-6.3 | 7.1 | Integration test (7.3) |
| 02-REQ-6.4 | 7.1 | Integration test (7.3) |
| 02-REQ-6.5 | 7.1 | Unit test (7.3, arg parsing) |
| 02-REQ-6.E1 | 7.2 | Unit test (7.3, connection failure) |
| 02-REQ-7.1 | 8.2 | Integration test `test_lock_happy_path` |
| 02-REQ-7.2 | 8.3 | Integration test `test_lock_rejected_speed` |
| 02-REQ-7.3 | 8.3 | Integration test `test_lock_rejected_door_open` |
| 02-REQ-7.4 | 8.2 | Integration test `test_unlock_happy_path` |
| 02-REQ-7.E1 | 8.1 | Integration test skip behavior |

## Notes

- **proptest dependency:** Add `proptest` as a `[dev-dependency]` in the
  locking-service `Cargo.toml` for property-based testing of `validate_lock`.
- **Trait-based Kuksa client mock:** Define a `DataBroker` trait in the
  locking-service crate with async methods matching the `KuksaClient`
  interface. The real implementation wraps `KuksaClient`; tests inject a mock.
  This allows testing `lock_handler` without a running Kuksa.
- **Integration test lifecycle:** Integration tests start LOCKING_SERVICE as a
  spawned process, wait for its startup log message, then run test assertions.
  Tests clean up by killing the process on completion. An alternative is to
  run the handler in a spawned Tokio task within the test process.
- **Kuksa proto version:** Verify the exact version of `kuksa.val.v2` proto
  files matches the Kuksa Databroker container image version used in
  `infra/compose.yaml`. Version mismatch will cause gRPC errors.
- **No changes to spec 01 skeletons for other services:** Only
  `locking-service` and `mock-sensors` are modified. All other service
  skeletons remain as UNIMPLEMENTED stubs.
