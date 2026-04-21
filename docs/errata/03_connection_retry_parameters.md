# Errata: Connection Retry Parameter Contradiction

**Spec:** 03_locking_service  
**Requirement:** 03-REQ-1.E1  
**Status:** Resolved — implementation follows the GrpcBrokerClient component table

## Summary

The design document contains contradictory retry parameters for the DATA_BROKER
connection in `GrpcBrokerClient::connect`:

1. **Component table** states: "5 attempts, 1s/2s/4s/8s delays" — 4 delays between
   5 attempts, maximum delay ~15 seconds.
2. **Operational Readiness section** states: "1s, 2s, 4s, 8s, 16s. Maximum startup
   delay before failure is ~31 seconds" — 5 delays implying 6 attempts.

## Implementation Decision

The implementation follows the component table: 5 connection attempts with delays
of 1s, 2s, 4s, 8s between them (4 delays total). The first attempt has no delay.
Maximum startup delay is 1+2+4+8 = 15 seconds, not ~31 seconds.

This was chosen because:
- The component table is the authoritative interface specification
- The Operational Readiness section is descriptive/secondary documentation
- 15 seconds is a more reasonable startup timeout for a safety-critical service

## Code Reference

`rhivos/locking-service/src/broker.rs`, `GrpcBrokerClient::connect`:
```rust
let delays_ms: &[u64] = &[1000, 2000, 4000, 8000]; // 4 delays
for attempt in 0..5usize { ... }                     // 5 attempts
```

## Recommendation

The Operational Readiness section of design.md should be updated to state:
"1s, 2s, 4s, 8s. Maximum startup delay before failure is ~15 seconds."
