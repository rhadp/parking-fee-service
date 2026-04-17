# Erratum 03: Contradiction Between REQ-2.3 and REQ-2.E2

**Spec:** 03_locking_service  
**Date:** 2026-04-17  
**Status:** Known divergence — resolved by design.md Path 5  

## Summary

REQ-2.3 and REQ-2.E2 are contradictory when `command_id` is absent from an
otherwise valid JSON payload.

## Conflicting Requirements

**REQ-2.3** (unconditional):
> WHEN a required field is missing or invalid, THE service SHALL respond with
> status "failed" and reason "invalid_command".

**REQ-2.E2** (conditional):
> IF the payload is valid JSON but missing the `action` field, THEN THE service
> SHALL respond with reason "invalid_command" **if a `command_id` can be extracted**.

For a payload like `{"doors":["driver"]}` (missing both `command_id` and `action`):
- REQ-2.3 requires a response (unconditionally)
- REQ-2.E2 implies no response when `command_id` cannot be extracted

## Resolution

The implementation follows **design.md Path 5** and REQ-2.E2:

1. `parse_command` classifies all serde data errors (missing/invalid fields) as
   `CommandError::InvalidCommand`.
2. In `main.rs`, when `parse_command` returns `Err(InvalidCommand)`, the service
   calls `extract_command_id(raw_json)` to attempt best-effort extraction.
3. **If `command_id` is present** → publish `{"status":"failed","reason":"invalid_command"}`.
4. **If `command_id` is absent** → discard without publishing a response.

REQ-2.E2 is treated as a narrowing exception to REQ-2.3: a response is only
published when there is a `command_id` to echo back. Without `command_id` the
response would be unparseable by CLOUD_GATEWAY_CLIENT anyway.

## Impact on Tests

- TS-03-4 (`test_parse_missing_command_id`) only verifies `parse_command` returns
  `Err` — it does not assert that a response is published, leaving the `main.rs`
  dispatch behavior to integration tests.
- No test currently covers the "valid JSON, no command_id, no action" case;
  that edge is covered by the property test TS-03-P1 via arbitrary string inputs.
