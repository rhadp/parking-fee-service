# Errata: 03_locking_service — kuksa.val.v1 vs. kuksa.val.v2 API Compatibility

**Spec:** 03_locking_service  
**Status:** Open — integration tests skip gracefully; no code change required until broker is upgraded

## Summary

The LOCKING_SERVICE uses the `kuksa.val.v1.VALService` gRPC API (methods: `Get`, `Set`,
`Subscribe`) to communicate with DATA_BROKER. The production DATA_BROKER image
(`ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0`) only exposes the `kuksa.val.v2.VAL`
gRPC API (methods: `GetValue`, `PublishValue`, `Subscribe`). These are incompatible wire
protocols — the service cannot subscribe or set/get signals against the real broker.

## Observed Behaviour

When the locking-service starts against kuksa-databroker 0.5.0:

1. **TCP connection succeeds** — tonic's `connect()` establishes a TCP connection.
2. **`set_bool(IsLocked, false)` fails** — gRPC returns "unknown service kuksa.val.v1.VALService";
   the error is logged but the service continues.
3. **`subscribe(Vehicle.Command.Door.Lock)` fails** — same error; the service logs it and exits
   with code 1.

The service never logs "locking-service ready".

## Affected Test Spec Entries

| Test | Behaviour |
|------|-----------|
| TS-03-1 (TestCommandSubscription) | Skips — service does not reach ready state |
| TS-03-13 (TestInitialState) | Skips — service does not reach ready state |
| TS-03-SMOKE-1 (TestSmokeLockHappyPath) | Skips — service does not reach ready state |
| TS-03-SMOKE-2 (TestSmokeUnlockHappyPath) | Skips — service does not reach ready state |
| TS-03-SMOKE-3 (TestSmokeLockRejectedMoving) | Skips — service does not reach ready state |
| TestGracefulShutdown | Skips — service does not reach ready state |
| TS-03-E1 (TestConnectionRetryFailure) | **Passes** — no infrastructure required |
| TestStartupLogging | **Passes** — log appears before connection attempt |

## Root Cause

The spec (design.md technology stack) lists tonic 0.11 and the kuksa.val.v1 proto, which was
the stable API at spec authoring time. The DATA_BROKER was upgraded to 0.5.0 (v2 API) as part
of spec 02_data_broker. The locking-service spec was not updated to reflect the API migration.

## Resolution Options

1. **Migrate locking-service to kuksa.val.v2.VAL** (preferred for production use):  
   Replace the vendored `proto/kuksa/val/v1/val.proto` with the v2 proto from the cloud-gateway-client
   crate, update `broker.rs` to use `GetValue`/`PublishValue`/`Subscribe` (v2 method names), and
   update `build.rs` accordingly. This mirrors the approach taken in spec 04 task group 9.

2. **Run tests against a v1-compatible broker** (alternative for testing only):  
   Use an older kuksa-databroker image (≤ 0.4.x) that exposes the v1 API. Update
   `deployments/compose.yml` to pin to that image.

## Similar Errata

See `docs/errata/09_mock_apps_sensor_proto_compat.md` for the same issue affecting mock sensor
binaries, which also use the kuksa.val.v1 API.
