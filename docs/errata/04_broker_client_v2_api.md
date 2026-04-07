# Erratum: cloud-gateway-client broker_client uses Kuksa v2 gRPC API

**Spec:** 04_cloud_gateway_client
**Date:** 2026-04-07

## Deviation

The design document specifies that `broker_client.rs` uses a simplified
`kuksa.VAL` gRPC service with `Get`, `Set`, and `Subscribe` RPCs operating
on `DataEntry` messages containing a `Datapoint` with an inline `oneof value`.

The actual implementation uses the **Kuksa Databroker v2 API**
(`kuksa.val.v2.VAL`) which exposes different RPCs:

| Design (v1)       | Implementation (v2)       |
|--------------------|---------------------------|
| `Set(SetRequest)`  | `PublishValue(PublishValueRequest)` |
| `Get(GetRequest)`  | `GetValue(GetValueRequest)` |
| `Subscribe`        | `Subscribe` (same name, different message format) |

Key message differences:
- v1 `DataEntry { path, value: Datapoint { timestamp: i64, value: oneof } }`
- v2 `SignalID { path }` + `Datapoint { timestamp: Timestamp, value: Value { typed_value: oneof } }`
- v2 `SubscribeRequest` uses `signal_paths` (not `paths`)
- v2 `SubscribeResponse` uses `map<string, Datapoint>` (not `repeated DataEntry`)

## Reason

Eclipse Kuksa Databroker 0.5.0 only implements the v2 API. The v1 RPCs
return `Unimplemented` errors. This was discovered during wiring verification
(task group 9) when the service connected successfully but all Subscribe/Set
calls failed.

## Impact

- `broker_client.rs` rewritten to use `kuksa.val.v2` proto and API.
- `build.rs` updated to compile the v2 proto from a local copy.
- `prost-types` added as a dependency for `google.protobuf.Timestamp`.
- Integration test helpers updated to use v2 API (`PublishValue`, `GetValue`).
- Integration tests updated to handle v2 Subscribe's behavior of delivering
  current values immediately upon subscription.

## Proto file

The cloud-gateway-client now maintains its own copy of the v2 proto at
`rhivos/cloud-gateway-client/proto/kuksa/val.proto` to decouple from the
shared proto at `proto/kuksa/val.proto` which may be used by other components.
