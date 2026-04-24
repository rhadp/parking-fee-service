# Erratum: kuksa.val.v2 API Migration

**Spec:** 08_parking_operator_adaptor
**Affected requirements:** 08-REQ-3.1, 08-REQ-3.2, 08-REQ-4.1, 08-REQ-4.2, 08-REQ-4.3
**Date:** 2026-04-24

## Deviation

The requirements document and design document specify `kuksa.val.v1` throughout
(e.g. "tonic-generated kuksa.val.v1 gRPC client", "kuksa.val.v1 proto" in the
Technology Stack table). The implementation uses `kuksa.val.v2` instead.

## Rationale

The project has already migrated all services to `kuksa.val.v2`, as evidenced
by:

- The locking-service (`rhivos/locking-service/`) uses `kuksa.val.v2`
  proto definitions and client
- The mock-sensors crate (`rhivos/mock-sensors/`) uses `kuksa.val.v2`
- Erratum `03_kuksa_v2_migration.md` documents the same migration for
  the locking-service

The running DATA_BROKER instance (Kuksa Databroker 0.6) serves the v2 API.
Implementing against the v1 API would produce an incompatible service.

## API Mapping

| v1 RPC | v2 RPC | Usage in parking-operator-adaptor |
|--------|--------|-----------------------------------|
| `Set(SetRequest)` | `PublishValue(PublishValueRequest)` | Write Vehicle.Parking.SessionActive |
| `Subscribe(SubscribeRequest)` | `Subscribe(SubscribeRequest)` | Subscribe to IsLocked signal |

The v2 proto is vendored at `rhivos/parking-operator-adaptor/proto/kuksa/val.proto`,
matching the copy used by locking-service and mock-sensors.
