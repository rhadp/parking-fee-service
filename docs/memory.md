# Project Memory

## Architecture

- Integration tests that spawn multiple Tokio tasks subscribing to the same Kuksa signal must run serially; parallel execution causes race conditions where one handler processes another test's command.
- Kuksa Databroker enforces strict VSS signal types: `Vehicle.Speed` is `float` (`f32`), not `double` (`f64`). Using the wrong type produces an `InvalidArgument` gRPC error.

## Conventions

- Integration tests sharing infrastructure (Kuksa) use the `serial_test` crate with `#[serial]` to enforce sequential execution.
- Mock-sensors CLI arguments use exact VSS type matches (`f32` for speed, `f64` for location coordinates).

## Decisions

- We use `serial_test` (not `--test-threads=1`) because it provides per-test-group serialization without slowing down unrelated unit tests.
- The `SetSpeed` CLI argument uses `f32` (not `f64`) to match the VSS `Vehicle.Speed` datatype exactly, avoiding Kuksa type-mismatch errors.

## Fragile Areas

- Port 55555 (Kuksa default) frequently conflicts with other running instances in multi-worktree setups. Integration tests should use the `DATABROKER_ADDR` env var to support alternative ports.
- Kuksa Databroker fan-out subscriptions deliver to all subscribers; tests that spawn multiple lock handlers produce non-deterministic results unless serialized.

## Failed Approaches

_(none yet)_

## Open Questions

_(none yet)_
