# Errata: CLOUD_GATEWAY_CLIENT gRPC API Compatibility

**Related Spec:** 04_cloud_gateway_client
**Date:** 2026-04-17

## Summary

Two issues were discovered during wiring verification (task group 9):

1. The custom proto file (`kuksa.VALService`) was incompatible with the actual
   Eclipse Kuksa Databroker 0.5.0, which exposes `kuksa.val.v2.VAL`.
2. The VSS overlay file used flat dot-notation JSON that the databroker parser
   rejects with `ParseError("children required for type branch")`.

Both issues were corrected so that all integration and smoke tests pass with
real NATS and DATA_BROKER containers.

---

## Errata 1: Custom Proto Incompatible with kuksa.val.v2.VAL

**Affected requirements:** [04-REQ-3.1], [04-REQ-3.2], [04-REQ-3.3], [04-REQ-6.3],
[04-REQ-7.1], [04-REQ-8.1]

**Spec / design text:** The design specifies "gRPC communication using the
kuksa.val.v1 gRPC API" and mentions a `VALService` with `Get`, `Set`,
`Subscribe` RPCs.

**Reality:** `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0` exposes only
`kuksa.val.v2.VAL` with methods `Subscribe`, `Actuate`, `PublishValue`,
`GetValue` (no v1 API at all). The original custom `kuksa.VALService` proto had
no matching service on the server; all RPC calls returned `Unimplemented`.

**Mitigation:** The proto file was replaced with a minimal `kuksa.val.v2` proto
(`proto/kuksa/val/v2/val.proto`) that defines the subset of the v2 API used by
this service:

| Custom proto (removed) | v2 replacement |
|------------------------|----------------|
| `VALService/Get`       | `VAL/GetValue` |
| `VALService/Set`       | `VAL/PublishValue` |
| `VALService/Subscribe` | `VAL/Subscribe` |

`broker_client.rs` was updated to use the generated v2 types.

---

## Errata 2: Actuate vs PublishValue for Command Signals

**Affected requirements:** [04-REQ-6.3]

**Spec / design text:** The design says "write the command payload as-is to
`Vehicle.Command.Door.Lock` in DATA_BROKER via gRPC SetRequest."

**Reality:** In `kuksa.val.v2`, `Vehicle.Command.Door.Lock` is classified as an
`ENTRY_TYPE_ACTUATOR`. Writing to actuators requires a registered actuator
provider via the `OpenProviderStream` RPC. Without a provider (the LOCKING_SERVICE
is not running in the test environment), `VAL/Actuate` returns:
`Status { code: Unavailable, message: "Provider for vss_id ... does not exist" }`.

**Mitigation:** The `write_command` method in `broker_client.rs` uses
`VAL/PublishValue` instead of `VAL/Actuate`. This reflects the cloud-gateway-client's
role as the signal *source* (it publishes the command payload for the LOCKING_SERVICE
to consume via its subscription). `PublishValue` succeeds without a registered
provider. The LOCKING_SERVICE subscribes to the signal and processes it.

This interpretation is consistent with the intent of [04-REQ-6.3]: the payload
is forwarded verbatim and immediately; no actuator handshake is needed.

---

## Errata 3: VSS Overlay File Format

**Affected requirements:** Custom signal registration (Vehicle.Command.Door.Lock,
Vehicle.Command.Door.Response, Vehicle.Parking.SessionActive)

**Spec / design text:** No specific format was documented for the overlay file.

**Reality:** `kuksa-databroker:0.5.0`'s `--vss` flag requires JSON in the
standard VSS tree format, where each branch node must have a `children` key
containing its child nodes. Flat dot-notation (`"Vehicle.Parking": {"type":"branch"}`)
causes `ParseError("children required for type branch")`.

**Mitigation:** `deployments/vss-overlay.json` was rewritten from flat
dot-notation to properly nested VSS tree JSON format. All three custom signals
(`Vehicle.Command.Door.Lock`, `Vehicle.Command.Door.Response`,
`Vehicle.Parking.SessionActive`) are correctly registered after this fix.

---

## Impact on Tests

| Test | Root Cause | Resolution |
|------|-----------|------------|
| TS-04-10 through TS-04-15 | Custom proto incompatible with `kuksa.val.v2.VAL` | Replaced proto |
| TS-04-10, TS-04-14 | `Actuate` fails without registered provider | Use `PublishValue` |
| TS-04-SMOKE-1 | Service exited before smoke window (subscription failed) | Fixed by proto fix |
| All (DATA_BROKER startup) | vss-overlay.json parse error | Fixed overlay format |
