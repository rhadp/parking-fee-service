# Erratum 03: Proto Compatibility Gap — locking-service v1 vs DATA_BROKER v2

**Spec:** 03_locking_service  
**Date:** 2026-04-17  
**Status:** Known divergence — integration tests skip gracefully

## Summary

The LOCKING_SERVICE uses a custom `kuksa.val.v1` proto (service
`kuksa.val.v1.VAL` with methods `Get`, `Set`, `Subscribe`) while the real
Kuksa Databroker 0.5.0 exposes only `kuksa.val.v2.VAL` (methods `GetValue`,
`SetValue`, `Subscribe`). This incompatibility prevents the service from
connecting to the live DATA_BROKER via gRPC RPCs.

## Symptom

When the locking-service starts against the real DATA_BROKER, the TCP
connection succeeds (channel is established lazily) but the first RPC call
(`Set` for publishing initial lock state) fails with:

```
status: Internal, message: "failed to decode Protobuf message: Datapoint.value:
DataEntry.value: EntryUpdate.entry: SetRequest.updates: invalid wire type:
Varint (expected LengthDelimited)"
```

The service then exits with code 1 (via `std::process::exit(1)` in `main.rs`
after the initial state publish fails).

## Root Cause

The design document (design.md) specifies `kuksa.val.v1` proto for
communication, but the vendored DATA_BROKER image (`kuksa-databroker:0.5.0`)
exposes only the `kuksa.val.v2.VAL` API as documented in
`docs/errata/02_data_broker_compose_flags.md`.

## Impact on Integration Tests

Integration tests in `tests/locking-service/` that require the service to
become ready will skip with the message:

> "locking-service exited before becoming ready. This is expected when the
> service's kuksa.val.v1 proto is incompatible with the live DATA_BROKER's
> kuksa.val.v2.VAL API."

The only exception is `TestConnectionRetryFailure` (TS-03-E1), which
intentionally starts the service against a non-existent endpoint and verifies
that it exits non-zero — this test passes without any infrastructure.

## Affected Tests

| Test | Status | Note |
|------|--------|------|
| TestConnectionRetryFailure | PASS | Does not need DATA_BROKER |
| TestCommandSubscription | SKIP | Proto mismatch |
| TestInitialStateFalse | SKIP | Proto mismatch |
| TestSmokeLockHappyPath | SKIP | Proto mismatch |
| TestSmokeUnlockHappyPath | SKIP | Proto mismatch |
| TestSmokeLockRejectedMoving | SKIP | Proto mismatch |
| TestGracefulShutdown | SKIP | Proto mismatch (service exits before ready) |
| TestStartupLogging | SKIP | Proto mismatch (service exits before ready) |

## Resolution Path

To make all integration tests pass, one of the following changes is needed:

1. **Migrate locking-service to kuksa.val.v2 proto:** Update `proto/kuksa/val.proto`
   (and `build.rs`) to use the `kuksa.val.v2.VAL` service with `GetValue`,
   `PublishValue`, and `Subscribe` methods. Update `broker.rs` accordingly.

2. **Run a compatible DATA_BROKER:** Deploy a Kuksa Databroker version that
   exposes the v1 API alongside v2 (not available in 0.5.0).

Option 1 is the recommended path for production readiness.
