# Errata: Store Timeout Race Guard

**Spec:** 06_cloud_gateway
**Requirement:** 06-REQ-1.3
**Severity:** Critical (from skeptic review)

## Issue

The design specifies that `StoreResponse` cancels the timeout timer, but does
not specify that the timer goroutine must guard against overwriting a real
response. With `time.AfterFunc`, if the timer fires (goroutine enqueued) before
`StoreResponse` calls `Stop()`, `Stop()` returns false and the goroutine will
acquire the mutex after `StoreResponse` releases it, potentially overwriting
the real response with `{status:"timeout"}`.

## Implementation Decision

The `StartTimeout` timer callback checks `if _, exists := s.responses[commandID]; !exists`
before writing the timeout entry. This guard prevents overwriting a real
response that arrived while the timer goroutine was waiting for the lock.

Additionally, `StartTimeout` holds the mutex while both creating the timer
via `time.AfterFunc` and registering it in the `timers` map. This ensures:

1. The timer goroutine cannot run until after the lock is released (since it
   also acquires the same lock), eliminating the window where `StoreResponse`
   couldn't find the timer to cancel it.
2. No stale timer references remain in the map from timers that fire before
   registration.

These two mechanisms together satisfy Property 6 (stored status SHALL NOT be
"timeout" when a real response arrives before the timeout) under all
interleaving schedules.
