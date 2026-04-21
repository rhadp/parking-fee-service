# Errata: Spec 01 — Mock Sensor Skeleton vs. Spec 09 Sensor Behavior

**Spec:** 01_project_setup
**Related Spec:** 09_mock_apps
**Status:** Resolved — spec 09 requirements take precedence for sensor binaries

## Summary

The mock sensor binaries (location-sensor, speed-sensor, door-sensor) were
originally implemented with a `std::env::args().len() == 1` check that printed
a version string and exited 0 when invoked with no arguments, following spec 01
skeleton behavior (01-REQ-4.1, 01-REQ-4.3).

However, spec 09 requirements are explicit:

- **09-REQ-1.E1:** IF `--lat` or `--lon` is missing, exit code 1.
- **09-REQ-2.E1:** IF `--speed` is missing, exit code 1.
- **09-REQ-3.E1:** IF neither `--open` nor `--closed` is provided, exit code 1.

The "no arguments" case is a subset of "missing required arguments." Spec 09
requirements take precedence for these binaries, and the no-args version bypass
has been removed.

## Resolution

All three sensor binaries now delegate argument parsing entirely to `clap`.
When invoked with no arguments, clap prints a usage error to stderr and exits
with code 2 (clap's convention for usage errors). This satisfies the spec 09
requirement that missing arguments produce a non-zero exit with an error on
stderr. See also `09_mock_apps_clap_exit_code.md` for the exit code 2 vs 1
divergence.

The spec 01 skeleton tests (`TestRustSkeletonBinaryPrintsVersion`,
`TestSkeletonDeterminism`) have been updated to exclude sensor binaries, and
`TestMockSensorBinaryPrintsName` has been replaced with
`TestMockSensorBinaryNoArgsExitsNonZero` to validate spec 09 behavior.

## Affected Files

- `rhivos/mock-sensors/src/bin/location-sensor.rs`
- `rhivos/mock-sensors/src/bin/speed-sensor.rs`
- `rhivos/mock-sensors/src/bin/door-sensor.rs`
- `rhivos/mock-sensors/tests/sensor_args.rs`
- `tests/setup/skeleton_binary_test.go`
