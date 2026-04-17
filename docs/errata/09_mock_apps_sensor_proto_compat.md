# Errata: Spec 09 Mock Apps — Sensor Proto Compatibility with Real DATA_BROKER

**Related Spec:** 09_mock_apps (task group 5)
**Date:** 2026-04-17

## Situation

The sensor integration tests (TS-09-1 through TS-09-4) and the smoke test
TS-09-SMOKE-1 require a running DATA_BROKER that the mock sensor binaries can
publish to. These tests are designed to skip when DATA_BROKER is unavailable.

An additional skip condition was added during task group 5: the tests also skip
when the DATA_BROKER does not expose the `kuksa.VALService` gRPC service used
by the sensor binaries.

## Root Cause

The mock sensor binaries (`location-sensor`, `speed-sensor`, `door-sensor`)
were implemented in task group 2 using a simplified custom proto file located
at `rhivos/mock-sensors/proto/kuksa/val.proto`. This proto defines:

```protobuf
service VALService {
  rpc Set(SetRequest) returns (SetResponse);
  rpc Get(GetRequest) returns (GetResponse);
  ...
}
```

The real `kuksa-databroker` v0.5.0 (used by this project's Docker Compose
infrastructure) exposes the **`kuksa.val.v2.VAL`** service, which has a
different API:
- `PublishValue` instead of `Set`
- `GetValue` instead of `Get`
- Different message formats

When a sensor binary tries to call `kuksa.VALService/Set` on a real
kuksa-databroker v0.5.0 instance, the broker returns gRPC status
`Unimplemented` because it does not implement the `kuksa.VALService` interface.

## Impact

- Sensor integration tests skip when connected to a standard kuksa-databroker.
- The sensors would only work against a custom DATA_BROKER implementation that
  implements the `kuksa.VALService` proto defined in
  `rhivos/mock-sensors/proto/kuksa/val.proto`.
- The `TestLocationSensor`, `TestSpeedSensor`, `TestDoorSensorOpen`,
  `TestDoorSensorClosed`, and `TestAllSensorsSmoke` tests are written and
  syntactically correct, but skip in any environment with a standard DATA_BROKER.

## Mitigation

The sensor integration tests check for `kuksa.VALService` support via grpcurl
service reflection before attempting to run the sensor binaries. This ensures
clean skipping with a descriptive message.

## Future Resolution

To make sensor integration tests pass against the standard kuksa-databroker:

1. Update `rhivos/mock-sensors/proto/kuksa/val.proto` to use the official
   `kuksa.val.v2` proto specification.
2. Update `rhivos/mock-sensors/src/lib.rs` to use the `kuksa.val.v2.VAL`
   gRPC service and `PublishValue` RPC instead of `Set`.
3. Update integration tests to query values via `kuksa.val.v2.VAL/GetValue`.

This change is deferred to a future spec iteration.
