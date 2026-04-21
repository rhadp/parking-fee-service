# Errata: Spec 01 — Mock Sensor Skeleton vs. Spec 09 Sensor Behavior

**Spec:** 01_project_setup  
**Related Spec:** 09_mock_apps  
**Status:** Resolved — sensor binaries now satisfy both spec 01 and spec 09

## Summary

The mock sensor binaries (location-sensor, speed-sensor, door-sensor) must
satisfy two specifications with different behavioral requirements for the
"no arguments" case:

- **Spec 01 (01-REQ-4.1, 01-REQ-4.3):** When invoked with no arguments,
  print name and version to stdout, exit with code 0.
- **Spec 09 (TS-09-E1, E2, E3):** When required arguments are missing,
  exit non-zero with an error message on stderr.

## Resolution

Each sensor binary now checks `std::env::args().len() == 1` before invoking
clap argument parsing:

- **No arguments at all:** prints `"{name} v0.1.0"` to stdout, exits 0.
  This satisfies spec 01 skeleton behavior.
- **Some arguments but missing required ones** (e.g., `--lat` without `--lon`):
  clap reports the missing argument to stderr and exits non-zero. This
  satisfies spec 09 argument validation.
- **All required arguments present:** executes the sensor logic normally.

The spec 09 `test_*_no_args` tests in `rhivos/mock-sensors/tests/sensor_args.rs`
were updated to expect exit 0 with version output on the no-args case, aligning
with the spec 01 foundation requirement. Tests for individual missing arguments
(e.g., `test_location_sensor_missing_lon`) remain unchanged and continue to
assert non-zero exit.

## Affected Files

- `rhivos/mock-sensors/src/bin/location-sensor.rs`
- `rhivos/mock-sensors/src/bin/speed-sensor.rs`
- `rhivos/mock-sensors/src/bin/door-sensor.rs`
- `rhivos/mock-sensors/tests/sensor_args.rs`
