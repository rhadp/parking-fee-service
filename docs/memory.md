# Project Memory

## Architecture

- Proto files live in `proto/common/` and `proto/services/`, with generated Go packages committed under `proto/gen/go/`. Generated Go packages use the module path `github.com/rhadp/parking-fee-service/proto/gen/go` with subdirectory packages: `common`, `services/update`, `services/adapter`.
- The Rust workspace root is `rhivos/Cargo.toml`; all service crates are direct children of `rhivos/`, with `parking-proto` as the shared bindings crate. The `mock/sensors` crate is included as an external workspace member via `"../mock/sensors"`; external members must set `workspace = "../../rhivos"` in their own `Cargo.toml` for dependency resolution.
- `parking-proto` generates both server and client gRPC stubs at build time via `tonic-build` in `build.rs`, reading `.proto` files from `../../proto/` relative to the crate manifest. Every service crate depends on it for gRPC types.
- Proto packages (`parking.common`, `parking.services.update`, `parking.services.adapter`) map to a nested Rust module hierarchy in `parking-proto/src/lib.rs`: `common`, `services::update`, `services::adapter`.
- `update-service` and `parking-operator-adaptor` are gRPC servers that register service handlers from `parking-proto`.
- `locking-service` and `cloud-gateway-client` are **not** gRPC servers — they are clients (Kuksa Databroker and MQTT respectively) whose skeletons just log and wait for shutdown.
- Go backend services (`parking-fee-service`, `cloud-gateway`) are separate Go modules (not a Go workspace) with independent `go.mod` files, each using `github.com/rhadp/parking-fee-service/backend/{service}` as its module path.
- Go mock CLIs use `replace` directives in `go.mod` to reference local proto generated packages at `../../proto/gen/go`, keeping proto as the single source of truth.
- The Kuksa Databroker v1 API uses a `Set` RPC with `EntryUpdate` messages containing `DataEntry` with `Datapoint` values. The proto is vendored locally in `mock/sensors/proto/` rather than pulling the full Kuksa SDK.
- The root Makefile iterates over `GO_BACKEND_DIRS` to run Go commands; new Go modules must be added to this list.

## Conventions

- Proto generation for Go uses `module=` option to strip the module prefix and produce clean package directories matching `go_package` paths. Generated `.pb.go` files are committed to the repo to avoid requiring `protoc` for Go-only builds.
- All Makefile targets that need tooling depend on `check-tools` as a prerequisite.
- Workspace dependencies are pinned in `rhivos/Cargo.toml` under `[workspace.dependencies]` and referenced by member crates with `{ workspace = true }`.
- tonic 0.12 + prost 0.13 are the chosen versions (matching the design doc). tonic 0.14 introduced a breaking reorganization (`tonic-prost` / `tonic-prost-build`), so do not upgrade without updating the `build.rs` API.
- The `clap` dependency must include the `"env"` feature (in addition to `"derive"`) to support `env = "..."` attributes on CLI args. For bool args needing explicit `true`/`false` values, use `#[arg(long, num_args = 1)]` instead of bare `#[arg(long)]` (which creates a presence flag).
- Each service binary name matches the crate/directory name (e.g., `locking-service` crate produces `locking-service` binary) via an explicit `[[bin]]` section.
- Skeleton contract tests use `TcpListener::bind("127.0.0.1:0")` for random port allocation and `tokio_stream::wrappers::TcpListenerStream` for `serve_with_incoming`.
- Go skeleton services use `net/http` stdlib with Go 1.22+ method-prefixed route patterns (e.g., `"GET /healthz"`). Stub handlers return `{"error":"not implemented","route":"..."}` with HTTP 501.
- Go tests use `httptest.NewServer(newServeMux())`, testing the mux directly without starting the real server.
- The `envOrDefault` helper pattern is used across Go services for configurable listen addresses with env var fallback.
- Go CLI applications use stdlib-only flag parsing (no cobra/urfave). Flag parsing is manual via global flag extraction functions that return remaining args.
- Go service modules follow the pattern: `go.mod` with module path `github.com/rhadp/parking-fee-service/{subdir}`, using Go 1.25.7.

## Decisions

- We use `--go_opt=module=...` and `--go-grpc_opt=module=...` (not `paths=source_relative`) because it respects the `go_package` option and produces the correct directory hierarchy (e.g., `services/update/` and `services/adapter/` instead of flat `services/`).
- Proto files follow the design document specifications exactly — field numbers, types, and names match the design doc verbatim.
- Module hierarchy in `parking-proto/src/lib.rs` mirrors proto package nesting (not flat). This is required because prost generates cross-package references using `super::super::` relative paths that depend on module depth matching package depth.
- We use `tokio-stream` as a workspace dependency (not just dev-dependency) because `update-service` needs `ReceiverStream` for the `WatchAdapterStates` server-streaming RPC type alias.
- `.gitkeep` files are removed from service directories once real source files are added.
- Go services use stdlib `net/http` (not gin/echo) because the design specifies `net/http` and the services are REST-only skeletons.
- Go binaries built in-place by `go build ./...` need explicit `.gitignore` entries since they appear in the module directory.
- Rust and Go build/test/lint targets are wired into the Makefile together in task group 7.
- We vendor a minimal subset of Kuksa Databroker proto (`val.proto` + `types.proto`) in `mock/sensors/proto/` rather than depending on `kuksa-rust-sdk`, to avoid heavy transitive dependencies and version conflicts in the workspace.

## Fragile Areas

- The `PROTO_FILES` variable in the Makefile uses `$(shell find ...)` which may pick up unexpected `.proto` files if new ones are added outside `common/` and `services/`.
- The `build.rs` proto path resolution uses `CARGO_MANIFEST_DIR` + relative paths (`../../proto/`). If the workspace layout changes, these paths will break.
- `tonic-build` version must match `tonic` version (e.g., tonic-build 0.12 with tonic 0.12). Mixing versions causes compile failures.
- The streaming RPC `WatchAdapterStates` requires a `type WatchAdapterStatesStream` associated type in the `UpdateService` impl — this is a tonic requirement for server-streaming RPCs. The standard pattern is to use `ReceiverStream`.
- `GO_BACKEND_DIRS` in the Makefile must be manually updated when new Go modules are added (e.g., task group 8 mock CLI modules).
- Go 1.22+ method-prefixed route patterns (`"GET /healthz"`) require Go 1.22 minimum; the `go.mod` files must reflect at least this version.
- The `mock/sensors` crate's `build.rs` depends on vendored proto files at `mock/sensors/proto/kuksa/val/v1/`. If the Kuksa Databroker API changes, these must be updated manually.
- The `parking-app-cli` gRPC connection tests have a 5-second timeout per attempt, causing the test suite to take ~20s. Consider making the dial timeout configurable or shorter for tests.

## Failed Approaches

- A flat module layout in `parking-proto/src/lib.rs` (with `common`, `update`, `adapter` as sibling modules at crate root) fails because prost's generated `super::super::common` references cannot resolve when services aren't nested under a `services` parent module.

## Open Questions

_(none yet)_
