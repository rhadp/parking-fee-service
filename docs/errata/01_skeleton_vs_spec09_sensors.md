# Erratum 01: Skeleton vs Spec 09 Mock Sensor Test Conflicts

**Spec:** 01_project_setup  
**Date:** 2026-04-17  
**Status:** Known divergence — not a bug

## Summary

After implementing spec 01 task group 3 skeleton binaries, 5 tests in
`rhivos/mock-sensors/tests/sensors.rs` fail. These failures are expected and
pre-existing from spec 09 task group 1 (which wrote failing tests for the full
mock sensor implementation).

## Failing Tests

| Test | Expected | Spec 01 Behavior |
|------|----------|-----------------|
| `test_location_sensor_no_args` | exit 1 (required args missing) | exit 0 (version print) |
| `test_speed_sensor_no_args` | exit 1 (required args missing) | exit 0 (version print) |
| `test_speed_sensor_missing_speed` | exit 1 (`--speed` required) | exit 0 (version print) |
| `test_door_sensor_no_args` | exit 1 (required args missing) | exit 0 (version print) |
| `test_door_sensor_missing_state` | exit 1 (`--open`/`--closed` required) | exit 0 (version print) |

## Root Cause

**Spec 01 requirement (01-REQ-4.3):** Each mock-sensor binary SHALL print its
name and version when executed with no arguments and exit with code 0.

**Spec 09 requirement (09-REQ-E1 through E3):** Mock sensor binaries SHALL
require `--lat`/`--lon`, `--speed`, and `--open`/`--closed` flags respectively
and exit 1 when they are missing.

These requirements are fundamentally incompatible: spec 01 skeletons exit 0
with no args; spec 09 full implementations exit 1 with no args.

## Resolution

These failures will be resolved when spec 09 task group 2 implements the full
clap-based argument parsing for the sensor binaries. At that point, the sensors
will exit 1 when required args are missing (satisfying spec 09), and the spec
01 requirement for "skeleton exit 0 with no args" is superseded by the full
implementation.

For the duration of spec 01 task group 3, these 5 tests remain failing as
pre-existing spec 09 task group 1 failures. All spec 01 tests pass.
