# Erratum: kuksa.val.v2 API (Spec 09)

## Divergence

The requirements (09-REQ-1.1, 09-REQ-2.1, 09-REQ-3.1, 09-REQ-10.1, 09-REQ-10.2)
and design document reference `kuksa.val.v1` package and `Set` RPC for publishing
VSS signal values to DATA_BROKER.

## Actual Codebase

The vendored proto (`proto/kuksa/val.proto`) uses package `kuksa.val.v2` and the
RPC is named `PublishValue` (not `Set`). This is consistent with the Kuksa VAL v2
API already used by other crates in the workspace (e.g., `cloud-gateway-client`).

## Resolution

The mock-sensors implementation uses `kuksa.val.v2` and `PublishValue` RPC to
match the actual proto definition and existing codebase conventions. The VSS
signal path semantics and DatapointValue types remain unchanged.

## Also

The door sensor VSS path uses `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` as
specified in 09-REQ-3.1, correcting the task group 1 stub which used
`Vehicle.Cabin.Door.Row1.Left.IsOpen`.
