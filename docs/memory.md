# Project Memory

## Architecture

- Monorepo with three technology domains: Rust (`rhivos/`), Go (`backend/`, `mock/`), and proto (`proto/`). Root `Makefile` delegates to domain-specific tools.
- Local infrastructure uses Podman/Docker Compose with Eclipse Kuksa Databroker 0.5 (port 55555) and Eclipse Mosquitto (port 1883). Infrastructure config lives in `infra/config/{service}/` (e.g., `mosquitto.conf`, `vss_overlay.json`).
- Kuksa Databroker 0.5 uses `--vss` (aliased `--metadata`) to load VSS JSON files (comma-separated for multiple). The compose command also requires `--insecure` (disable TLS) and `--addr 0.0.0.0` (bind to all interfaces, since Kuksa defaults to `127.0.0.1`).
- Kuksa 0.5 exposes `kuksa.val.v2.VAL` via gRPC reflection by default. The v1 API (`kuksa.val.v1`) is also enabled but does NOT appear in gRPC reflection.
- Proto files live in `proto/common/` and `proto/services/`, with generated Go packages committed under `proto/gen/go/`. Generated Go packages use the module path `github.com/rhadp/parking-fee-service/proto/gen/go` with subdirectory packages: `common`, `services/update`, `services/adapter`.
- The Rust workspace root is `rhivos/Cargo.toml`; all service crates are direct children of `rhivos/`, with `parking-proto` as the shared bindings crate. The `mock/sensors` crate is included as an external workspace member via `"../mock/sensors"`; external members must set `workspace = "../../rhivos"` in their own `Cargo.toml` for dependency resolution.
- `parking-proto` generates both server and client gRPC stubs at build time via `tonic-build` in `build.rs`, reading `.proto` files from `../../proto/` relative to the crate manifest. It re-exports both parking-domain protos and Kuksa `kuksa.val.v2` protos, serving as the single Rust bindings crate for all services.
- `parking-proto` includes a `KuksaClient` helper module that wraps the Kuksa v2 gRPC API: `PublishValue` for writes, `GetValue` for reads (not the v1 `Set`/`Get` pattern). It also provides typed helpers like `get_f32`/`get_f64` and `subscribe_typed`.
- Proto packages (`parking.common`, `parking.services.update`, `parking.services.adapter`) map to a nested Rust module hierarchy in `parking-proto/src/lib.rs`: `common`, `services::update`, `services::adapter`.
- `update-service` and `parking-operator-adaptor` are gRPC servers that register service handlers from `parking-proto`.
- `locking-service` and `cloud-gateway-client` are **not** gRPC servers — they are clients (Kuksa Databroker and MQTT respectively) whose skeletons just log and wait for shutdown.
- Go backend services (`parking-fee-service`, `cloud-gateway`) are separate Go modules (not a Go workspace) with independent `go.mod` files, each using `github.com/rhadp/parking-fee-service/backend/{service}` as its module path.
- Go mock CLIs use `replace` directives in `go.mod` to reference local proto generated packages at `../../proto/gen/go`, keeping proto as the single source of truth.
- Kuksa Databroker 0.5 uses the `kuksa.val.v2` API (not v1). The v2 protos are vendored in `proto/vendor/kuksa/val/v2/` and compiled by `parking-proto`. The v1 API uses `Set`/`Get` RPCs; `mock/sensors` still uses v1 protos vendored locally at `mock/sensors/proto/kuksa/val/v1/`.
- The root Makefile iterates over `GO_BACKEND_DIRS` to run Go commands; new Go modules must be added to this list.

## Conventions

