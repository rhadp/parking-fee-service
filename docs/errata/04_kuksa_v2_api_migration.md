# Errata: Spec 04 — Kuksa v2 API Migration

## Issue

The design.md specifies using `tonic` for gRPC communication with
DATA_BROKER but does not mandate a specific kuksa API version. The initial
implementation used the kuksa.val.v1 API (`VAL.Set`, `VAL.Get`,
`VAL.Subscribe`).

Testing against kuksa-databroker 0.5.0 revealed two problems:

1. **v1 `Set` RPC is non-functional.** The v1 `Set` call returns success
   (empty response) but does not actually update the signal value. Confirmed
   via `grpcurl`: after `Set`, a subsequent `Get` returns the old value.

2. **Vendored v1 proto field numbers were wrong.** The original vendored
   `val.proto` used field numbers starting at 10 for the `Datapoint.value`
   oneof, while kuksa-databroker 0.5.0 uses field numbers starting at 11.
   This caused silent data loss on writes (unknown field ignored by server)
   and decode errors on reads ("invalid wire type: Varint, expected
   LengthDelimited").

## Resolution

The implementation was migrated to the kuksa.val.v2 API:

- **Writes** use `VAL.PublishValue` (v2) instead of `VAL.Set` (v1).
- **Subscriptions** use `VAL.Subscribe` (v2) instead of `VAL.Subscribe` (v1).
  The v2 subscribe response uses `map<string, Datapoint>` (keyed by signal
  path) instead of the v1 `repeated DataEntry`.
- A minimal v2 proto (`proto/kuksa/val/v2/val.proto`) was vendored with
  only the `PublishValue` and `Subscribe` RPCs.
- The v1 proto was corrected (field numbers, timestamp type) and retained
  for type re-exports but is no longer used for RPC calls.

The v2 field numbers (string=11, bool=12, ..., double=18) match
kuksa-databroker 0.5.0's wire format, and `PublishValue` correctly updates
signal values.

## Impact

- `broker_client.rs`: rewired from v1 to v2 client
- `integration_tests.rs`: all helpers switched to v2 API
- `build.rs`: compiles both v1 and v2 protos
- `Cargo.toml`: added `prost-types` dependency for `google.protobuf.Timestamp`
