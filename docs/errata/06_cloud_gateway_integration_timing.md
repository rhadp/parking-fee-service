# Errata: 06_cloud_gateway — Integration Test Timing

**Spec:** 06_cloud_gateway  
**Date:** 2026-04-17  
**Severity:** minor

## Issue

`TestStartupLogging` in `backend/cloud-gateway/main_test.go` used a 500ms capture
window to allow the subprocess binary to emit its startup logs. When all packages
run in parallel (e.g., `go test -race ./...`), the binary may be starved of CPU
time and fail to produce output within 500ms, causing intermittent test failures.

No data race is involved. The issue is OS scheduling under concurrent test load.

## Resolution

The capture window in `TestStartupLogging` was increased from 500ms to 2s. This
remains well within the "before NATS connect timeout" safety margin (NATS retry
backoff takes ≥7s without a server), while eliminating false-positive failures
under parallel test execution.

## Related Ambiguity (Skeptic Finding)

Skeptic finding on 06-REQ-5.E1 noted that "5 total attempts" with "1s,2s,4s" backoff
values is ambiguous (only 3 inter-attempt delays enumerated). The implementation uses
exponential backoff capped at 4s:

| Attempt | Delay before next |
|---------|------------------|
| 1       | 1s               |
| 2       | 2s               |
| 3       | 4s               |
| 4       | 4s               |
| 5       | — (last attempt) |

Total elapsed ≥ 11s for 5 attempts. `TS-06-E6` asserts `elapsed >= 7s`, which this
implementation satisfies (it actually takes ~11s). The test assertion is conservative
but not incorrect.