- Containerfiles use the `.Containerfile` extension (not `Dockerfile`), organized as `containers/{domain}/{service-name}.Containerfile`.
- Rust Containerfiles copy the full workspace context (all members) because Cargo workspace resolution requires every member to be present even when building a single binary.
- Containerfiles follow multi-stage build pattern: Rust uses `rust:1.75` builder + `debian:bookworm-slim` runtime; Go uses `golang:1.22` builder + `gcr.io/distroless/static-debian12:nonroot` runtime. Go Containerfiles are more targeted since Go modules are independent.
- Container images are tagged `{service-name}:latest` with no registry prefix for local development.
- Go services compile with `CGO_ENABLED=0` for fully static binaries, enabling distroless runtime images.
- All Makefile targets use bracketed prefix logging (e.g., `[make target]`).
- Makefile targets auto-detect container runtime: prefers `podman`, falls back to `docker`. Uses `command -v podman || command -v docker` pattern, shared between Makefile and test scripts.
- Test scripts under `tests/` are standalone bash scripts that can be run independently, using green/red color output with pass/fail counters. They gracefully degrade when dependencies are unavailable (e.g., container daemon not running).
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
- Go service modules follow the pattern: `go.mod` with module path `github.com/rhadp/parking-fee-service/{subdir}`, specifying `go 1.22`.
- VSS overlay JSON follows the COVESA VSS tree structure: `{ "Vehicle": { "type": "branch", "children": { ... } } }`. Leaf nodes require `type`, `datatype`, `description`; optional fields: `allowed`, `unit`, `min`, `max`.
- Third-party proto files are vendored into `proto/vendor/` with package paths preserved (e.g., `proto/vendor/kuksa/val/v2/val.proto`). Kuksa v2 protos are compiled with `build_server(false)` since Rust services are only clients.
- VSS signal path constants live in `parking-proto::signals` for shared use across crates, avoiding hardcoded signal path strings in each service.
- Integration tests requiring Kuksa Databroker use `#[ignore]` and `DATABROKER_ADDR` env var with default `http://localhost:55555`.
- Rust service modules follow the design doc structure (e.g., locking-service: `config.rs`, `safety.rs`, `lock_handler.rs`, `main.rs`).
- Property-based tests use `proptest` and live in a nested `mod prop` inside the main `#[cfg(test)]` module.
- `prop_assert_eq!` inside `proptest!` blocks does not support Rust inline format captures (`{var}`); use positional/named format args or explicit `format!()` calls instead.
- Custom CLI value types (e.g., `DoorState`, `LockAction`) implement `FromStr` for clap integration rather than using clap's built-in `value_enum`.
- `#[allow(clippy::enum_variant_names)]` is needed on `Command` enums where variant names share prefixes matching CLI subcommand naming conventions.
- Rust service `main()` delegates to a `run()` function for clean error handling: errors are logged via `tracing::error!` and the process exits with code 1, satisfying requirement 02-REQ-6.E1.

## Decisions

- We copy the full Rust workspace in Containerfiles (not stub workspace members) because maintaining accurate stub `Cargo.toml` for each member is fragile and breaks when dependencies change.
- Mock sensor Containerfile includes vendored Kuksa proto files (`mock/sensors/proto/kuksa/val/v1/`) needed by its `build.rs`.
- We use `--go_opt=module=...` and `--go-grpc_opt=module=...` (not `paths=source_relative`) because it respects the `go_package` option and produces the correct directory hierarchy (e.g., `services/update/` and `services/adapter/` instead of flat `services/`).
- Proto files follow the design document specifications exactly — field numbers, types, and names match the design doc verbatim.
- Module hierarchy in `parking-proto/src/lib.rs` mirrors proto package nesting (not flat). This is required because prost generates cross-package references using `super::super::` relative paths that depend on module depth matching package depth.
- We use `tokio-stream` as a workspace dependency (not just dev-dependency) because `update-service` needs `ReceiverStream` for the `WatchAdapterStates` server-streaming RPC type alias.
- `.gitkeep` files are removed from directories once real config or source files are added (applies to both service dirs and `infra/config/` subdirs).
- Go services use stdlib `net/http` (not gin/echo) because the design specifies `net/http` and the services are REST-only skeletons.
- Go binaries built in-place by `go build ./...` need explicit `.gitignore` entries since they appear in the module directory.
- Rust and Go build/test/lint targets are wired into the Makefile together in task group 7.
- We vendor Kuksa v1 protos locally in `mock/sensors/proto/` (not via `kuksa-rust-sdk`) to avoid heavy transitive dependencies. The shared v2 protos live in `proto/vendor/kuksa/val/v2/` and are compiled by `parking-proto` for all other services.
- Container runtime detection uses `ifndef CONTAINER_RUNTIME` with `$(error ...)` for clear error reporting when no runtime is installed, matching requirement 01-REQ-6.E2.
- We use `go vet` (not `golangci-lint`) for Go linting because it has zero external dependencies and covers the essential checks.
- Infrastructure smoke tests split into static tests (always run) and live tests (skipped when container daemon unavailable), ensuring CI without Docker can still validate config files.
- We added `KuksaClient` directly to `parking-proto` (not a separate crate) because it's small and all Rust services already depend on `parking-proto`.
- We use `thiserror` v2 for error types in the `KuksaClient` helper, added as a workspace dependency.
- `KuksaClient` type-coerces between float/double transparently in `get_f32`/`get_f64` to handle VSS data type flexibility (some signals report as f32, others as f64).
- We use `proptest` (not `quickcheck`) because it provides better shrinking and clearer error reports for property-based testing.
- The `validate_lock` function in locking-service is deliberately pure (no side effects), making it fully testable without mocking infrastructure.
- Speed threshold is checked first in locking-service validation order (before door-ajar), so `RejectedSpeed` takes priority when both conditions fail.
- The locking-service `Config` struct defaults `databroker_addr` to `http://localhost:55555` (with scheme), matching the Kuksa tonic client's URL format.

