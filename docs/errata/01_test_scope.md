# Errata: Spec 01 — Test Scope Deviations

## Issue

The spec 01 `make test` and `make check` targets do not run the full
`cargo test --workspace` or `go test ./...` across all modules as specified in
01-REQ-6.3 and 01-REQ-6.5. The following deviations exist:

### 1. Rust crates excluded from test-rust

`test-rust` uses `cargo test --workspace --exclude cloud-gateway-client
--exclude update-service` instead of `cargo test --workspace`.

- **Crate exclusions** (`--exclude`): `cloud-gateway-client` (spec 04 TG1
  stubs) and `update-service` (spec 07 TG1 stubs) contain failing tests that
  require implementation from their respective specs. They are excluded entirely
  until those specs are implemented.
  ~~`locking-service` was previously excluded (spec 03 TG1 stubs)~~ but is now
  included after spec 03 task group 3 implementation.
  ~~`parking-operator-adaptor` was previously excluded (spec 08 TG1 stubs)~~
  but is now included after spec 08 implementation.
  ~~`mock-sensors` integration tests were previously excluded via `--lib --bins`~~
  but are now included after spec 09 task group 5 implementation.
  ~~`--lib --bins` flag was previously used to exclude integration test files~~
  but has been removed since no crate has integration tests under `tests/`.

**Impact:** Test regressions in cloud-gateway-client and update-service are not
caught by `make test`. They are covered when specs 04 and 07 are implemented.

### 2. ~~Go mock/parking-operator excluded from test-go~~ Resolved

~~`test-go` does not run `go test` in `mock/parking-operator/`.~~
Now included after spec 09 implementation.

### 3. ~~Go tests/mock-apps excluded from test-go~~ Resolved

~~`test-go` does not run `go test` in `tests/mock-apps/`.~~
Now included after spec 09 task group 5 implementation. Runs integration tests
for all six mock tools (sensor, parking-operator, companion-app, parking-app)
including smoke tests and property tests.

### 4. Go tests/databroker and tests/locking-service excluded from test-go

`test-go` does not run `go test` in `tests/databroker/` or
`tests/locking-service/` because they contain integration tests that require
a running Kuksa Databroker container (and for locking-service, the compiled
Rust binary). Live gRPC tests skip when the container is not running.
`go vet` is run in the lint target for both modules.

**Impact:** Regressions in tests/databroker and tests/locking-service are not
caught by `make test`. They are covered when infrastructure is available.

### 5. ~~Go backend modules scoped to root package~~ Partially resolved

~~`test-go` uses `go test .` (root package only) instead of `go test ./...` for
`backend/parking-fee-service` and `backend/cloud-gateway`.~~
`backend/parking-fee-service` now uses `go test ./...` after spec 05 task
group 4 implementation. `backend/cloud-gateway` still uses `go test .` because
spec 06 subpackage tests are not yet implemented.

**Impact:** Regressions in backend/cloud-gateway subpackages are not caught by
`make test`. They are covered when spec 06 is implemented.

### 6. Sensor binary skeleton behavior

The mock-sensor binaries (`location-sensor`, `speed-sensor`, `door-sensor`)
were implemented by spec 09 with full clap-based argument parsing. They require
specific command-line arguments (`--lat`, `--lon`, `--speed`, `--open`) and
exit non-zero without them. This deviates from:

- **01-REQ-4.1**: "WHEN a Rust skeleton binary is executed, THEN it SHALL print
  a version string to stdout and exit with code 0."
- **01-REQ-4.3**: "Each mock-sensor binary SHALL print its name and version
  when executed and exit with code 0."

Spec 09 supersedes the skeleton behavior defined by spec 01 for these binaries.
The original spec 01 requirements were written for skeletons that would be
replaced by real implementations.

**Test workarounds:**
- `TestMockSensorBinaries` (TS-01-15) uses `--help` to verify the binary name
  appears in output and ignores the exit code.
