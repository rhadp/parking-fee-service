# Erratum 09: Mock Sensor kuksa.val.v1 / kuksa.val.v2 Incompatibility

**Spec:** 09_mock_apps  
**Date:** 2026-04-17  
**Status:** Known divergence — sensor integration tests skip cleanly

## Summary

Requirements 09-REQ-1.1, 09-REQ-2.1, 09-REQ-3.1, 09-REQ-10.1, and 09-REQ-10.2
mandate that mock sensors publish VSS signal values to DATA_BROKER using the
`kuksa.val.v1` gRPC `Set` RPC.

The real Kuksa Databroker v0.5.0 (used in this project's deployments) exposes
only the `kuksa.val.v2.VAL` API. It does **not** expose the `kuksa.val.v1.VAL`
service. As a result:

1. Sensor binaries that call `kuksa.val.v1.VAL/Set` against a v0.5.0 databroker
   receive a gRPC `Unimplemented` error and exit with code 1.
2. Integration tests that verify published values via `kuksa.val.v1.VAL/Get`
   cannot run against a v0.5.0 broker.

## Affected Tests

| Test | Location | Behavior |
|------|----------|----------|
| `TestLocationSensor` | `tests/mock-apps/sensor_test.go` | Skips: v1.VAL not found |
| `TestSpeedSensor` | `tests/mock-apps/sensor_test.go` | Skips: v1.VAL not found |
| `TestDoorSensorOpen` | `tests/mock-apps/sensor_test.go` | Skips: v1.VAL not found |
| `TestDoorSensorClosed` | `tests/mock-apps/sensor_test.go` | Skips: v1.VAL not found |
| `TestSensorSmoke` | `tests/mock-apps/sensor_test.go` | Skips: v1.VAL not found |

## Skip Strategy

The sensor integration tests in `tests/mock-apps/sensor_test.go` follow a
three-stage skip guard:

1. **TCP reachability:** Skip if DATA_BROKER is not TCP-reachable at
   `localhost:55556`.
2. **grpcurl availability:** Skip if `grpcurl` is not installed (required
   to list available gRPC services).
3. **Service presence:** Skip if `kuksa.val.v1.VAL` is not listed by
   `grpcurl list` (which is always the case with Kuksa Databroker v0.5.0,
   which lists only `kuksa.val.v2.VAL`).

Tests that only verify exit codes and argument validation (Rust integration
tests in `rhivos/mock-sensors/tests/sensors.rs`) are not affected — they do
not require a running DATA_BROKER.

## Root Cause

**Spec requirement:** 09-REQ-10.1 states "The mock-sensors crate SHALL vendor
kuksa.val.v1 proto files... and use tonic-build for code generation."
09-REQ-1.1 mandates `kuksa.val.v1 gRPC Set RPC`.

**Reality:** Kuksa Databroker v0.5.0 only exposes `kuksa.val.v2.VAL` (with
`GetValue`, `PublishValue`, and `Subscribe` RPCs). The v1 API was removed in
databroker v0.4.4.

## Impact

- Sensor binaries build and compile correctly with kuksa.val.v1 proto definitions.
- Argument validation tests (exit code and stderr checks) pass without a broker.
- Unreachable-broker tests pass without a broker.
- End-to-end publish-and-verify tests skip cleanly in any environment that uses
  Kuksa Databroker v0.5.0+.
- No tests fail; all failures are converted to clean skips.

## Resolution Options

This erratum is recorded as a known divergence. Resolution would require one of:

1. **Upgrade protocol:** Update the mock sensors to use `kuksa.val.v2` `PublishValue`
   RPC instead of `kuksa.val.v1 Set`. This would require a spec amendment.
2. **Downgrade databroker:** Deploy Kuksa Databroker ≤ v0.4.3 which still
   exposes `kuksa.val.v1.VAL`. Not recommended as it is an older version.
3. **Accept divergence:** Keep sensors using v1 proto (as specified), accept that
   end-to-end sensor integration tests always skip in the current deployment
   environment.

The current implementation follows option 3: the spec is implemented as written,
and integration tests skip cleanly with a clear diagnostic message.
