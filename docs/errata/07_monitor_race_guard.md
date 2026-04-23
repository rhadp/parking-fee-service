# Errata: Container Monitor Race Guard

**Spec:** 07_update_service
**Requirements:** 07-REQ-9.1, 07-REQ-9.2
**Severity:** Critical (correctness)

## Problem

The original design specified the container monitor as a background task that
calls `podman wait`, then checks the adapter state and transitions accordingly.
The check (is the adapter still RUNNING?) and the transition were two separate
operations, creating a time-of-check-to-time-of-use (TOCTOU) race:

1. `podman wait` returns (container exited).
2. Monitor calls `get_adapter()` — sees adapter is RUNNING.
3. Between steps 2 and 3, `RemoveAdapter` transitions the adapter to STOPPED.
4. Monitor calls `transition()` — emits a spurious STOPPED to STOPPED event
   (or ERROR event on a non-RUNNING adapter).

This race allows the monitor to produce invalid state transitions and
duplicate events, violating the state machine invariant (Property 3).

## Resolution

Added `StateManager::transition_from(adapter_id, expected_state, new_state,
error_msg)` — an atomic compare-and-swap that only transitions if the
adapter's current state matches `expected_state`. The check and transition
happen within a single mutex lock hold, eliminating the TOCTOU window.

The container monitor now calls `transition_from(..., Running, ...)` instead
of the two-step `get_adapter` + `transition` sequence. If `RemoveAdapter` or
the single-adapter stop has already transitioned the adapter away from
RUNNING, `transition_from` returns `Err(InvalidTransition)`, which the
monitor silently ignores.

## Impact

- No change to external behavior when there is no concurrent operation.
- Under concurrent `RemoveAdapter` or `InstallAdapter` (single-adapter stop),
  the monitor no longer produces spurious events or invalid transitions.
- The `install_lock` (tokio::sync::Mutex) in `UpdateServiceImpl` already
  serializes the single-adapter check-stop-create sequence for `InstallAdapter`
  calls, addressing a related atomicity concern.
