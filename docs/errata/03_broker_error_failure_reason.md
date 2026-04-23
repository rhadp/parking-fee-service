# Erratum: `broker_error` Failure Reason

**Spec:** 03_locking_service
**Requirements:** 03-REQ-4.1, 03-REQ-4.2, 03-REQ-5.2
**Date:** 2026-04-23

## Context

The original design specifies four failure reasons for command responses:
`vehicle_moving`, `door_open`, `unsupported_door`, `invalid_command`.

The original implementation unconditionally updated the in-memory `lock_state`
after calling `broker.set_bool()`, even when the call failed. This caused the
in-memory state to diverge from DATA_BROKER: the service would believe the door
was locked/unlocked while the broker retained the stale value. Subsequent
commands would hit the idempotent path and return "success" without ever
retrying the failed write.

## Divergence

The implementation now returns `status: "failed"` with `reason: "broker_error"`
when `set_bool` fails, and does **not** update `lock_state`. This is a new
failure reason not listed in the original design's failure reason table.

## Rationale

Returning "success" when the state write actually failed is a safety-critical
misrepresentation for an ASIL-B service. The caller (CLOUD_GATEWAY_CLIENT)
would report success to the COMPANION_APP while the physical lock state
remains unchanged. Preserving state consistency between in-memory tracking
and DATA_BROKER is essential for correct idempotency behavior.

## Impact

- CLOUD_GATEWAY_CLIENT should handle `reason: "broker_error"` as a transient
  failure eligible for retry.
- The response schema gains one additional reason value.
