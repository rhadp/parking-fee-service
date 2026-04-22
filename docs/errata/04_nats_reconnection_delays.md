# Erratum: NATS Reconnection Delay Values

**Spec:** 04_cloud_gateway_client
**Requirement:** [04-REQ-2.2]
**Date:** 2026-04-22

## Issue

[04-REQ-2.2] specifies:

> the system SHALL retry with exponential backoff (1s, 2s, 4s) for up to 5
> attempts.

This lists only 3 delay values. Five connection attempts require 4
inter-attempt delays, so the specification is internally inconsistent.

TS-04-15 (the corresponding test) independently specifies attempts at t=0,
t~1s, t~3s, t~7s, t~15s, which implies delays of 1s, 2s, 4s, and 8s (four
delays). The fourth delay of 8s is absent from [04-REQ-2.2].

## Resolution

The implementation follows the exponential backoff formula `2^(n-1)` seconds
for `n` in 1..4, yielding delays of 1s, 2s, 4s, 8s between the 5 connection
attempts. This matches TS-04-15 and the natural continuation of the pattern
listed in [04-REQ-2.2].

The requirement text should be read as specifying the *pattern* (powers of 2
starting at 1s) rather than an exhaustive list. The implementation uses:

```rust
let delay = Duration::from_secs(1 << (attempt - 1)); // 1s, 2s, 4s, 8s
```

## Impact

None. The implementation is correct and consistent with the test
specification. The requirement text is ambiguous but the intent is clear.
