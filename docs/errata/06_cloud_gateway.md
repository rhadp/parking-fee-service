# Errata: 06_cloud_gateway

Spec divergences and ambiguities discovered during implementation and review.

---

## E1 — NATS Retry Backoff: 3-delay spec vs 4-delay implementation

**Source:** Skeptic major finding `e4f983cec8fc92f2`
**Affects:** [06-REQ-5.E1], TS-06-E6

**Problem:**
REQ-5.E1 states "retry with exponential backoff (1s, 2s, 4s) up to 5 attempts" but the
parenthetical lists only 3 delay intervals, which corresponds to 4 total connection attempts
(attempt→1s→attempt→2s→attempt→4s→attempt). TS-06-E6 asserts `elapsed >= 7s` (1+2+4=7s) yet
also passes `maxRetries=5`, which produces 5 total attempts and 4 inter-attempt waits
(1+2+4+8=15s). The test assertion is a lower bound, so both interpretations pass.

**Resolution:**
Implement 5 total connection attempts (`maxRetries=5` means `for attempt := 1; attempt <= 5`)
with 4 inter-attempt exponential backoff waits: 1s, 2s, 4s, 8s = 15s total. The "1s, 2s, 4s"
list in REQ-5.E1 is treated as the first three intervals of a standard doubling sequence; the
fourth (8s) is implied by the pattern. This is consistent with the identically worded
requirement in spec 04 (REQ-2.2), resolved the same way
(see `docs/errata/04_cloud_gateway_client.md` E1).

The test retains `>= 7s` as the lower bound per TS-06-E6, with an added upper bound of `< 30s`
to prevent regression from wildly different retry strategies.

---

## E2 — Semantically Invalid Configuration Not Rejected

**Source:** Skeptic major finding `f605a882669795de`
**Affects:** [06-REQ-6.E1]

**Problem:**
REQ-6.E1 only mandates exit on "file does not exist" or "invalid JSON". An empty JSON object
`{}` is syntactically valid and parses successfully in Go, producing zero-values: `port=0`,
`nats_url=""`, `tokens=[]`. The service would attempt to bind on port 0 and connect to NATS with
an empty URL, yielding confusing runtime failures rather than a clear startup error. The spec is
silent on minimum required fields, valid port ranges, non-empty tokens list, or non-empty
`nats_url`.

**Resolution:**
No validation beyond JSON syntax is added. The implementation follows REQ-6.E1 literally:
reject missing files and invalid JSON, accept everything else. In practice, an empty `nats_url`
causes `natsclient.Connect` to fail after 5 retry attempts, and `port=0` causes the OS to assign
a random port (which is harmless, if confusing). This is acceptable for a demo-grade service.
A future spec revision could add semantic validation requirements.
