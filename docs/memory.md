# Project Memory

## Architecture

- Proto files live in `proto/common/` and `proto/services/`, with generated Go packages committed under `proto/gen/go/`. Generated Go packages use the module path `github.com/rhadp/parking-fee-service/proto/gen/go` with subdirectory packages: `common`, `services/update`, `services/adapter`.
- The Rust workspace root is `rhivos/Cargo.toml`; all service crates are direct children of `rhivos/`, with `parking-proto` as the shared bindings crate.
- `parking-proto` generates both server and client gRPC stubs at build time via `tonic-build` in `build.rs`, reading `.proto` files from `../../proto/` relative to the crate manifest. Every service crate depends on it for gRPC types.
- Proto packages (`parking.common`, `parking.services.update`, `parking.services.adapter`) map to a nested Rust module hierarchy in `parking-proto/src/lib.rs`: `common`, `services::update`, `services::adapter`.
- `update-service` and `parking-operator-adaptor` are gRPC servers that register service handlers from `parking-proto`.
- `locking-service` and `cloud-gateway-client` are **not** gRPC servers — they are clients (Kuksa Databroker and MQTT respectively) whose skeletons just log and wait for shutdown.

## Conventions

- Proto generation for Go uses `module=` option to strip the module prefix and produce clean package directories matching `go_package` paths. Generated `.pb.go` files are committed to the repo to avoid requiring `protoc` for Go-only builds.
- All Makefile targets that need tooling depend on `check-tools` as a prerequisite.
- Workspace dependencies are pinned in `rhivos/Cargo.toml` under `[workspace.dependencies]` and referenced by member crates with `{ workspace = true }`.
- tonic 0.12 + prost 0.13 are the chosen versions (matching the design doc). tonic 0.14 introduced a breaking reorganization (`tonic-prost` / `tonic-prost-build`), so do not upgrade without updating the `build.rs` API.
- The `clap` dependency must include the `"env"` feature (in addition to `"derive"`) to support `env = "..."` attributes on CLI args.
- Each service binary name matches the crate/directory name (e.g., `locking-service` crate produces `locking-service` binary) via an explicit `[[bin]]` section.
- Skeleton contract tests use `TcpListener::bind("127.0.0.1:0")` for random port allocation and `tokio_stream::wrappers::TcpListenerStream` for `serve_with_incoming`.

## Decisions

- We use `--go_opt=module=...` and `--go-grpc_opt=module=...` (not `paths=source_relative`) because it respects the `go_package` option and produces the correct directory hierarchy (e.g., `services/update/` and `services/adapter/` instead of flat `services/`).
- Proto files follow the design document specifications exactly — field numbers, types, and names match the design doc verbatim.
- Module hierarchy in `parking-proto/src/lib.rs` mirrors proto package nesting (not flat). This is required because prost generates cross-package references using `super::super::` relative paths that depend on module depth matching package depth.
- We use `tokio-stream` as a workspace dependency (not just dev-dependency) because `update-service` needs `ReceiverStream` for the `WatchAdapterStates` server-streaming RPC type alias.
- `.gitkeep` files are removed from service directories once real source files are added.

## Fragile Areas

- The `PROTO_FILES` variable in the Makefile uses `$(shell find ...)` which may pick up unexpected `.proto` files if new ones are added outside `common/` and `services/`.
- The `build.rs` proto path resolution uses `CARGO_MANIFEST_DIR` + relative paths (`../../proto/`). If the workspace layout changes, these paths will break.
- `tonic-build` version must match `tonic` version (e.g., tonic-build 0.12 with tonic 0.12). Mixing versions causes compile failures.
- The streaming RPC `WatchAdapterStates` requires a `type WatchAdapterStatesStream` associated type in the `UpdateService` impl — this is a tonic requirement for server-streaming RPCs. The standard pattern is to use `ReceiverStream`.

## Failed Approaches

- A flat module layout in `parking-proto/src/lib.rs` (with `common`, `update`, `adapter` as sibling modules at crate root) fails because prost's generated `super::super::common` references cannot resolve when services aren't nested under a `services` parent module.

## Open Questions

_(none yet)_
