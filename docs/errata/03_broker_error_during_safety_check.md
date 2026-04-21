# Errata: 03_locking_service — Broker Error During Safety Check

**Spec:** 03_locking_service
**Requirements:** 03-REQ-3.1, 03-REQ-3.2, 03-REQ-5.2
**Status:** Open — behavior defined by implementation, not by spec

## Summary

03-REQ-5.2 enumerates exactly four failure reasons (`vehicle_moving`, `door_open`,
`unsupported_door`, `invalid_command`) with no provision for internal broker errors.
When `BrokerClient::get_float` or `BrokerClient::get_bool` returns `BrokerError`
during safety check execution for a lock command, the required behavior is undefined
by the spec.

## Implementation Decision

The `check_safety` function in `safety.rs` uses `unwrap_or(None)` on broker errors,
which chains into the existing safe-default logic:

- `get_float` error → treated as `None` → speed defaults to 0.0 (safe)
- `get_bool` error → treated as `None` → door defaults to closed (safe)

This means a broker error during a lock command's safety check will cause the lock
to proceed as if conditions are safe. This aligns with the ASIL-B principle of
defaulting to the safe state, but could allow locking while the vehicle is actually
moving if the speed sensor signal is unreachable.

## Rationale

The safe-default approach is consistent with:
- 03-REQ-3.E1: speed signal with no value treated as 0.0
- 03-REQ-3.E2: door signal with no value treated as closed

Broker errors are analogous to "no value available" from the service's perspective.

## Test Coverage

Three unit tests verify this behavior (added beyond test_spec.md):
- `safety::tests::test_get_float_error_treated_as_safe`
- `safety::tests::test_get_bool_error_treated_as_safe`
- `safety::tests::test_both_broker_errors_treated_as_safe`

## Recommendation

The spec should be updated to either:
1. Explicitly list broker errors as a fifth failure reason (e.g., `internal_error`), or
2. Explicitly state that broker errors during safety checks are treated as safe defaults
