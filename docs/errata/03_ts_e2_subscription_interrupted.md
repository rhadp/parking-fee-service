# Errata: TS-03-E2 Subscription Stream Interrupted — No Dedicated Test

**Related Spec:** 03_locking_service
**Date:** 2026-04-17

## Situation

The test specification (test_spec.md) includes:

> **TS-03-E2: Subscription Stream Interrupted**
> **Requirement:** 03-REQ-1.E2
> **Type:** integration
> **Description:** Verify the service attempts to resubscribe when the subscription stream
> is interrupted.
>
> **Preconditions:** DATA_BROKER is running. LOCKING_SERVICE is connected and subscribed.
> **Input:** Restart the DATA_BROKER while LOCKING_SERVICE is running.
> **Expected:** Service logs a resubscribe warning.

## Divergence

No dedicated test function for TS-03-E2 was implemented in `tests/locking-service/`.

The tasks.md traceability table records:

> 03-REQ-1.E2 | TS-03-E2 | 3.1, 3.3 | (verified via log inspection in integration tests)

This note was intended to indicate the behaviour is observable via logging when the
databroker is restarted, not that a specific test exists.

## Reason

Implementing TS-03-E2 reliably requires:
1. Starting the databroker container via `podman compose`.
2. Waiting for the locking-service to subscribe.
3. Restarting the databroker (stop + start), which causes the gRPC subscription stream to break.
4. Asserting that the service logs "resubscribing" without crashing.

Step 3 is non-trivial in a Go test helper: it requires the container to fully stop and
restart within a bounded time window while the service is running. Due to the complexity
and the risk of flakiness, task group 4 deferred this test.

## Impact

- 03-REQ-1.E2 is implemented in `src/broker.rs` (the subscription loop retries on
  stream errors) but is not covered by an automated test in this repository.
- All other 03-REQ-1.* requirements are covered by passing tests.

## Resolution

A future spec or test-enhancement session can add `TestSubscriptionInterrupted` that:
1. Uses `podman-compose restart databroker`.
2. Polls the service logs for "resubscrib" within 30 seconds.
3. Verifies the service does not exit non-zero.
