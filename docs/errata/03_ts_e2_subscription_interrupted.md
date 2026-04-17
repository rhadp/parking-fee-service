# Erratum 03: TS-03-E2 — Subscription Stream Interruption Test Not Implemented

**Spec:** 03_locking_service  
**Date:** 2026-04-17  
**Status:** Known gap — no dedicated test; partial coverage via code inspection

## Summary

TS-03-E2 (requirement 03-REQ-1.E2) requires verifying that when the
subscription stream to DATA_BROKER is interrupted, the LOCKING_SERVICE
attempts to resubscribe up to a maximum number of attempts before exiting.
No dedicated test exists for this behavior.

## Affected Test Spec Entry

**TS-03-E2:**
> Verify the service attempts to resubscribe when the subscription stream is
> interrupted. Expected: Service logs a resubscribe warning.

## Requirement

**03-REQ-1.E2:**
> IF the subscription stream is interrupted, THEN THE service SHALL attempt
> to resubscribe up to a maximum number of attempts before exiting.

## Reason for Gap

### Test Implementation Complexity

Implementing TS-03-E2 as a reliable integration test requires:
1. Starting DATA_BROKER and LOCKING_SERVICE
2. Waiting for the service to become subscribed
3. Restarting DATA_BROKER mid-operation to interrupt the stream
4. Observing the resubscribe behavior via logs

This requires precise timing control and Docker/Podman lifecycle management
that is fragile in CI environments.

### Proto Compatibility Blocker

The proto compatibility gap documented in
`docs/errata/03_locking_service_proto_compat.md` prevents the service from
reaching the subscribed state with the live DATA_BROKER. Integration tests
that require a running subscribed service currently skip. TS-03-E2 would also
skip for this reason.

### Resubscription Logic Implemented in Code

The resubscription logic is implemented in `rhivos/locking-service/src/main.rs`
in the `run_service` function. The subscription loop:
1. Calls `broker.subscribe(SIGNAL_COMMAND)` 
2. On stream error or end, logs a warning and retries (up to `MAX_SUBSCRIBE_RETRIES = 3`)
3. After exhausting retries, exits with a non-zero code

This can be verified via code inspection rather than a live test.

## Impact

| Item | Status |
|------|--------|
| 03-REQ-1.E2 | Implemented in code; not covered by automated test |
| TS-03-E2 | Not implemented |

## Resolution Path

To add test coverage for TS-03-E2:
1. Resolve the kuksa.val.v1 vs v2 proto compatibility gap (see
   `docs/errata/03_locking_service_proto_compat.md`)
2. Add a `TestSubscriptionStreamInterrupted` integration test that:
   - Starts the service against a compatible DATA_BROKER
   - Waits for the "locking-service ready" log line
   - Restarts the DATA_BROKER container
   - Asserts that the service logs contain "resubscribing"
