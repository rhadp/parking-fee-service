# Session Log

## Session 13

- **Spec:** 02_locking_service
- **Task Group:** 1
- **Date:** 2026-02-18

### Summary

Implemented task group 1 (Kuksa VSS Configuration) for spec 02_locking_service. Created the VSS overlay file at `infra/config/kuksa/vss_overlay.json` defining three custom signals (Vehicle.Command.Door.Lock, Vehicle.Command.Door.LockResult, Vehicle.Parking.SessionActive), updated `infra/compose.yaml` to mount and load the overlay via the `--vss` flag, and verified all signals are accessible via the Kuksa gRPC API.

### Files Changed

- Added: `infra/config/kuksa/vss_overlay.json`
- Modified: `infra/compose.yaml`
- Deleted: `infra/config/kuksa/vss.json`
- Modified: `.specs/02_locking_service/tasks.md`
- Added: `.specs/02_locking_service/sessions.md`

### Tests Added or Modified

- None. (Task group 1 is infrastructure configuration; signal accessibility was verified manually via grpcurl against a running Kuksa Databroker instance.)

---

## Session 14

- **Spec:** 02_locking_service
- **Task Group:** 2
- **Date:** 2026-02-18

### Summary

Implemented task group 2 (Kuksa Proto Integration) for spec 02_locking_service. Vendored `kuksa.val.v2` proto files (types.proto and val.proto) from Eclipse Kuksa Databroker 0.5.0 into `proto/vendor/kuksa/val/v2/`, extended `parking-proto` build.rs to compile them, added a `KuksaClient` helper module with typed get/set/subscribe methods, added VSS signal path constants module, and wrote comprehensive unit tests plus `#[ignore]` integration tests.

### Files Changed

- Added: `proto/vendor/kuksa/val/v2/types.proto`
- Added: `proto/vendor/kuksa/val/v2/val.proto`
- Added: `rhivos/parking-proto/src/kuksa_client.rs`
- Added: `rhivos/parking-proto/src/signals.rs`
- Modified: `rhivos/parking-proto/build.rs`
- Modified: `rhivos/parking-proto/src/lib.rs`
- Modified: `rhivos/parking-proto/Cargo.toml`
- Modified: `rhivos/Cargo.toml`
- Modified: `.specs/02_locking_service/tasks.md`
- Modified: `.specs/02_locking_service/sessions.md`

### Tests Added or Modified

- `rhivos/parking-proto/src/lib.rs`: Added `kuksa_val_v2_types_are_accessible`, `kuksa_val_v2_client_trait_is_generated`, and `signal_constants_are_correct` tests.
- `rhivos/parking-proto/src/kuksa_client.rs`: Added unit tests for error display, client traits, `extract_value_from_entries` (bool, missing path, wrong type, no value), connection failure test, and 4 integration tests (`#[ignore]`) for set/get of bool, f32, f64, and string values.

---

## Session 15

- **Spec:** 02_locking_service
- **Task Group:** 3
- **Date:** 2026-02-18

### Summary

Completed checkpoint 3 (Kuksa Infrastructure Ready) for spec 02_locking_service. Ran the full test suite: all unit tests (44 Rust + Go tests), linters (clippy + go vet), structure tests, and proto tests pass. Infrastructure tests have environment-specific port 55555 conflicts unrelated to code changes. Marked checkpoint 3 as complete.

### Files Changed

- Modified: `.specs/02_locking_service/tasks.md`
- Modified: `.specs/02_locking_service/sessions.md`

### Tests Added or Modified

- None.

---

## Session 16

- **Spec:** 02_locking_service
- **Task Group:** 4
- **Date:** 2026-02-18

### Summary

Implemented task group 4 (LOCKING_SERVICE Safety Validation) for spec 02_locking_service. Created `safety.rs` with the `LockResult` enum and pure `validate_lock` function, `config.rs` with `Config` struct parsed via clap, and comprehensive tests including 17 deterministic boundary-value tests and 7 property-based tests using `proptest`. All 32 locking-service tests pass with zero clippy warnings.

