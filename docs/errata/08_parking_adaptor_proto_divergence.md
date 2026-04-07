# Errata: Parking Adaptor Proto Divergence

**Spec:** 08 (PARKING_OPERATOR_ADAPTOR)
**Related spec:** 01 (Project Setup, group 6 proto definitions)

## Issue

The existing `proto/adapter/adapter_service.proto` (from spec 01) does not match
the gRPC interface required by spec 08. Specific mismatches:

1. **StartSessionRequest**: Spec 01 includes `vehicle_id` and `zone_id` fields.
   Spec 08 REQ-1.2 says `StartSession(zone_id)` only — `vehicle_id` comes from
   the `VEHICLE_ID` environment variable, not the gRPC request.

2. **StopSessionRequest**: Spec 01 includes a `session_id` field. Spec 08
   REQ-1.3 says `StopSession()` takes no parameters — `session_id` is taken from
   in-memory state.

3. **Response messages**: Spec 01 defines `StartSessionResponse` with
   `active`/`start_time`/`zone_id` fields but no `status` or `rate`. Spec 08
   requires `session_id`, `status`, and `rate` in the start response, and
   `session_id`, `status`, `duration_seconds`, `total_amount`, `currency` in the
   stop response.

4. **GetStatusRequest/GetRateRequest**: Spec 01 requires `session_id` and
   `operator_id` parameters. Spec 08 requires no parameters (state is in memory).

5. **ParkingRate**: Spec 01 includes `operator_id` (field 1). Spec 08's Rate has
   `rate_type`, `amount`, `currency` with no `operator_id`.

## Resolution

A local proto file was created at
`rhivos/parking-operator-adaptor/proto/parking_adaptor.proto` that matches spec
08's requirements. This proto is compiled by the crate's `build.rs` and used for
the adaptor's gRPC service definition.

The original `proto/adapter/adapter_service.proto` is left unchanged. A future
reconciliation should update the shared proto to match the actual service
interface or maintain both as separate service definitions.
