# Errata: 01_makefile_test_scope

Divergence from spec 01 requirements for `make test` and `make check` scope.

---

## E1 — test-rust previously excluded TDD stub crates (RESOLVED)

**Affects:** [01-REQ-6.3], TS-01-20

**Problem:**  
`make test` (`test-rust`) previously excluded `cloud-gateway-client`,
`update-service`, and `parking-operator-adaptor` from `cargo test`, and
`test-go` used `go test .` (root package only) for `backend/parking-fee-service`
and `backend/cloud-gateway` instead of `go test ./...`.

**Resolution:**  
All excluded crates now pass their tests. The exclusions have been removed:
- `test-rust` now runs `cargo test --workspace` (all workspace members).
- `test-go` now runs `go test ./...` for all Go modules (including sub-packages).
- `make check` now runs `lint test` (actual test execution, not compile-only).

This fully satisfies 01-REQ-6.3 and 01-REQ-6.5.

## E2 — proto code generation output not importable (OPEN)

**Affects:** [01-REQ-10.2]

**Problem:**  
01-REQ-10.2 requires generated Go code to be placed in a location importable
by Go modules. The `make proto` target writes output to `gen/`, which is
in `.gitignore` (generated at build time) and is not registered as a Go module
in `go.work`. No Go module currently imports from `gen/`.

**Mitigation:**  
Since no Go module in the current codebase imports from the generated `gen/`
directory, this is a latent issue. When a module needs proto-generated types,
it should either:
1. Vendor the generated code into a `pb/` sub-package (as `mock/parking-app-cli/pb/`
   and `tests/mock-apps/pb/` already do), or
2. Add `gen/` as a Go module in `go.work` with a `go.mod` file.

The existing pattern of vendoring proto-generated code into module-local `pb/`
directories is the established project convention.
