# Errata: Spec 01 — Mock Sensor Skeleton vs. Spec 09 Sensor Tests

**Spec:** 01_project_setup  
**Related Spec:** 09_mock_apps  
**Status:** Expected — resolved by spec 09 implementation

## Summary

When running `cargo test --workspace` in `rhivos/`, 8 tests in
`mock-sensors/tests/sensor_args.rs` fail. These tests belong to spec 09
(mock apps) task group 1 and were intentionally written as failing tests
before the full sensor implementation exists.

## Failing Tests

All 8 tests live in `rhivos/mock-sensors/tests/sensor_args.rs`:

- `test_location_sensor_no_args` — expects exit 1 when `--lat`/`--lon` absent
- `test_location_sensor_missing_lon` — expects exit 1 when `--lon` absent
- `test_location_sensor_missing_lat` — expects exit 1 when `--lat` absent
- `test_speed_sensor_no_args` — expects exit 1 when `--speed` absent
- `test_door_sensor_no_args` — expects exit 1 when `--open`/`--closed` absent
- `test_location_sensor_unreachable_broker` — expects exit 1 on DATA_BROKER unreachable
- `test_speed_sensor_unreachable_broker` — expects exit 1 on DATA_BROKER unreachable
- `test_door_sensor_unreachable_broker` — expects exit 1 on DATA_BROKER unreachable

## Root Cause

Spec 01 task group 2 created the mock sensor skeleton binaries
(`location-sensor`, `speed-sensor`, `door-sensor`) with minimal implementations
that print a version string and exit 0. The spec 09 sensor_args integration tests
require fully implemented argument parsing and DATA_BROKER connectivity, which
will be delivered by spec 09 task groups 3–5.

## Affected Make Targets

- `make test` (`test-rust`) — **scoped to exclude mock-sensors**:
  `cargo test --workspace --exclude mock-sensors`. All tests pass.
- `cargo test --workspace` (unscoped) — 8 failures from sensor_args.rs.
- `make check` — uses `--no-run` for compilation check only; all tests compile.

## Resolution

These failures will be resolved when spec 09 implements the full sensor
argument-parsing logic and DATA_BROKER connectivity. At that point:
1. The `--exclude mock-sensors` scope guard in the Makefile can be removed.
2. This errata entry can be closed.
