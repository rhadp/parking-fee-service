# Project Memory

## Architecture

- Proto files live in `proto/common/` and `proto/services/`, with generated Go packages committed under `proto/gen/go/`. Rust bindings are generated at build time via `tonic-build` in `rhivos/parking-proto/build.rs` (not yet created).
- Generated Go packages use the module path `github.com/rhadp/parking-fee-service/proto/gen/go` with subdirectory packages: `common`, `services/update`, `services/adapter`.

## Conventions

- Proto generation for Go uses `module=` option to strip the module prefix and produce clean package directories matching `go_package` paths.
- Generated `.pb.go` files are committed to the repo (standard Go proto workflow) to avoid requiring `protoc` for Go-only builds.
- All Makefile targets that need tooling depend on `check-tools` as a prerequisite.

## Decisions

- We use `--go_opt=module=...` and `--go-grpc_opt=module=...` (not `paths=source_relative`) because it respects the `go_package` option and produces the correct directory hierarchy (e.g., `services/update/` and `services/adapter/` instead of flat `services/`).
- Proto files follow the design document specifications exactly — field numbers, types, and names match the design doc verbatim.

## Fragile Areas

- The `PROTO_FILES` variable in the Makefile uses `$(shell find ...)` which may pick up unexpected `.proto` files if new ones are added outside `common/` and `services/`.

## Failed Approaches

_(none yet)_

## Open Questions

_(none yet)_
