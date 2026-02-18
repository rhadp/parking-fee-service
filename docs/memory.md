# Project Memory

## Architecture

- Proto files live in `proto/common/` and `proto/services/`, with generated Go packages committed under `proto/gen/go/`. Generated Go packages use the module path `github.com/rhadp/parking-fee-service/proto/gen/go` with subdirectory packages: `common`, `services/update`, `services/adapter`.
- The Rust workspace lives in `rhivos/` with a workspace manifest listing all service crates and `parking-proto` as the shared bindings crate.
- `parking-proto` generates Rust types at build time via `tonic-build` in `build.rs`, reading `.proto` files from `../../proto/` relative to the crate manifest.
- Proto packages (`parking.common`, `parking.services.update`, `parking.services.adapter`) map to a nested Rust module hierarchy in `parking-proto/src/lib.rs`: `common`, `services::update`, `services::adapter`.

## Conventions

- Proto generation for Go uses `module=` option to strip the module prefix and produce clean package directories matching `go_package` paths. Generated `.pb.go` files are committed to the repo to avoid requiring `protoc` for Go-only builds.
- All Makefile targets that need tooling depend on `check-tools` as a prerequisite.
- Workspace dependencies are pinned in `rhivos/Cargo.toml` under `[workspace.dependencies]` and referenced by member crates with `{ workspace = true }`.
- tonic 0.12 + prost 0.13 are the chosen versions (matching the design doc). tonic 0.14 introduced a breaking reorganization (`tonic-prost` / `tonic-prost-build`), so do not upgrade without updating the `build.rs` API.

## Decisions

- We use `--go_opt=module=...` and `--go-grpc_opt=module=...` (not `paths=source_relative`) because it respects the `go_package` option and produces the correct directory hierarchy (e.g., `services/update/` and `services/adapter/` instead of flat `services/`).
- Proto files follow the design document specifications exactly — field numbers, types, and names match the design doc verbatim.
- Module hierarchy in `parking-proto/src/lib.rs` mirrors proto package nesting (not flat). This is required because prost generates cross-package references using `super::super::` relative paths that depend on module depth matching package depth.

## Fragile Areas

- The `PROTO_FILES` variable in the Makefile uses `$(shell find ...)` which may pick up unexpected `.proto` files if new ones are added outside `common/` and `services/`.
- The `build.rs` proto path resolution uses `CARGO_MANIFEST_DIR` + relative paths (`../../proto/`). If the workspace layout changes, these paths will break.
- `tonic-build` version must match `tonic` version (e.g., tonic-build 0.12 with tonic 0.12). Mixing versions causes compile failures.

## Failed Approaches

- A flat module layout in `parking-proto/src/lib.rs` (with `common`, `update`, `adapter` as sibling modules at crate root) fails because prost's generated `super::super::common` references cannot resolve when services aren't nested under a `services` parent module.

## Open Questions

_(none yet)_