## Fragile Areas

- **Rust Containerfiles ↔ workspace member list:** Adding or removing a workspace member in `rhivos/Cargo.toml` requires updating all Rust Containerfiles.
- **`mock/parking-app-cli` Containerfile ↔ Go proto layout:** This Containerfile must include `proto/gen/go/` because of the `replace` directive in `go.mod`. Changes to the Go proto generation layout will break it.
- The `PROTO_FILES` variable in the Makefile uses `$(shell find ...)` which may pick up unexpected `.proto` files if new ones are added outside `common/` and `services/`.
- The `build.rs` proto path resolution uses `CARGO_MANIFEST_DIR` + relative paths (`../../proto/`). If the workspace layout changes, these paths will break.
- `tonic-build` version must match `tonic` version (e.g., tonic-build 0.12 with tonic 0.12). Mixing versions causes compile failures.
- The streaming RPC `WatchAdapterStates` requires a `type WatchAdapterStatesStream` associated type in the `UpdateService` impl — this is a tonic requirement for server-streaming RPCs. The standard pattern is to use `ReceiverStream`.
- `GO_BACKEND_DIRS` in the Makefile must be manually updated when new Go modules are added (e.g., task group 8 mock CLI modules).
- Go 1.22+ method-prefixed route patterns (`"GET /healthz"`) require Go 1.22 minimum; the `go.mod` files must reflect at least this version.
- The `mock/sensors` crate's `build.rs` depends on vendored proto files at `mock/sensors/proto/kuksa/val/v1/`. If the Kuksa Databroker API changes, these must be updated manually.
- The `parking-app-cli` gRPC connection tests have a 5-second timeout per attempt, causing the test suite to take ~20s. Consider making the dial timeout configurable or shorter for tests.
- Bash arithmetic with `set -euo pipefail`: `((var++))` returns exit code 1 when pre-increment value is 0. Use `var=$((var + 1))` instead.
- **Kuksa v2 `PublishValue` vs `Actuate`:** `PublishValue` sets signal values but requires provider permissions if the signal is registered as a sensor (not an actuator). `Actuate` is specifically for actuator signals. This distinction may matter for certain VSS signals.
- **`subscribe_typed` silent drops:** The `KuksaClient::subscribe_typed` implementation uses `filter_map` which silently drops entries with wrong types. Intentional for the demo but could mask type mismatch issues in production.
- **`proptest` macro format strings:** The `proptest!` macro expands format strings through `concat!`, which breaks Rust inline captures. Always use positional/named format args or skip custom messages in `prop_assert_eq!`.

## Failed Approaches

- A flat module layout in `parking-proto/src/lib.rs` (with `common`, `update`, `adapter` as sibling modules at crate root) fails because prost's generated `super::super::common` references cannot resolve when services aren't nested under a `services` parent module.

## Open Questions

- Go toolchain version spread: `go.mod` files specify `go 1.22` minimum, Containerfiles use `golang:1.22`, but the local toolchain is `go1.25.7`. This works due to Go's backward compatibility but may cause unexpected behavior with newer language features.
