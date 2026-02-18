# Project Memory

## Architecture

- The project has three main technology domains: Go backend services (`backend/`), Rust RHIVOS services (`rhivos/`), and Go mock CLI tools (`mock/`).
- The vehicle-to-cloud pipeline has 5 components: COMPANION_APP CLI (Go REST client) → CLOUD_GATEWAY (Go REST+MQTT server) → Mosquitto MQTT broker → CLOUD_GATEWAY_CLIENT (Rust MQTT+gRPC client) → Kuksa DATA_BROKER (gRPC) → LOCKING_SERVICE (Rust gRPC).
- CLOUD_GATEWAY uses `paho.mqtt.golang` with async subscription setup — `SubscribeMultiple` runs in a goroutine, so subscriptions may not be established immediately after connection.
- CLOUD_GATEWAY_CLIENT uses `rumqttc` (async Rust) and publishes a non-retained registration message on every startup via QoS 2.
- CLOUD_GATEWAY_CLIENT uses trait-based abstraction (`DataBrokerWriter`, `DataBrokerReader`, `LockResultSubscriber`) for all Kuksa interactions; `KuksaAdapter` is the concrete implementation, enabling unit testing without real infrastructure.
- The telemetry publisher reuses `status_handler::read_vehicle_state()` and the `DataBrokerReader` trait for signal reads, avoiding duplicated signal-reading logic.
- Background tasks (result forwarder, telemetry publisher) are spawned via `tokio::spawn` only after Kuksa connects successfully. If Kuksa is unavailable, the MQTT event loop still runs but commands fail gracefully.
- Mock services live under `mock/` with each service as a separate Go module (flat structure: `go.mod`, `main.go`, `main_test.go` in package `main`). The mock PARKING_OPERATOR is a standalone HTTP server.
- The `mock/companion-app-cli` is a pure HTTP client — it has no MQTT or Kuksa dependencies, only stdlib `net/http`.

## Conventions

- Protocol and design documentation lives in `docs/` (e.g., `mqtt-protocol.md`, `vehicle-pairing.md`, `vss-signals.md`).
- Each module has a top-level doc comment listing the requirements it satisfies (e.g. `//! - 03-REQ-4.1: ...`).
- Rust tests are inline (`#[cfg(test)] mod tests`) within each source file, not in separate test files.
- Integration tests requiring infrastructure (Mosquitto, Kuksa) use `#[ignore]` and are run manually with `--ignored`.
- `chrono_timestamp()` is a local helper duplicated across modules (mqtt.rs, status_handler.rs, result_forwarder.rs, telemetry.rs) — returns Unix seconds as `i64`.
- Go version is 1.25.7 across all modules. Module paths use the `github.com/rhadp/parking-fee-service/` prefix.
- Go mock CLIs use `flag` package with env var fallbacks via `envOrDefault()`. The companion-app-cli historically used manual flag parsing but the `flag`+`envOrDefault` pattern is now standard.
- Go test files in `mock/` use `httptest.NewRecorder()` and `httptest.NewRequest()` for handler testing (not `httptest.Server`). CLI tools use `httptest.NewServer` to verify HTTP request construction.
- HTTP routing in Go mock servers uses Go 1.22+ method-based patterns (e.g., `"POST /parking/start"`).
- Binary names are added to `.gitignore` to prevent committing build artifacts.
- The `run()` function pattern takes `io.Writer` params for stdout/stderr to enable testability without capturing `os.Stdout`.
- Config is read from env vars first, then overridden by CLI flags; the `envOrDefault` helper is the standard pattern.
- The `config` struct pattern (rather than global variables) is used to avoid state leaking between tests.
- E2E integration tests live in `tests/` at the project root (e.g., `tests/test_cloud_e2e.sh`). Makefile target `test-e2e` runs E2E tests; `test` runs unit tests only.
- The Makefile's `GO_MOCK_DIRS` variable must be updated when adding a new Go mock module.
- Infrastructure (Kuksa + Mosquitto) is managed via `make infra-up` / `make infra-down` using podman/docker compose.
- Integration tests must exit 0 and report "SKIP" when infrastructure is unavailable (03-REQ-7.E1).
- The README "Current Status" table must be kept up-to-date when services move from skeleton to implemented.

## Decisions

- Telemetry uses QoS 0 (fire-and-forget) because periodic messages are best-effort; missing one is acceptable since the next arrives shortly.
- The first telemetry interval tick is consumed (skipped) to avoid publishing immediately on startup before signals settle.
- `publish_telemetry_tick()` is exposed as a public function separate from the loop to facilitate unit testing of a single publish cycle.
- The companion-app-cli uses a `config` struct (not global vars) because global vars caused test pollution when tests ran in parallel or sequence.
- Bearer tokens default to empty string (not "demo-token") because the real flow requires explicit pairing first; token is now required for lock/unlock/status.
- Error responses from the gateway are parsed as JSON `{error, code}` when possible, falling back to `http.StatusText()` for non-JSON responses.
- We use a shell script (not Go/Rust test) for E2E integration tests because the tests orchestrate multiple independently-built services (Go + Rust binaries) and require process lifecycle management.
- JSON parsing in E2E test scripts uses `python3 -c` with `json.load` rather than `jq` because `python3` is universally available and `jq` may not be installed.
- The mock PARKING_OPERATOR uses stdlib `net/http` only (no external router) because Go 1.22+ mux supports method-based routing natively.
- Mock session IDs use a simple monotonic counter (`sess-001`, `sess-002`) because the mock only needs unique IDs for testing, not cryptographic randomness.
- Fee calculation uses `math.Ceil` for per-minute rounding: `rate_amount × ceil(duration_seconds / 60)`.

## Fragile Areas

- `PendingCommandState` (`Arc<Mutex<Option<PendingCommand>>>`) only tracks one pending command at a time. Concurrent lock/unlock commands overwrite the pending state. This is a documented simplification for the single-vehicle demo.
- The worktree CWD may not be the repository root — git and cargo commands must be run from the worktree root, not a subdirectory.
- Tests that use `os.Unsetenv` without `t.Setenv` can leak env state between tests. Use `t.Setenv` (which auto-restores) whenever possible.
- **MQTT registration timing**: The CGC publishes a non-retained registration message at startup. If CLOUD_GATEWAY has not yet subscribed (async subscription), the message is lost. The E2E test mitigates this with a 2s sleep after GW healthz and a polling retry loop (up to 20 retries).
- **Port 55555 on macOS/podman**: The default Kuksa Databroker port (55555) can become stuck due to stale `gvproxy` port forwards in podman's VM networking layer. Workaround: start Kuksa on an alternate port (e.g., 55556) and pass `DATABROKER_ADDR=http://localhost:55556` to the test.
- **Python boolean output in E2E tests**: Python's `json.load` prints booleans as `True`/`False` (capitalized), not JSON `true`/`false`. The `json_get` helper normalizes this to lowercase.

## Failed Approaches

- Using `curl -sf` (silent + fail) for HTTP status checking: the `-f` flag causes curl to exit with error and return "000" for 4xx/5xx responses, masking the actual status code. Must use `-s` without `-f` and read `%{http_code}` separately.
- Using `"${PIDS_TO_KILL[@]}"` with `set -u` (nounset) when the array is empty causes an "unbound variable" error in bash. Must guard with `${#PIDS_TO_KILL[@]} -gt 0` check first.

## Open Questions

_(No entries yet.)_
