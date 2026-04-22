# Errata: Spec 01 — Test Scope Deviations

## Issue

The spec 01 `make test` and `make check` targets do not run the full
`cargo test --workspace` or `go test ./...` across all modules as specified in
01-REQ-6.3 and 01-REQ-6.5. Two deviations exist:

### 1. Rust integration tests excluded

`test-rust` uses `cargo test --workspace --lib --bins` instead of
`cargo test --workspace`. This skips integration test suites in `tests/`
directories.

**Reason:** The `rhivos/mock-sensors/tests/cli_tests.rs` file contains 9
integration tests from spec 09 (mock apps). These tests expect the sensor
binaries to have full CLI argument parsing and broker connection logic, which
is spec 09 implementation scope. The spec 01 placeholder tests (`it_compiles`)
are all unit tests and pass correctly.

**Impact:** Integration test regressions in sensor binaries are not caught by
`make test`. They are covered when spec 09 is implemented.

### 2. Go mock/parking-operator excluded from test-go

`test-go` does not run `go test` in `mock/parking-operator/` because
`server_test.go` (spec 09) contains integration tests that require the HTTP
server routes to be implemented.

**Impact:** Regressions in mock/parking-operator are not caught by `make test`.
They are covered when spec 09 is implemented.

## Resolution

Once specs 09+ implement the sensor binaries and mock-operator server, the
Makefile should be updated to:
- Use `cargo test --workspace` (no `--lib --bins` restriction)
- Include `mock/parking-operator` in `test-go`
