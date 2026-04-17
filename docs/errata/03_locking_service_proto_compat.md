# Errata: LOCKING_SERVICE Custom Proto vs DATA_BROKER API

**Related Spec:** 03_locking_service
**Date:** 2026-04-17

## Summary

The LOCKING_SERVICE uses a custom, simplified gRPC proto (`kuksa.VALService`) for
communicating with DATA_BROKER.  This proto was designed as a lightweight abstraction
over the actual Kuksa API.  However, `kuksa-databroker:0.5.0` exposes `kuksa.val.v2.VAL`
(not `kuksa.VALService`), which means the service's internal gRPC RPCs go to different
HTTP/2 paths than the DATA_BROKER expects.

## Impact

- **Unit tests:** Not affected.  Unit tests use `MockBrokerClient` and do not require
  a real DATA_BROKER connection.  All 39 unit tests + 6 property tests pass.

- **Integration tests:** Live tests (TS-03-1, TS-03-13, TS-03-SMOKE-*) that start a
  real DATA_BROKER container will fail because the `GrpcBrokerClient` sends RPCs to
  `/kuksa.VALService/Set` and `/kuksa.VALService/Get`, which the DATA_BROKER does not
  implement.  The DATA_BROKER responds with `Unimplemented`.

- **Connection retry test:** `TestConnectionRetryFailure` (TS-03-E1) is not affected
  because it verifies behaviour when the DATA_BROKER is completely unreachable.

## Custom Proto vs DATA_BROKER Endpoint Mapping

| Custom proto path | Expected by DATA_BROKER |
|---|---|
| `/kuksa.VALService/Get` | `/kuksa.val.v2.VAL/GetValue` |
| `/kuksa.VALService/Set` | `/kuksa.val.v2.VAL/PublishValue` |
| `/kuksa.VALService/Subscribe` | `/kuksa.val.v2.VAL/Subscribe` |

## Mitigation

To make live integration tests pass, the `GrpcBrokerClient` in `src/broker.rs` should
be updated to use the `kuksa.val.v2.VAL` API.  This requires:

1. Replacing the custom `proto/kuksa/val.proto` with the official `kuksa/val/v2/val.proto`
   from the Eclipse Kuksa repository.
2. Updating `build.rs` to compile the correct proto.
3. Updating `src/broker.rs` to use the v2 service client and message types.

This is deferred to a follow-up task to avoid scope creep in the current session.

## Status

- Integration tests are implemented correctly and compile.
- Tests that require live DATA_BROKER connectivity skip gracefully when the container
  is not running.
- `TestConnectionRetryFailure` (TS-03-E1) passes without any infrastructure.
- The locking-service's internal gRPC proto mismatch is a known issue pending fix.
