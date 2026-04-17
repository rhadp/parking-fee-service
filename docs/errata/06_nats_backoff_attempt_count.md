# Erratum 06: NATS Connection Retry — Backoff Delays vs. Attempt Count

**Spec:** `06_cloud_gateway`
**Requirements:** [06-REQ-5.E1]
**Test spec:** TS-06-E6
**Severity:** Major (inconsistency between requirements and test assertion)

## Problem

[06-REQ-5.E1] states the service shall retry "with exponential backoff (1s, 2s,
4s) up to 5 attempts". This is ambiguous:

- The listed delays (1s, 2s, 4s) imply **4 total attempts** (one initial +
  three retries with the listed waits).
- "up to 5 attempts" implies **5 total attempts** (one initial + four retries).

The test spec TS-06-E6 asserts:

```
ASSERT elapsed >= 7 * time.Second
```

7 seconds corresponds to the sum of three backoff delays (1+2+4), which fits
**4 total attempts**. However, the requirement text says **5 attempts**.

If the implementation faithfully follows "5 attempts" with delays [1s, 2s, 4s,
8s], the total wait is 15 seconds — which satisfies `elapsed >= 7s` but
exceeds what the delay list implies.

## Resolution

This erratum adopts the same resolution as
[`docs/errata/04_nats_backoff_attempt_count.md`](04_nats_backoff_attempt_count.md):

The implementation (task group 4) will use **five total attempts** (one initial
attempt + four retries) with backoff delays **[1s, 2s, 4s, 8s]**. This:

- Satisfies "up to 5 attempts" from REQ-5.E1.
- Satisfies `elapsed >= 7s` in TS-06-E6 (15s >= 7s).
- Follows the spirit of exponential doubling.

The listed delays "1s, 2s, 4s" in REQ-5.E1 are treated as non-exhaustive
examples; the fourth retry interval (8s) is added to reach the mandated
5-attempt ceiling.

## Test Impact

`TestNATSConnectionRetryExhaustion` (TS-06-E6) asserts `elapsed >= 7s` with
`maxRetries = 5`. With the [1s, 2s, 4s, 8s] implementation this test passes
(15s >= 7s). The `>= 7s` bound is permissive enough to accept either the
4-attempt (7s) or 5-attempt (15s) interpretation.
