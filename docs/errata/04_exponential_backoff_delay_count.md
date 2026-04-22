# Errata: Spec 04 — Exponential Backoff Delay Count

## Issue

REQ-2.2 specifies "exponential backoff (1s, 2s, 4s) for up to 5 attempts",
listing only 3 delay values. Five connection attempts produce 4 inter-attempt
gaps, so the requirement is internally inconsistent.

TS-04-15 independently specifies attempts at t=0, t~1s, t~3s, t~7s, t~15s,
implying delays of 1s, 2s, 4s, and 8s (four delays). The fourth delay of 8s
is absent from REQ-2.2.

## Resolution

The implementation follows the exponential backoff *pattern* (doubling delays)
for all 4 inter-attempt gaps: 1s, 2s, 4s, 8s. This is consistent with
TS-04-15 and the stated "exponential backoff" intent of REQ-2.2. The
requirement text listing only three values is treated as an incomplete
enumeration rather than a cap.

Summary: 5 attempts total, 4 delays: 1s, 2s, 4s, 8s.
