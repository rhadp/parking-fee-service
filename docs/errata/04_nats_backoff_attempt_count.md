# Erratum 04: NATS Connection Retry — Backoff Delays vs. Attempt Count

**Spec:** `04_cloud_gateway_client`
**Requirements:** [04-REQ-2.2], [04-REQ-2.E1]
**Test spec:** TS-04-15
**Severity:** Major (inconsistency between requirements and integration test)

## Problem

[04-REQ-2.2] and the design's startup-path comment both list three explicit
backoff delays: **1 s, 2 s, 4 s** — which cover exactly four total attempts
(one initial attempt + three retries).

However, the same requirement says "**up to 5 attempts**", and the integration
test TS-04-15 expects timestamps **t=0, t≈1 s, t≈3 s, t≈7 s, t≈15 s** — which
requires four delay intervals (1 s, 2 s, 4 s, 8 s) for a total of **five**
attempts.

The three artefacts are mutually inconsistent:

| Artefact | Implied delays | Implied attempt count |
|---|---|---|
| REQ-2.2 ("1s, 2s, 4s") | 1 s, 2 s, 4 s | 4 |
| REQ-2.2 ("up to 5 attempts") | unspecified | 5 |
| TS-04-15 (timestamps) | 1 s, 2 s, 4 s, 8 s | 5 |

## Resolution

The implementation (task group 5) will follow **TS-04-15** as the authoritative
test contract, using four retry intervals **1 s, 2 s, 4 s, 8 s** for a maximum
of **five total attempts** (one initial + four retries).  This satisfies:

- "up to 5 attempts" from REQ-2.2
- the timestamp expectations in TS-04-15
- the spirit of exponential backoff (doubling each interval)

The listed delays "1s, 2s, 4s" in REQ-2.2 are treated as non-exhaustive
examples; the fourth retry interval (8 s) is added to reach the mandated
5-attempt ceiling.

## Gap: REQ-7.E1 and REQ-3.E1 have no automated test coverage

The coverage matrix in `test_spec.md` shows **both** [04-REQ-7.E1] and
[04-REQ-3.E1] with `-` in both the unit-test and integration-test columns.
These are important safety properties (non-JSON broker response must not be
relayed; DATA_BROKER connection failure must cause exit-1) but have no
corresponding test spec entries.

Task group 8 implementers should add integration tests for these paths.
