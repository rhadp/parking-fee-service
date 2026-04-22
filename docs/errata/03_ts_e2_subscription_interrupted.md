# Errata: Spec 03 — TS-03-E2 Subscription Stream Interrupted

## Issue

TS-03-E2 specifies an integration test that restarts the DATA_BROKER while
the LOCKING_SERVICE is running, then asserts the service logs a "resubscribing"
message. Requirement 03-REQ-1.E2 mandates the service "attempt to resubscribe
up to a maximum number of attempts before exiting", but never specifies the
maximum number.

There are two problems:

1. **Unspecified maximum:** The requirements, design, and test spec do not
   define the maximum number of resubscribe attempts. The implementation uses
   5 attempts (matching the connection retry count) as a reasonable default,
   but this cannot be verified against a requirement.

2. **Infrastructure fragility:** The test requires restarting the DATA_BROKER
   container while the service is running and observing resubscription behavior.
   This is inherently race-prone in CI environments and difficult to make
   deterministic.

## Resolution

No dedicated test exists for TS-03-E2. The resubscription logic is implemented
in `broker.rs` (`try_resubscribe` method with `MAX_RESUBSCRIBE_ATTEMPTS = 5`).
Code inspection confirms:

- On stream end or error, `subscription_loop` calls `try_resubscribe`.
- `try_resubscribe` increments the attempt counter, applies exponential backoff,
  and reattempts the gRPC subscribe call.
- After `MAX_RESUBSCRIBE_ATTEMPTS` (5) failures, it logs an error and returns
  `false`, causing the subscription loop to exit.
- The main loop detects the closed channel and exits with code 1.

The requirement is verified by code inspection rather than an automated test.
