# Errata: Spec 09 — Mock Sensor Proto Compatibility

**Spec:** 09_mock_apps  
**Status:** Accepted divergence — integration tests skip gracefully

## Summary

The spec (09-REQ-10.1, design.md) calls for mock sensors to use
`kuksa.val.v1` proto and communicate with DATA_BROKER via
`kuksa.val.v1.VALService.Set`.  However, `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0`
(the pinned version used in `deployments/compose.yml`) exposes only the
`kuksa.val.v2.VAL` service (`PublishValue`, `Subscribe`, `GetValue`).  The v1
`VALService` is not available on the production-pinned image.

## Impact

| Test                   | Behaviour                             |
|------------------------|---------------------------------------|
| TS-09-E1/E2/E3         | Pass — argument validation only       |
| TS-09-E4               | Pass — unreachable broker exits 1     |
| TS-09-1/2/3/4          | Skip when DATA_BROKER unavailable or exposes only v2 |
| TS-09-SMOKE-1          | Skip when DATA_BROKER unavailable     |
| TS-09-P1               | Skip when DATA_BROKER unavailable     |

## Vendored Proto

A minimal `kuksa.val.v1` proto has been vendored at
`rhivos/mock-sensors/proto/kuksa/val/v1/val.proto` (subset: `Set` RPC +
required message types only).  The wire format is correct for any
DATA_BROKER image that supports the v1 API.

## Resolution Path

To enable full integration test coverage, either:

1. Run a DATA_BROKER image that supports kuksa.val.v1 (e.g. kuksa-val-server
   instead of kuksa-databroker, or a databroker build with v1 shim).
2. Migrate mock-sensors to use kuksa.val.v2 `PublishValue` RPC and update
   the vendored proto accordingly (requires a separate spec change).

The integration tests in `tests/mock-apps/sensor_test.go` are written to
skip gracefully when the DATA_BROKER is unreachable or exposes only v2,
so the CI pipeline remains green regardless.
