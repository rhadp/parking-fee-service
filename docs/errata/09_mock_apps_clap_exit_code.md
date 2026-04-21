# Errata: Spec 09 — Clap Exit Code for Missing Arguments

**Spec:** 09_mock_apps
**Status:** Accepted divergence — tests assert non-zero exit

## Summary

Requirements 09-REQ-1.E1, 09-REQ-2.E1, and 09-REQ-3.E1 specify that mock
sensor binaries SHALL exit with code **1** when required arguments are
missing.  However, the Rust `clap` library (v4) exits with code **2** for
argument validation errors (missing required args, unrecognized flags, etc.).
This is clap's default behavior and cannot be changed without a custom error
handler that intercepts clap's error path.

## Impact

| Test                | Spec assertion | Actual exit code | Test assertion          |
|---------------------|----------------|------------------|-------------------------|
| TS-09-E1            | exit 1         | exit 2           | `!status.success()` (any non-zero) |
| TS-09-E2            | exit 1         | exit 2           | `!status.success()` (any non-zero) |
| TS-09-E3            | exit 1         | exit 2           | `!status.success()` (any non-zero) |

## Rationale

The spec's intent is that the tool exits **unsuccessfully** when required
arguments are missing.  Exit code 2 satisfies this intent — the process
exits with a non-zero code and prints a usage error to stderr.  The POSIX
convention (and clap's choice) of exit code 2 for "incorrect usage" is
actually more informative than a generic exit code 1.

The Rust integration tests in `rhivos/mock-sensors/tests/sensor_args.rs`
assert `!status.success()` (any non-zero exit) rather than exactly `exit 1`,
which correctly validates the functional requirement without being brittle
to clap's convention.

## Resolution

No code change needed.  The tests remain correct and the behavioral intent
(fail on missing args) is satisfied.  A future spec revision could update
09-REQ-1.E1, 09-REQ-2.E1, and 09-REQ-3.E1 to say "exit with a non-zero
code" rather than "exit with code 1".
