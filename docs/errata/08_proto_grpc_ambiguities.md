# Errata 08: Proto/gRPC Design Ambiguities

**Spec:** 08_parking_operator_adaptor  
**Date:** 2026-04-23  
**Status:** Resolved

## Erratum 1 — GetStatus Rate Field (CRITICAL)

**Requirement:** 08-REQ-1.4  
**Finding:** The canonical `SessionStatus` proto message had fields
`{session_id, active, start_time, zone_id}` but no `rate` field. Requirement
08-REQ-1.4 mandates that `GetStatus` return rate information alongside those
fields. Test TS-08-4 asserts `resp.rate.rate_type == "per_hour"`.

**Resolution:** Added `ParkingRate rate = 5` to `SessionStatus` in both proto
files:
- `proto/adapter/adapter_service.proto` (canonical source)
- `rhivos/parking-operator-adaptor/proto/parking_adaptor/v1/adapter_service.proto`
  (vendored copy for tonic-build)

Updated `grpc_server.rs` `get_status` handler to populate the `rate` field in
the `SessionStatus` response from the in-memory `SessionState.rate`. Returns
`rate: None` when no session is active.

## Erratum 2 — Manual StartSession vehicle_id Handling (MAJOR)

**Requirement:** 08-REQ-1.2, 08-REQ-3.3  
**Finding:** `StartSessionRequest` in the proto definition includes a
`vehicle_id` field alongside `zone_id`. Requirement 08-REQ-1.2 describes
`StartSession(zone_id)` taking only zone_id. Requirement 08-REQ-3.3 specifies
that autonomous starts use `vehicle_id` from the `VEHICLE_ID` env var. The
spec does not say whether the gRPC `vehicle_id` field is used for manual calls.

**Resolution:** The `vehicle_id` field in `StartSessionRequest` is **ignored**
by the adaptor. For both autonomous and manual `StartSession` calls, the
`VEHICLE_ID` environment variable value is always used when calling the
PARKING_OPERATOR. This ensures consistent vehicle identification regardless
of call path. The `vehicle_id` proto field is retained in the schema for
forward compatibility.

## Erratum 3 — Manual StopSession session_id Handling (MAJOR)

**Requirement:** 08-REQ-1.3  
**Finding:** `StopSessionRequest` in the proto definition includes a
`session_id` field (field 1), but 08-REQ-1.3 describes `StopSession()` as
taking no input parameters — the adaptor stops the active session using
internal state. The spec does not specify whether to use, ignore, or validate
the gRPC-provided session_id.

**Resolution:** The `session_id` field in `StopSessionRequest` is **ignored**
by the adaptor. `StopSession` always stops the currently active in-memory
session, identified by internal session state. The `session_id` proto field is
retained in the schema for forward compatibility.

## Erratum 4 — Orphaned OperatorClient.get_status() Design Element (MAJOR)

**Finding:** The design document architecture flowchart shows a
`GET /parking/status/{id}` REST call to the PARKING_OPERATOR, and the module
interfaces section defines `async fn get_status(&self, session_id: &str)` on
`OperatorClient`. Neither appears in any requirement, acceptance criterion, or
execution path.

**Resolution:** The `get_status()` method was **not implemented**. Session
status is served entirely from in-memory state via the `GetStatus` gRPC RPC.
No `GET /parking/status/{id}` REST endpoint is called. The design flowchart
is superseded by this erratum.

## Erratum 5 — TS-08-P6 Property Test Formulation (MAJOR)

**Finding:** TS-08-P6 (Sequential Event Processing) as specified in
`test_spec.md` uses "compare `result_sequential == result_concurrent`" as the
invariant. This is not a valid deterministic invariant because concurrent
delivery order is scheduler-dependent.

**Resolution:** TS-08-P6 is implemented as a **determinism test**: the same
ordered event sequence is processed twice sequentially, and the final session
state is asserted to be identical both times. This correctly validates that
sequential processing produces deterministic outcomes. The serialization
guarantee (at-most-one-in-flight) is an architectural property enforced by
the single-writer event loop channel, not directly testable via proptest.
