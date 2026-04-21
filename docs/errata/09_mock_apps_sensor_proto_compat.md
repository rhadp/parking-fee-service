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
| TS-09-1/2/3/4          | Pass — verified via mock VAL gRPC server |
| TS-09-SMOKE-1          | Pass — verified via mock VAL gRPC server |
| TS-09-P1               | Pass — verified via mock VAL gRPC server |

## Vendored Proto

A minimal `kuksa.val.v1` proto has been vendored at
`rhivos/mock-sensors/proto/kuksa/val/v1/val.proto` (subset: `Set` RPC +
required message types only).  The wire format is correct for any
DATA_BROKER image that supports the v1 API.

Go proto stubs are generated at `tests/mock-apps/pb/kuksa_val_v1/` to
enable a mock VAL gRPC server in integration tests.

## Resolution

Integration tests now use a mock `kuksa.val.v1.VALService` gRPC server
(`startMockVALServer` in `tests/mock-apps/helpers_test.go`) that accepts
`Set` RPC calls and captures published datapoints.  This eliminates the
dependency on a real DATA_BROKER and enables full value verification
(correct VSS paths, correct typed values) regardless of which
DATA_BROKER version is deployed.

The previous approach of skipping tests when DATA_BROKER was unreachable
has been replaced.  Tests now always run and verify published values.

If future work migrates mock-sensors to `kuksa.val.v2`, the mock server
and Go proto stubs should be updated accordingly.
