# D1 Timeout/Degradation Reconciliation

**Spec:** 03_vehicle_cloud_connectivity
**Task Group:** 6
**Date:** 2026-02-23

## Summary

The spec tests `TestEdge_CommandTimeout` (TS-03-E5) and
`TestProperty_GracefulDegradation` (TS-03-P7) originally expected behavior
that conflicted with design decision D1. This document describes the
reconciliation.

## Original Test Expectations

- **TS-03-E5 (CommandTimeout):** Expected HTTP 504 when MQTT broker was
  unreachable. The test started the gateway with
  `MQTT_BROKER_URL=tcp://localhost:19999` (non-existent broker) and expected
  the command handler to wait for the tracker timeout then return 504.

- **TS-03-P7 (GracefulDegradation):** Expected an error response (HTTP >= 400)
  when MQTT was unreachable. Failed because the implementation returned 202.

## Actual Behavior (Correct per D1)

Per design decision D1, when the MQTT broker is unreachable:

1. `Publish()` returns an error immediately (broker not connected).
2. The command handler returns **HTTP 202 Accepted** with `{"status": "pending"}`
   immediately (degraded mode per 03-REQ-2.E1).
3. The command is never registered in the tracker, so no timeout occurs.

The 504 timeout (03-REQ-2.E3) applies only when MQTT IS connected but no
vehicle responds within the configured timeout.

## Resolution

### TestEdge_CommandTimeout

Split into two subtests:
- `degraded_mode`: MQTT unreachable -> expects 202 Accepted (per D1)
- `connected_timeout`: MQTT connected, no responder -> expects 504 (per 03-REQ-2.E3; requires Mosquitto)

### TestProperty_GracefulDegradation

Updated to accept 202 as valid graceful degradation. The property's invariant
is that the REST API remains responsive (does not hang or crash), not that it
returns an error code. A 202 with `"pending"` status is a valid graceful
degradation response.
