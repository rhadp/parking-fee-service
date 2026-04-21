# Agent-Fox Memory

_100 facts | last updated: 2026-04-21_

## locking-service (Rust crate)

- The prost-generated `Field` enum from `kuksa.val.v1` uses `Field::Value` (not `Field::FieldValue`) for the `FIELD_VALUE` proto constant. The name is derived from stripping the enum prefix (`FIELD_`), so `FIELD_VALUE` → `Value`.
- `GrpcBrokerClient` wraps `ValServiceClient<Channel>`. The `client` field is `Clone` (tonic clients are cheap to clone), so methods take `&self` and clone the client before calling RPCs.
- `BrokerClient` trait uses `#[async_trait(?Send)]`; `GrpcBrokerClient` impl must also use `#[async_trait(?Send)]`.
- The `subscribe` method on `GrpcBrokerClient` takes `&mut self` (required to call the RPC) and spawns a `tokio::spawn` (not `spawn_local`) task to forward string datapoints to an `mpsc::channel(64)`.
- `process_command` idempotent check uses the in-memory `lock_state` bool, NOT a broker read. This is intentional for ASIL-B: "keep locked" always wins even if broker signals are unsafe.
- The `main` function is `#[tokio::main]` async; the previous sync stub had a `todo!()`.
- Proto files live at `proto/kuksa/val/v1/val.proto`; `build.rs` uses `tonic_build::configure().build_server(false)`.
- Dependencies: `tonic = "0.12"`, `prost = "0.13"`, `futures = "0.3"`, `tonic-build = "0.12"` (build-dep).
