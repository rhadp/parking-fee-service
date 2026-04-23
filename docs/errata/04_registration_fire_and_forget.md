# Erratum: Registration Fire-and-Forget vs Startup Determinism

**Spec:** 04_cloud_gateway_client
**Requirements:** [04-REQ-4.2], [04-REQ-9.2]
**Date:** 2026-04-23

## Issue

REQ-4.2 mandates that registration is "fire-and-forget; the system SHALL NOT
wait for an acknowledgment." REQ-9.2 mandates that "WHEN any step in the
startup sequence fails, the system SHALL log the failure and exit with code 1
without proceeding to subsequent steps." Step 4 of the startup sequence
(REQ-9.1) is self-registration.

These requirements are irreconcilable: a publish failure at step 4 would need
to simultaneously cause exit-with-code-1 (REQ-9.2) and be silently tolerated
(REQ-4.2).

## Resolution

The implementation follows REQ-4.2 (fire-and-forget). Registration publish
failures are logged at WARN level and the service continues to step 5
(command/telemetry processing). This is the safer interpretation because:

1. NATS `publish` is inherently fire-and-forget (no acknowledgment in core
   NATS), so a publish call succeeding only means the client accepted the
   message locally — not that the server received it.
2. Stopping the entire service because a non-critical status announcement
   failed would reduce availability unnecessarily.
3. The `publish_registration` method still returns `Result`, so local errors
   (e.g. disconnected client) are caught and logged.

## Impact

REQ-9.2's "any step" is interpreted as "any step whose failure is
unrecoverable." Registration failure is treated as recoverable, consistent
with REQ-4.2's fire-and-forget semantics.
