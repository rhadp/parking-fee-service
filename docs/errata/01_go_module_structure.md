# Erratum: Go Module Structure (Spec 01)

## Deviation

The design document specifies separate `go.mod` files for `backend/` and `mock/`
directories, with a `go.work` file linking them as independent Go modules.

## Actual Implementation

A single root `go.mod` is used instead. The `backend/` and `mock/` directories
are packages within the root module `github.com/rhadp/parking-fee-service`,
not separate modules.

The `go.work` file references `.` (root module) and `./tests/setup` as `use`
directives, with `./backend` and `./mock` documented as comments.

## Rationale

Go workspaces do not support the `./...` wildcard pattern from the workspace
root directory. The `go build ./...` and `go test ./...` commands (required by
01-REQ-3.2 and 01-REQ-3.3) only match packages within modules listed in
`go.work`, and the `./...` pattern requires the current directory (`.`) to be a
prefix of at least one workspace member's directory.

With separate `go.mod` files in `backend/` and `mock/`, the only way to build
all packages is `go build ./backend/... ./mock/...`, which does not satisfy the
requirement for `go build ./...`.

Using a single root module makes `./...` resolve all packages under the root,
satisfying both the build/test requirements and the Go workspace structure.

## Impact

- `TestEdgeMissingGoMod` (TS-01-E3) is skipped because `backend/go.mod` does
  not exist. The equivalent test would need to remove the root `go.mod`.
- Import paths use `github.com/rhadp/parking-fee-service/backend/...` and
  `github.com/rhadp/parking-fee-service/mock/...` instead of separate module
  paths.
