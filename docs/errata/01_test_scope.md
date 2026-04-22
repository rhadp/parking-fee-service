# Errata: Spec 01 — Test Scope Deviations

## Issue

The spec 01 `make test` and `make check` targets do not run the full
`cargo test --workspace` or `go test ./...` across all modules as specified in
01-REQ-6.3 and 01-REQ-6.5. The following deviations exist:

### 1. Rust crates excluded from test-rust

`test-rust` uses `cargo test --workspace --exclude cloud-gateway-client
--exclude update-service --exclude parking-operator-adaptor` instead of
`cargo test --workspace`.

- **Crate exclusions** (`--exclude`): `cloud-gateway-client` (spec 04 TG1
  stubs), `update-service` (spec 07 TG1 stubs), and `parking-operator-adaptor`
  (spec 08 TG1 stubs) contain failing tests that require implementation from
  their respective specs. They are excluded entirely until those specs are
  implemented.
  `locking-service` was previously excluded (spec 03 TG1 stubs) but is now
  included after spec 03 task group 3 implementation.
  ~~`mock-sensors` integration tests were previously excluded via `--lib --bins`~~
  but are now included after spec 09 task group 5 implementation.

**Impact:** Test regressions in cloud-gateway-client, update-service, and
parking-operator-adaptor are not caught by `make test`. They are covered when
specs 04, 07, and 08 are implemented.

### 2. ~~Go mock/parking-operator excluded from test-go~~ Resolved

~~`test-go` does not run `go test` in `mock/parking-operator/`.~~
Now included after spec 09 implementation.

### 3. ~~Go tests/mock-apps excluded from test-go~~ Resolved

~~`test-go` does not run `go test` in `tests/mock-apps/`.~~
Now included after spec 09 task group 5 implementation. Runs integration tests
for all six mock tools (sensor, parking-operator, companion-app, parking-app)
including smoke tests and property tests.

### 4. Go backend modules scoped to root package

`test-go` uses `go test .` (root package only) instead of `go test ./...` for
`backend/parking-fee-service` and `backend/cloud-gateway`. Subpackages
(`config/`, `store/`, `handler/`, etc.) contain TG1 stub tests from specs 05
and 06 that fail until those specs are implemented.

**Impact:** Regressions in backend subpackages are not caught by `make test`.
They are covered when specs 05 and 06 are implemented.

### 5. Sensor binary skeleton behavior

The mock-sensor binaries (`location-sensor`, `speed-sensor`, `door-sensor`)
were implemented by spec 09 with full clap-based argument parsing. They require
specific command-line arguments and exit non-zero without them. This deviates
from 01-REQ-4.3 which states they SHALL print name/version and exit 0.

The setup verification tests (TS-01-15) use `--help` to verify the binary name
appears in output. The determinism test (TS-01-P2) uses `CombinedOutput` to
compare across invocations.

### 6. cloud-gateway-client does not reject unknown flags

`cloud-gateway-client` does not implement flag parsing in its skeleton and
ignores unknown flags (exits 0). This deviates from 01-REQ-4.E1 which states
skeletons SHALL reject unrecognized flags. The unknown-flag test (TS-01-E4)
excludes `cloud-gateway-client`. Flag parsing is spec 04's scope.

### 7. Proto generated code module

The `make proto` target generates Go code into `gen/` at the repository root.
`gen/` is a standalone Go module (`github.com/rhadp/parking-fee-service/gen`)
added to `go.work`. The generated code compiles and is importable by other
modules via the Go workspace. The proto files themselves are not modified by
code generation.

## Resolution

Once the relevant specs implement the required components, the Makefile should
be updated to:
- Remove `--exclude cloud-gateway-client` (after spec 04 implementation)
- Remove `--exclude update-service` (after spec 07 implementation)
- Remove `--exclude parking-operator-adaptor` (after spec 08 implementation)
- ~~Remove `--exclude locking-service` (after spec 03 implementation)~~ Done
- ~~Use `cargo test --workspace` without `--lib --bins` (after spec 09)~~ Done
- ~~Include `mock/parking-operator` in `test-go` (after spec 09)~~ Done
- ~~Include `tests/mock-apps` in `test-go` (after spec 09)~~ Done
- Use `go test ./...` for backend modules (after specs 05 and 06)
