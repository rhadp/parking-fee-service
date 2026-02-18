# Project Memory

## Architecture

- The project spans **Go** (backend/cloud services) and **Rust** (RHIVOS vehicle-side services), connected via MQTT through Eclipse Mosquitto.
- `parking-proto` crate provides generated gRPC/protobuf bindings for Kuksa DATA_BROKER; it's shared across all Rust services.
- Rust workspace root is `rhivos/Cargo.toml` with shared dependency versions. New dependencies must be added to `[workspace.dependencies]` first, then referenced by individual crates.

## Conventions

- **Go HTTP**: stdlib `net/http` with Go 1.22+ routing (method prefixes like `"GET /healthz"`), no frameworks.
- **Rust stack**: `clap` derive for CLI args, `tracing` for structured logging, `tokio` for async runtime.
- **MQTT JSON wire format**: snake_case field names. Go uses `json:"snake_case"` struct tags; Rust uses `serde(rename)` when the Rust field name differs from the JSON key (e.g., `command_type` field maps to `"type"` in JSON).
- Rust `CommandResult` enum variants use SCREAMING_SNAKE_CASE to match the MQTT wire format directly, requiring `#[allow(non_camel_case_types, clippy::upper_case_acronyms)]`.

## Decisions

- MQTT message schemas are defined independently in Go and Rust (not generated from shared protobuf) because the MQTT transport uses JSON, not protobuf.
- `serde` and `serde_json` are workspace-level dependencies to support JSON serialization across Rust services.
- New modules in cloud-gateway-client are declared with `#[allow(dead_code)]` at the `mod` level since they define types consumed by later task groups.

## Fragile Areas

- **Go ↔ Rust wire-format parity**: changes to MQTT message schemas require updating both `messages/types.go` (Go) and `src/messages.rs` (Rust) in lockstep — there is no automated check.
- `StatusResponse` and `TelemetryMessage` use pointer/Option types for nullable fields; the JSON representation uses `null` for absent values, not field omission. Mismatches cause silent deserialization failures.

## Failed Approaches

_(None recorded yet.)_

## Open Questions

_(None recorded yet.)_
