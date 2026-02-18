# Project Memory

## Architecture

- The project spans **Go** (backend/cloud services) and **Rust** (RHIVOS vehicle-side services), connected via MQTT through Eclipse Mosquitto.
- `parking-proto` crate provides generated gRPC/protobuf bindings for Kuksa DATA_BROKER; it's shared across all Rust services.
- Rust workspace root is `rhivos/Cargo.toml` with shared dependency versions. New dependencies must be added to `[workspace.dependencies]` first, then referenced by individual crates.
- CLOUD_GATEWAY (`backend/cloud-gateway/`) is a Go service using stdlib `net/http` with Go 1.22+ ServeMux pattern routing (method + path patterns like `"GET /healthz"`).
- The state store (`state/store.go`) is the central in-memory data structure: thread-safe via `sync.RWMutex`, keyed by VIN, with a reverse token→VIN lookup map for O(1) auth validation.
- REST handlers in `api/` depend on state store and an `MQTTPublisher` interface, allowing the MQTT layer to be plugged in without changing handler code.
- CLOUD_GATEWAY uses `eclipse/paho.mqtt.golang` for MQTT. The client is created in `main.go` and injected into REST handlers via the `api.MQTTPublisher` interface.
- MQTT subscriptions are established on initial connect and on every reconnect via `SetOnConnectHandler`, ensuring recovery from broker restarts (03-REQ-2.E1).
- The `mqtt.Client` type embeds a reference to `state.Store` and directly updates it from MQTT message handlers — no intermediate channels or queues.
- CLOUD_GATEWAY_CLIENT uses `rumqttc` (v0.24) as the async MQTT client library with `tokio` as the runtime. The MQTT client is split into a `MqttClient` handle (for publishing) and an `EventLoop` (for driving the connection), following rumqttc's async architecture.
- VIN persistence uses a simple `vin.json` file in a configurable data directory; `load_or_create` handles first-start generation and subsequent reuse.

## Conventions

- **Go modules**: Go 1.25.7, organized as separate modules per service (`backend/cloud-gateway`, `mock/companion-app-cli`, etc.).
- **Go HTTP**: stdlib `net/http` with Go 1.22+ routing (method prefixes like `"GET /healthz"`), no frameworks.
- **Go testing**: `httptest.NewServer` with the real mux for integration-style testing rather than calling handlers directly. MQTT handler unit tests use `newTestClient(store)` to create a handler-only client (no real MQTT connection).
- **Go integration tests**: use `skipIfNoMQTT(t)` to probe the broker before running; they skip cleanly with an informative message when infrastructure is unavailable.
- **Go error responses**: follow `{"error": "...", "code": "..."}` JSON format consistently.
- **Go auth tokens**: Bearer tokens are 32-byte base64url-encoded random strings generated via `crypto/rand`.
- **Rust stack**: `clap` derive for CLI args, `tracing` for structured logging, `tokio` for async runtime.
- **Rust config**: Config modules use `clap::Parser` with `#[arg(long, env = "...", default_value = "...")]` for all flags, following the same pattern as `locking-service/src/config.rs`.
- **Rust errors**: Error types in cloud-gateway-client are hand-rolled enums with `Display` and `Error` impls (not `thiserror`), consistent with keeping dependencies minimal.
- **MQTT JSON wire format**: snake_case field names. Go uses `json:"snake_case"` struct tags; Rust uses `serde(rename)` when the Rust field name differs from the JSON key (e.g., `command_type` field maps to `"type"` in JSON). Topic parsing uses `strings.SplitN(topic, "/", 3)` to extract VIN and suffix from `vehicles/{vin}/{suffix}` format.
- **MQTT topics**: All topics use `{vin}` as a placeholder, resolved via `messages::topic_for(pattern, vin)`.
- **MQTT QoS**: Registration messages use QoS 2 (ExactlyOnce); telemetry uses QoS 0 (AtMostOnce).
- Rust `CommandResult` enum variants use SCREAMING_SNAKE_CASE to match the MQTT wire format directly, requiring `#[allow(non_camel_case_types, clippy::upper_case_acronyms)]`.

## Decisions

- MQTT message schemas are defined independently in Go and Rust (not generated from shared protobuf) because the MQTT transport uses JSON, not protobuf.
- `serde` and `serde_json` are workspace-level dependencies to support JSON serialization across Rust services.
- New modules in cloud-gateway-client are declared with `#[allow(dead_code)]` at the `mod` level since they define types consumed by later task groups.
- We use `rumqttc` (not `paho-mqtt`) for the Rust MQTT client because it's async-native with Tokio and the design doc specifies it.
- We use `google/uuid` (not stdlib) for command ID generation because Go stdlib doesn't provide UUID generation.
- We use a single `handleMessage` callback for all MQTT subscriptions (not per-topic callbacks) because Paho's `SubscribeMultiple` with a shared callback simplifies routing and ensures consistent error handling.
- The `rand` crate (v0.8) is used for VIN/PIN generation — it matches the workspace's existing `rand` usage pattern.
- The MQTT broker address is parsed from `host:port` format (not a URI) because that's what `rumqttc::MqttOptions` expects.
- Command response handler updates vehicle lock state (`IsLocked`) based on command result, not just the command status — ensures cached state reflects actual vehicle state after lock/unlock.
- The `MQTTPublisher` interface was defined in `api/handlers.go` with a no-op default, so REST handlers work standalone without MQTT — this avoids blocking task group 2 on task group 3. The `newServeMux` signature is `(store, publisher)`; tests pass `nil` for the no-op publisher.
- `GetVehicle` returns a shallow copy (not a pointer to internal state) to prevent callers from accidentally mutating the store.

## Fragile Areas

- **Go ↔ Rust wire-format parity**: changes to MQTT message schemas require updating both `messages/types.go` (Go) and `src/messages.rs` (Rust) in lockstep — there is no automated check.
- `StatusResponse` and `TelemetryMessage` use pointer/Option types for nullable fields; the JSON representation uses `null` for absent values, not field omission. Mismatches cause silent deserialization failures.
- `main_test.go` tested for HTTP 501 stubs; it had to be rewritten when stubs were replaced with real handlers. Future task groups that change `main.go` wiring may need corresponding `main_test.go` updates.
- `PairVehicle` uses sentinel errors (`ErrVehicleNotFound`, `ErrPINMismatch`) compared with `==` in handlers; wrapping these errors differently will break handler logic.
- The `paho.mqtt.golang` dependency pulls in `gorilla/websocket` and `golang.org/x/net` as transitive dependencies — monitor for security updates.
- The `subscribe` method runs its subscription wait in a goroutine to avoid blocking `OnConnect`. If subscriptions fail, they are only logged — no retry mechanism beyond the next reconnect cycle.
- The `parse_host` / `parse_port` functions in cloud-gateway-client use simple `rsplit_once(':')` parsing — IPv6 addresses would break this. Acceptable for demo scope.
- The event loop (`run_event_loop`) currently logs inbound publish messages but does not dispatch them to handlers — task groups 6 and 7 will wire command processing and telemetry.

## Failed Approaches

_(None recorded yet.)_

## Open Questions

- The MQTT client in `main.go` uses `log.Fatalf` on initial connection failure. For production, a retry loop before starting the HTTP server would be more resilient.
