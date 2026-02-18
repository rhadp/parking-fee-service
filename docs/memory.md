# Project Memory

## Architecture

- CLOUD_GATEWAY_CLIENT uses trait-based abstraction (`DataBrokerWriter`, `DataBrokerReader`, `LockResultSubscriber`) for all Kuksa interactions; `KuksaAdapter` is the concrete implementation. This enables unit testing without real infrastructure.
- The telemetry publisher reuses `status_handler::read_vehicle_state()` and the `DataBrokerReader` trait for signal reads, avoiding duplicated signal-reading logic.
- Background tasks (result forwarder, telemetry publisher) are spawned via `tokio::spawn` only after Kuksa connects successfully. If Kuksa is unavailable, the MQTT event loop still runs but commands fail gracefully.

## Conventions

- Each module has a top-level doc comment listing the requirements it satisfies (e.g. `//! - 03-REQ-4.1: ...`).
- Tests are inline (`#[cfg(test)] mod tests`) within each source file, not in separate test files.
- Integration tests requiring infrastructure (Mosquitto, Kuksa) use `#[ignore]` and are run manually with `--ignored`.
- `chrono_timestamp()` is a local helper duplicated across modules (mqtt.rs, status_handler.rs, result_forwarder.rs, telemetry.rs) — returns Unix seconds as `i64`.

## Decisions

- Telemetry uses QoS 0 (fire-and-forget) because periodic messages are best-effort; missing one is acceptable since the next arrives shortly.
- The first telemetry interval tick is consumed (skipped) to avoid publishing immediately on startup before signals settle.
- `publish_telemetry_tick()` is exposed as a public function separate from the loop to facilitate unit testing of a single publish cycle.

## Fragile Areas

- `PendingCommandState` (`Arc<Mutex<Option<PendingCommand>>>`) only tracks one pending command at a time. Concurrent lock/unlock commands overwrite the pending state. This is a documented simplification for the single-vehicle demo.
- The worktree CWD may not be the repository root — git and cargo commands must be run from the worktree root, not a subdirectory.

## Failed Approaches

_(No entries yet.)_

## Open Questions

_(No entries yet.)_
