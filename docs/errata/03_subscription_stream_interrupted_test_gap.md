# Erratum: TS-03-E2 / 03-REQ-1.E2 — Subscription Stream Interrupted Test Gap

## Context

During wiring verification (task group 5, spec 03_locking_service), the
following gap was identified between the test specification and the actual
test suite.

## Gap

**Test Spec Entry:** TS-03-E2 (Subscription Stream Interrupted)
**Requirement:** 03-REQ-1.E2

The test specification defines TS-03-E2 as an integration test that verifies
the service attempts to resubscribe when the subscription stream is
interrupted (e.g., by restarting DATA_BROKER). However, no corresponding test
function exists in the test suite.

The Skeptic review (minor severity) also noted that:

- TS-03-E2 is not assigned to any task group in tasks.md.
- The test's assertion is weak — it only checks for a "resubscribing" log line
  without verifying actual recovery or the maximum retry behavior.

## Implementation Status

The resubscription logic **is implemented** in `main.rs` (the subscription
loop retries on stream interruption). Only the test is missing.

## Rationale for Deferral

This test requires orchestrating a DATA_BROKER container restart mid-test,
which adds significant infrastructure complexity. The resubscription behavior
is a best-effort resilience mechanism rather than a safety-critical path. The
core subscription and connection retry behaviors are covered by TS-03-1 and
TS-03-E1 respectively.

## Recommendation

Add TS-03-E2 as a future integration test when container lifecycle
orchestration tooling is mature enough to reliably restart DATA_BROKER
mid-test.