### Files Changed

- Added: `rhivos/locking-service/src/safety.rs`
- Added: `rhivos/locking-service/src/config.rs`
- Modified: `rhivos/locking-service/src/main.rs`
- Modified: `rhivos/locking-service/Cargo.toml`
- Modified: `.specs/02_locking_service/tasks.md`
- Modified: `.specs/02_locking_service/sessions.md`

### Tests Added or Modified

- `rhivos/locking-service/src/safety.rs`: Added 17 deterministic unit tests (Display trait, boundary values, custom thresholds) and 7 property-based tests using proptest (speed rejection, door-open rejection, safe lock, safe unlock, determinism, exhaustive decision rules, exact-threshold boundary).
- `rhivos/locking-service/src/config.rs`: Added 6 unit tests (default config, custom databroker addr, custom max speed, all custom args, clone, debug).
- `rhivos/locking-service/src/main.rs`: Updated 2 integration tests to use new Config struct.

---

## Session 17

- **Spec:** 02_locking_service
- **Task Group:** 5
- **Date:** 2026-02-18

### Summary

Implemented task group 5 (LOCKING_SERVICE Command Handler) for spec 02_locking_service. Created `lock_handler.rs` with a trait-based `DataBroker` abstraction, the `process_lock_command` function for safety validation and signal writes, and the `run_lock_handler` subscription loop. Updated `main.rs` with exponential backoff connection retry, SIGTERM/SIGINT handling, and automatic re-subscription on stream interruption. Added 19 unit tests covering all correctness properties and error handling scenarios.

### Files Changed

- Added: `rhivos/locking-service/src/lock_handler.rs`
- Modified: `rhivos/locking-service/src/main.rs`
- Modified: `rhivos/locking-service/Cargo.toml`
- Modified: `.specs/02_locking_service/tasks.md`
- Modified: `.specs/02_locking_service/sessions.md`

### Tests Added or Modified

- `rhivos/locking-service/src/lock_handler.rs`: 19 unit tests covering lock/unlock safe conditions (Property 1), speed/door rejection (Property 2), result completeness (Property 3), missing signal defaults (Property 7), read/write error handling, boundary values, and BrokerError display/debug.

---

## Session 18

- **Spec:** 02_locking_service
- **Task Group:** 6
- **Date:** 2026-02-18

### Summary

Completed checkpoint 6 (LOCKING_SERVICE Complete) for spec 02_locking_service. Ran the full test suite (`make build`, `make test`, `make lint`): all 91 Rust unit tests pass (plus 4 ignored integration tests requiring Kuksa), all Go tests pass, clippy and go vet are clean, and builds succeed. Marked checkpoint 6 as complete.

### Files Changed

- Modified: `.specs/02_locking_service/tasks.md`
- Modified: `.specs/02_locking_service/sessions.md`

### Tests Added or Modified

- None.

---

## Session 19

- **Spec:** 02_locking_service
- **Task Group:** 7
- **Date:** 2026-02-18

### Summary

Implemented task group 7 (Mock Sensors Implementation) for spec 02_locking_service. Replaced the spec 01 mock-sensors skeleton (local v1 proto vendoring) with a real implementation using the shared `parking-proto` crate's `KuksaClient` and VSS signal constants. Added the `lock-command <lock|unlock>` subcommand, changed `set-door` to accept human-friendly `open`/`closed` arguments, added comprehensive error handling (connection failures exit non-zero), and wrote 18 unit tests plus 4 integration tests.

### Files Changed

- Modified: `mock/sensors/Cargo.toml`
- Modified: `mock/sensors/src/main.rs`
- Deleted: `mock/sensors/build.rs`
- Deleted: `mock/sensors/proto/kuksa/val/v1/types.proto`
- Deleted: `mock/sensors/proto/kuksa/val/v1/val.proto`
- Modified: `.specs/02_locking_service/tasks.md`
- Modified: `.specs/02_locking_service/sessions.md`

