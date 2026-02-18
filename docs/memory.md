# Project Memory

## Architecture

- The project spans **Go** (backend/cloud services) and **Rust** (RHIVOS vehicle-side services), connected via MQTT through Eclipse Mosquitto.
- `parking-proto` crate provides generated gRPC/protobuf bindings for Kuksa DATA_BROKER; it's shared across all Rust services.
- Rust workspace root is `rhivos/Cargo.toml` with shared dependency versions. New dependencies must be added to `[workspace.dependencies]` first, then referenced by individual crates.
- CLOUD_GATEWAY (`backend/cloud-gateway/`) is a Go service using stdlib `net/http` with Go 1.22+ ServeMux pattern routing (method + path patterns like `"GET /healthz"`).
- The state store (`state/store.go`) is the central in-memory data structure: thread-safe via `sync.RWMutex`, keyed by VIN, with a reverse token→VIN lookup map for O(1) auth validation.
- REST handlers in `api/` depend on state store and an `MQTTPublisher` interface, allowing the MQTT layer to be plugged in without changing handler code.

## Conventions

- **Go modules**: Go 1.25.7, organized as separate modules per service (`backend/cloud-gateway`, `mock/companion-app-cli`, etc.).
- **Go HTTP**: stdlib `net/http` with Go 1.22+ routing (method prefixes like `"GET /healthz"`), no frameworks.
- **Go testing**: `httptest.NewServer` with the real mux for integration-style testing rather than calling handlers directly.
- **Go error responses**: follow `{"error": "...", "code": "..."}` JSON format consistently.
- **Go auth tokens**: Bearer tokens are 32-byte base64url-encoded random strings generated via `crypto/rand`.
- **Rust stack**: `clap` derive for CLI args, `tracing` for structured logging, `tokio` for async runtime.
- **MQTT JSON wire format**: snake_case field names. Go uses `json:"snake_case"` struct tags; Rust uses `serde(rename)` when the Rust field name differs from the JSON key (e.g., `command_type` field maps to `"type"` in JSON).
- Rust `CommandResult` enum variants use SCREAMING_SNAKE_CASE to match the MQTT wire format directly, requiring `#[allow(non_camel_case_types, clippy::upper_case_acronyms)]`.

## Decisions

- MQTT message schemas are defined independently in Go and Rust (not generated from shared protobuf) because the MQTT transport uses JSON, not protobuf.
- `serde` and `serde_json` are workspace-level dependencies to support JSON serialization across Rust services.
- New modules in cloud-gateway-client are declared with `#[allow(dead_code)]` at the `mod` level since they define types consumed by later task groups.
- We use `google/uuid` (not stdlib) for command ID generation because Go stdlib doesn't provide UUID generation.
- The `MQTTPublisher` interface was defined in `api/handlers.go` with a no-op default, so REST handlers work standalone without MQTT — this avoids blocking task group 2 on task group 3.
- `GetVehicle` returns a shallow copy (not a pointer to internal state) to prevent callers from accidentally mutating the store.

## Fragile Areas

- **Go ↔ Rust wire-format parity**: changes to MQTT message schemas require updating both `messages/types.go` (Go) and `src/messages.rs` (Rust) in lockstep — there is no automated check.
- `StatusResponse` and `TelemetryMessage` use pointer/Option types for nullable fields; the JSON representation uses `null` for absent values, not field omission. Mismatches cause silent deserialization failures.
- `main_test.go` tested for HTTP 501 stubs; it had to be rewritten when stubs were replaced with real handlers. Future task groups that change `main.go` wiring may need corresponding `main_test.go` updates.
- `PairVehicle` uses sentinel errors (`ErrVehicleNotFound`, `ErrPINMismatch`) compared with `==` in handlers; wrapping these errors differently will break handler logic.

## Failed Approaches

_(None recorded yet.)_

## Open Questions

_(None recorded yet.)_
