# Errata: Mock Sensors kuksa.val.v2 API

## Context

Spec 09 requirements (09-REQ-1.1, 09-REQ-2.1, 09-REQ-3.1, 09-REQ-10.1,
09-REQ-10.2) mandate that mock sensors use the `kuksa.val.v1` gRPC API
with a `Set` RPC. The design document also specifies v1.

## Deviation

The mock sensors use the `kuksa.val.v2` gRPC API with the `PublishValue`
RPC, consistent with:

- The project-wide `proto/kuksa/val.proto` (package `kuksa.val.v2`)
- Other RHIVOS components (locking-service, parking-operator-adaptor)
  that also use v2 (see `docs/errata/03_kuksa_v2_migration.md` and
  `docs/errata/08_kuksa_v2_migration.md`)
- The Eclipse Kuksa Databroker v0.5.0+ which exposes only v2

## API Mapping

| Spec (v1)     | Implementation (v2)  |
|---------------|---------------------|
| `Set` RPC     | `PublishValue` RPC  |
| `DataEntry`   | `PublishValueRequest` with `SignalID` + `Datapoint` |
| `Datapoint.value` (oneof) | `Value.typed_value` (oneof) |

## Testing Impact

Integration tests (TS-09-1 through TS-09-4, TS-09-SMOKE-1, TS-09-P1)
use a stub `kuksa.val.v2.VAL` gRPC server implemented in Go
(`tests/mock-apps/internal/kuksav2/`) with generated code from the
`rhivos/mock-sensors/proto/kuksa/val.proto` definition. This stub
captures `PublishValue` calls and allows tests to verify that the correct
VSS signal paths and values were published.

Tests against the real Kuksa Databroker would also work since the
implementation uses the same v2 API the broker exposes.

## Resolution

No code change needed. The spec text should be updated to reference
`kuksa.val.v2` and `PublishValue` in a future spec revision.
