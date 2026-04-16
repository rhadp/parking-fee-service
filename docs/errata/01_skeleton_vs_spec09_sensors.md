# Errata: Spec 01 Skeleton vs Spec 09 Sensor Tests

**Related Spec:** 01_project_setup (task group 3)
**Date:** 2026-04-17

## Situation

The spec 09 test suite (`rhivos/mock-sensors/tests/sensor_tests.rs`) contains
integration tests written in spec 09 group 1 (the "RED phase" — failing tests
written before the implementation). These tests expect the sensor binaries to
exit with code 1 when invoked without required arguments:

- `test_location_sensor_no_args`: expects `location-sensor` to exit 1
- `test_speed_sensor_no_args`: expects `speed-sensor` to exit 1
- `test_door_sensor_no_state_flag`: expects `door-sensor` to exit 1

## Divergence

Spec 01 requires skeleton binaries to print version info and exit 0 when
invoked with no arguments (01-REQ-4.1, 01-REQ-4.3). The sensor skeletons
implemented in spec 01 task group 3 print their version and exit 0 with no
args, which is correct per spec 01 but conflicts with the above spec 09 tests.

## State Before Task Group 3

All 8 tests in `sensor_tests.rs` were failing (0 passed). The sensor binaries
simply printed version strings and exited 0 regardless of arguments.

## State After Task Group 3

5 of 8 tests now pass. The flag-rejection logic (exit 1 + usage to stderr for
any arg starting with `-`) satisfies the "missing required arg" and "unreachable
broker" test cases (those tests pass flags like `--lat`, `--lon`, `--speed`,
`--open` which start with `--` and are now rejected). The "no args" tests
still fail because no-args exits 0 (version print).

## Resolution

The 3 remaining failures (`test_*_no_args`) require the full spec 09
sensor implementation (proper argument parsing that enforces required arguments).
These tests will be fixed when spec 09 groups 2+ are implemented. They are
pre-existing spec 09 group 1 RED-phase tests and are NOT regressions
introduced by spec 01 task group 3.

## Impact

- `cargo test --workspace` in `rhivos/` reports 3 failures (down from 8)
- These are spec 09 tests, not spec 01 tests
- All spec 01 placeholder tests pass
