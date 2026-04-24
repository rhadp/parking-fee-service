# Erratum: kuksa.val.v2 API Migration

**Spec:** 03_locking_service
**Affected requirements:** 03-REQ-1.1, 03-REQ-1.E1, 03-REQ-1.E2
**Date:** 2026-04-24

## Deviation

The requirements document and design document specify `kuksa.val.v1` throughout
(e.g. "kuksa.val.v1 gRPC Subscribe RPC", "tonic-generated Rust client from
kuksa.val.v1 proto definitions"). The implementation uses `kuksa.val.v2` instead.

## Rationale

The project has already migrated all other services to `kuksa.val.v2`, as
evidenced by:

- Commit `fix(04): migrate broker client and tests to kuksa v2 API`
- Commit `fix(08): migrate to kuksa v2 API and fix signal handler race`
- The mock-sensors crate (`rhivos/mock-sensors/`) uses `kuksa.val.v2` proto
  definitions exclusively

The running DATA_BROKER instance serves the v2 API. Implementing the
locking-service against the v1 API would produce a service incompatible with
the deployed infrastructure.

## API Mapping

| v1 RPC | v2 RPC | Usage in locking-service |
|--------|--------|--------------------------|
| `Get(GetRequest)` | `GetValue(GetValueRequest)` | Read speed and door signals |
| `Set(SetRequest)` | `PublishValue(PublishValueRequest)` | Write lock state and responses |
| `Subscribe(SubscribeRequest)` | `Subscribe(SubscribeRequest)` | Subscribe to command signal |

The v2 proto is vendored at `rhivos/locking-service/proto/kuksa/val.proto`,
matching the copy used by mock-sensors.
