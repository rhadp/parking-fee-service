# Errata: TS-03-E2 — Subscription Stream Interruption Not Unit-Tested

**Spec:** 03_locking_service  
**Requirement:** 03-REQ-1.E2  
**Test Spec Entry:** TS-03-E2  
**Status:** Open — no automated test; behaviour is implemented but not verified in CI

## Summary

TS-03-E2 requires verifying that LOCKING_SERVICE logs a resubscribe warning and
attempts to resubscribe when the subscription stream to DATA_BROKER is interrupted
(e.g. DATA_BROKER is restarted while the service is running).

This test spec entry has no corresponding automated test because:

1. **Integration infrastructure gap**: The test requires starting LOCKING_SERVICE
   connected to a live DATA_BROKER, then restarting DATA_BROKER while the service
   is running, and then inspecting the service logs for a resubscribe warning. This
   requires Podman, a running DATA_BROKER container, and orchestration of a container
   restart mid-test — complexity beyond the current integration test harness.

2. **kuksa.val.v1/v2 API compatibility**: The locking-service uses the `kuksa.val.v1`
   gRPC API while the DATA_BROKER image (0.5.0) only exposes `kuksa.val.v2`. This
   prevents the service from ever reaching ready state against the real broker.
   See `docs/errata/03_locking_service_proto_compat.md`.

## Implementation Status

The resubscription logic is implemented in `rhivos/locking-service/src/main.rs`:
- On subscription stream error, the service logs a warning and retries subscription
  up to 5 attempts before exiting with a non-zero exit code.
- The retry loop includes logging at the `warn!` level.

## Affected Test Spec Entry

| Entry | Expected Assertion |
|-------|--------------------|
| TS-03-E2 | `logs_contain("resubscribing")` after DATA_BROKER restart |

## Resolution

To close this gap, one of the following would be needed:

1. **Upgrade locking-service to kuksa.val.v2** (prerequisite for any live integration
   test): See resolution options in `docs/errata/03_locking_service_proto_compat.md`.

2. **Add a mock DATA_BROKER for stream interruption testing**: A lightweight gRPC
   server in the integration test module that sends a stream termination, which
   would allow verifying the resubscription path without a real DATA_BROKER.

## Minor Findings Addressed

The Skeptic review noted (minor finding `1ab0456df59009a2`) that 03-REQ-1.E2 does
not specify the maximum resubscription count. The implementation uses 5 attempts
(matching the connection retry count in `GrpcBrokerClient::connect`). This number
is documented in the code but not in the requirements, which is an underdefinition
in the spec.
