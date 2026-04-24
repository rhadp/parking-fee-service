# Errata: 06_cloud_gateway (auto-generated)

**Spec:** 06_cloud_gateway
**Date:** 2026-04-23
**Status:** Active
**Source:** Auto-generated from reviewer blocking findings

## Findings

### Finding 1

**Summary:** [critical] The design specifies that `StoreResponse` cancels the timeout timer, but does not specify that the timer goroutine must guard against overwriting a real response. With `time.AfterFunc`, if the timer fires (goroutine enqueued) before `StoreResponse` calls `Stop()`, `Stop()` returns false and the goroutine will acquire the mutex after `StoreResponse` releases it, then overwrite the real response with `{status:'timeout'}`. Property 6 ('stored status SHALL NOT be timeout' when real response arrives) can be silently violated by any naive implementation. The design must explicitly require the timer goroutine to check whether a real response already exists before writing the timeout entry.
**Requirement:** 06-REQ-1.3
**Task Group:** 1

### Finding 2

**Summary:** [major] The design states the auth middleware must 'Extract VIN from URL path' and 'handle both /vehicles/{vin}/commands and /vehicles/{vin}/commands/{command_id} patterns', but does not specify the extraction mechanism. In Go 1.22 ServeMux, `r.PathValue("vin")` is only populated after the mux routes the request. Unit tests in TS-06-9 invoke `auth.Middleware(cfg)(testHandler)` directly via `httptest.NewRequest` without a ServeMux, so `r.PathValue` returns "" — causing the VIN comparison to always fail with 403, regardless of the token. If the middleware manually parses the URL path, both unit and production contexts work. The design must specify which approach is required.
**Requirement:** 06-REQ-3.2
**Task Group:** 1

### Finding 3

**Summary:** [major] The glossary defines exponential backoff as '(1s, 2s, 4s)' — only three interval values. Five connection attempts require up to four intervals (between attempts 1-2, 2-3, 3-4, 4-5); the fourth interval is never specified. TS-06-E6 asserts only `elapsed >= 7s` (sum of three intervals: 1+2+4), which is consistent with four total attempts — not five. An implementation making only four attempts passes the test while violating the '5 attempts' requirement, making the test non-verifiable as written.
**Requirement:** 06-REQ-5.E1
**Task Group:** 1

### Finding 4

**Summary:** [major] The requirement, design, and tasks are internally contradictory about the number of NATS connection attempts. 06-REQ-5.E1 says 'up to 5 attempts'. The design error-handling table says 'Retry 5x with backoff' (5 retries = 6 total attempts). tasks.md task 5.1 says 'Call natsclient.Connect with ... 5 retries' (also 6 total attempts). The function signature uses the parameter name `maxRetries`. '5 attempts' equals 4 retries, while '5 retries' equals 6 attempts. These produce observably different behavior and the spec is self-contradictory.
**Requirement:** 06-REQ-5.E1
**Task Group:** 1