- `TestPropertySkeletonDeterminism` (TS-01-P2) uses `CombinedOutput` for
  sensor binaries to compare across invocations without asserting exit code 0.

### 7. cloud-gateway-client does not reject unknown flags

`cloud-gateway-client` was fully reimplemented by spec 04 as a production
service that reads configuration from environment variables. It does not parse
command-line flags at all. When invoked with an unknown flag:

- If required environment variables are **not set**, it exits 1 due to missing
  config — not because the flag was rejected.
- If required environment variables **are set**, it would start normally,
  ignoring the unknown flag entirely (exits 0).

This deviates from **01-REQ-4.E1** which states skeleton binaries SHALL print
a usage message to stderr and exit with a non-zero code when invoked with an
unrecognized flag. The `TestSkeletonUnknownFlagExitsNonZero` (TS-01-E4)
excludes `cloud-gateway-client` because including it would produce a
misleading pass (exit 1 due to missing config, not flag rejection).

Adding flag rejection to `cloud-gateway-client` would require changes to
spec 04's implementation and is outside spec 01's scope.

### 8. Rust service binaries no longer have skeleton behavior

`update-service` (spec 07) and `parking-operator-adaptor` (spec 08) have been
replaced by full implementations that require runtime configuration and exit
non-zero without it. This deviates from:

- **01-REQ-4.1**: "WHEN a Rust skeleton binary is executed, THEN it SHALL print
  a version string to stdout and exit with code 0."

Only `locking-service` retains skeleton behavior (prints version and exits 0
when invoked without a subcommand). `cloud-gateway-client` was already excluded
(see section 7).

**Test impact:** `TestRustSkeletonBinaries` (TS-01-13) and
`TestPropertySkeletonDeterminism` (TS-01-P2) only test `locking-service` for
strict skeleton behavior. Other Rust binaries are excluded.

### 9. Go module binaries no longer have skeleton behavior

All Go modules have been replaced by full implementations from later specs:
- `backend/parking-fee-service` (spec 05): HTTP server, requires port binding
- `backend/cloud-gateway` (spec 06): HTTP/NATS gateway, requires config file
- `mock/parking-app-cli` (spec 09): CLI tool, requires subcommand
- `mock/companion-app-cli` (spec 09): CLI tool, requires subcommand
- `mock/parking-operator` (spec 09): gRPC server, requires subcommand

None print a simple version string to stdout and exit 0 as specified by
**01-REQ-4.2**. They all exit non-zero when invoked without proper arguments
or configuration.

**Test impact:** `TestGoSkeletonBinaries` (TS-01-14) uses CombinedOutput and
checks that the component name appears somewhere in the output (usage messages,
log lines) without asserting exit code 0. All 5 Go modules are now included
(including `backend/cloud-gateway`, which outputs "cloud-gateway" in its log
messages).

### 10. Proto generated code module

The `make proto` target generates Go code into `gen/` at the repository root.
`gen/` is a standalone Go module (`github.com/rhadp/parking-fee-service/gen`)
added to `go.work`. The generated code is committed to git so that Go modules
that import `gen/` packages can build from a clean checkout without requiring
`protoc`. Regenerate with `make proto` after modifying `.proto` files.

## Resolution

Once the relevant specs implement the required components, the Makefile should
be updated to:
- Remove `--exclude cloud-gateway-client` (after spec 04 implementation)
- Remove `--exclude update-service` (after spec 07 implementation)
- ~~Remove `--exclude parking-operator-adaptor` (after spec 08 implementation)~~ Done
- ~~Remove `--exclude locking-service` (after spec 03 implementation)~~ Done
- ~~Use `cargo test --workspace` without `--lib --bins` (after spec 09)~~ Done
- ~~Include `mock/parking-operator` in `test-go` (after spec 09)~~ Done
- ~~Include `tests/mock-apps` in `test-go` (after spec 09)~~ Done
- ~~Use `go test ./...` for backend/parking-fee-service (after spec 05)~~ Done
- Use `go test ./...` for backend/cloud-gateway (after spec 06)
