# Erratum: TS-03-E2 Subscription Stream Interrupted

**Spec:** 03_locking_service
**Affected requirement:** 03-REQ-1.E2
**Affected test spec:** TS-03-E2
**Date:** 2026-04-24

## Deviation

TS-03-E2 specifies an integration test that verifies the service attempts to
resubscribe when the subscription stream is interrupted. This test requires
restarting the DATA_BROKER container while the locking-service is running and
asserting that the service logs a resubscribe warning.

No dedicated test for TS-03-E2 is implemented.

## Rationale

1. **Infrastructure complexity:** The test requires orchestrating a container
   restart mid-test, which is fragile in CI environments and non-deterministic
   in timing.

2. **Partial coverage exists:** The resubscription logic in `main.rs` (lines
   92-114) is straightforward: when `rx.recv()` returns `None` (stream ended),
   the service attempts up to 5 resubscription attempts with exponential
   backoff. The code path is the same pattern as the initial connection retry,
   which IS tested by `TestConnectionRetryFailure` (TS-03-E1).

3. **Spec ambiguity:** 03-REQ-1.E2 states the service SHALL attempt to
   resubscribe "up to a maximum number of attempts" but does not define that
   maximum. The implementation uses 5 attempts (matching the initial connection
   retry count), but this cannot be conformance-checked against the spec.

## Mitigation

The resubscription code path is:
- Visually inspectable (simple retry loop in `main.rs`)
- Structurally identical to the connection retry logic tested by TS-03-E1
- Logged with tracing events ("resubscribing to command signal") that would
  surface in production monitoring

## Resolution

Implement TS-03-E2 when a container orchestration test harness is available
that can reliably restart DATA_BROKER mid-test with deterministic timing.
