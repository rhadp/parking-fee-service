# Errata: Spec 01 Test Scope and Go Workspace Patterns

## Go 1.26 `./...` Pattern Change

The spec tests `TestGoBuildAllModules` and `TestGoTestPasses` use `go build ./...`
and `go test ./...` from the repository root. In Go 1.26, the `./...` pattern
no longer traverses into subdirectories that contain separate Go modules in a
workspace. Instead, each module must be addressed individually using its
directory path (e.g., `go build ./backend/parking-fee-service/...`).

**Impact:** The Makefile uses explicit module paths instead of `./...` patterns
for `build-go`, `test-go`, and `lint` targets. Setup tests that use `./...`
may fail on Go 1.26+ until they are updated to use explicit module paths.

## Test Scoping in Makefile

The `make test` target scopes Rust and Go tests to avoid running integration
tests from other specifications that are not yet implemented:

- **test-rust:** Uses `cargo test --workspace --lib --bins` which runs unit
  tests in lib and bin targets but skips integration tests in `tests/`
  directories (e.g., `rhivos/mock-sensors/tests/cli_tests.rs` from spec 09).

- **test-go:** Excludes `mock/parking-operator/server` (spec 09 server
  implementation is stubbed — `server.New()` returns nil). Also excludes
  `tests/mock-apps` (spec 09 integration tests) and `tests/setup`
  (setup verification tests have their own `make test-setup` target).

## Mock Module Skeleton Status

The existing mock modules (`mock/parking-app-cli`, `mock/companion-app-cli`,
`mock/parking-operator`) have stub `main.go` files that print "not yet
implemented" and exit 1. This causes `TestGoSkeletonBinaries` to fail for
these modules. Proper skeleton behavior (print version, exit 0) is
deferred to Task Group 3.