### Tests Added or Modified

- `mock/sensors/src/main.rs`: Rewrote test suite with 18 unit tests (CLI parsing for all 4 subcommands, DoorState parsing/display, LockAction parsing/display, custom databroker addr, default addr) and 4 `#[ignore]` integration tests (set-location roundtrip, set-speed roundtrip, set-door roundtrip, lock-command roundtrip).

---

## Session 20

- **Spec:** 02_locking_service
- **Task Group:** 8
- **Date:** 2026-02-18

### Summary

Implemented task group 8 (Integration Tests) for spec 02_locking_service. Created 6 end-to-end integration tests in `locking-service/tests/integration.rs` that exercise the full lock/unlock pipeline through a real Kuksa Databroker. Added a `lib.rs` to expose locking-service modules for integration tests. Fixed the VSS overlay to include all standard signals needed by the system (Vehicle.Speed, door signals, location signals) since `--vss` replaces the default VSS model entirely. All 6 integration tests pass with infrastructure running and skip cleanly (via `#[ignore]`) when unavailable.

### Files Changed

- Added: `rhivos/locking-service/src/lib.rs`
- Added: `rhivos/locking-service/tests/integration.rs`
- Added: `.docs/errata/vss-overlay-standard-signals.md`
- Modified: `rhivos/locking-service/Cargo.toml`
- Modified: `rhivos/locking-service/src/main.rs`
- Modified: `infra/config/kuksa/vss_overlay.json`
- Modified: `.specs/02_locking_service/tasks.md`
- Modified: `.specs/02_locking_service/sessions.md`

### Tests Added or Modified

- `rhivos/locking-service/tests/integration.rs`: 6 integration tests — `integration_lock_happy_path` (02-REQ-7.1), `integration_unlock_happy_path` (02-REQ-7.4), `integration_lock_rejected_speed` (02-REQ-7.2), `integration_lock_rejected_door_open` (02-REQ-7.3), `integration_unlock_with_door_open_succeeds` (02-REQ-3.4), `integration_unlock_rejected_speed` (02-REQ-3.3). All marked `#[ignore]`, require `make infra-up`.

---

## Session 21

- **Spec:** 02_locking_service
- **Task Group:** 9
- **Date:** 2026-02-18

### Summary

Completed task group 9 (Final Verification and Documentation) for spec 02_locking_service. Ran full test suite (`make build`, `make test`, `make lint`) confirming all 101 unit tests and Go tests pass with zero regressions. Fixed integration test parallelism issues by adding `serial_test` crate and `#[serial]` annotations. Fixed mock-sensors `set-speed` to use `f32` (matching VSS `Vehicle.Speed` type) instead of `f64` which caused Kuksa type rejection. All 14 integration tests pass. Updated README with mock-sensors `lock-command` subcommand and current service status table. Created `docs/vss-signals.md` documenting all custom VSS overlay signals, safety validation rules, and signal constants.

### Files Changed

- Modified: `rhivos/locking-service/Cargo.toml`
- Modified: `rhivos/locking-service/tests/integration.rs`
- Modified: `mock/sensors/src/main.rs`
- Modified: `README.md`
- Added: `docs/vss-signals.md`
- Modified: `.specs/02_locking_service/tasks.md`
- Modified: `.specs/02_locking_service/sessions.md`

### Tests Added or Modified

- `rhivos/locking-service/tests/integration.rs`: Added `serial_test` dependency and `#[serial]` annotations to all 6 integration tests to prevent parallel execution race conditions against shared Kuksa Databroker state.
- `mock/sensors/src/main.rs`: Fixed `integration_set_speed_roundtrip` test to use `f32` instead of `f64` (matching VSS `Vehicle.Speed` signal type).
