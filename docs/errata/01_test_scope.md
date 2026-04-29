# Errata: Test Scope for Spec 01 Project Setup

## Context

Spec 01 requirements 01-REQ-8.3 and 01-REQ-8.4 require that `cargo test` and
`go test` pass for all placeholder tests. However, the monorepo contains tests
from other specifications (e.g., spec 09 mock-sensors integration tests) that
depend on implementation work not covered by spec 01.

## Divergence

### Rust test scope (`TestCargoTestPasses`)

The setup verification test `TestCargoTestPasses` uses
`cargo test --workspace --lib --bins` instead of `cargo test --workspace`.

**Reason:** Spec 09 added integration tests in
`rhivos/mock-sensors/tests/cli_tests.rs` that test sensor argument parsing
behavior (e.g., missing `--lat`/`--lon`, unreachable broker). These tests
expect the sensors to have full CLI implementations, which is future work.
Spec 01 only requires skeleton behavior (print version, exit 0). The
`--lib --bins` flags restrict testing to library and binary unit tests,
matching the `make test-rust` Makefile target.

### Go test scope (`TestGoTestPasses`)

The setup verification test `TestGoTestPasses` tests individual module paths
rather than `go test ./...` from the repository root.

**Reason:** Two practical issues:
1. `go test ./...` does not work from the repo root in a Go workspace — it
   requires individual module paths.
2. `mock/parking-operator/server/` contains spec 09 tests that depend on
   server implementation not yet complete.

The test matches the `make test-go` Makefile target which explicitly lists
the modules to test.

### Makefile `test-rust` target

Uses `cargo test --workspace --lib --bins` to exclude spec 09 integration
tests from the default test run.

### Makefile `test-go` target

Explicitly lists Go test modules, excluding `mock/parking-operator` whose
server tests belong to spec 09.
