# Errata: TS-03-E2 — Subscription Stream Interruption

**Spec:** 03_locking_service  
**Requirement:** 03-REQ-1.E2  
**Test Spec Entry:** TS-03-E2  
**Status:** Resolved — resubscription logic implemented and verified via mock broker test

## Summary

TS-03-E2 requires verifying that LOCKING_SERVICE logs a resubscribe warning and
attempts to resubscribe when the subscription stream to DATA_BROKER is interrupted
(e.g. DATA_BROKER is restarted while the service is running).

This test spec entry is now covered by `TestSubscriptionStreamInterrupted` in
`tests/locking-service/integration_test.go`. The test uses a mock kuksa.val.v1
gRPC server (no real DATA_BROKER container required) that terminates the subscribe
stream on demand, triggering the service's resubscription logic.

## Implementation Status

The resubscription logic is implemented in `rhivos/locking-service/src/main.rs`:

- When the subscription stream ends (`receiver.recv()` returns `None`), the service
  calls `resubscribe()` which attempts up to 3 resubscription attempts with
  exponential backoff delays of 1s, 2s, 4s.
- Each attempt logs a warning containing the word "Resubscribing" at the `warn!` level.
- On successful resubscription, the new receiver replaces the old one and the main
  loop continues processing commands.
- If all 3 attempts fail, the service exits with a non-zero exit code.

## Test Coverage

`TestSubscriptionStreamInterrupted` (TS-03-E2) verifies:

1. The service reaches "locking-service ready" state against a mock v1 broker.
2. When the mock terminates the subscribe stream, the service logs "Resubscribing".
3. The service issues a second Subscribe RPC call (resubscription).
4. The service logs "Resubscribed" confirming successful resubscription.
5. The service remains running after resubscription (does not exit).

The mock broker (`mockV1Broker` in `tests/locking-service/helpers_test.go`) implements
the kuksa.val.v1.VALService gRPC interface using pre-generated Go protobuf code
in `tests/locking-service/pb/kuksa/`.

## Spec Underdefinition

03-REQ-1.E2 states the service SHALL attempt to resubscribe "up to a maximum number
of attempts" without specifying the actual maximum. The implementation uses 3 attempts
(distinct from the connection retry count of 5 in `GrpcBrokerClient::connect`). This
number is documented in the code constants `MAX_RESUBSCRIBE_ATTEMPTS` and
`RESUBSCRIBE_DELAYS_MS` but is not defined by the requirements.
