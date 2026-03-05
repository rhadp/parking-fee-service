# Implementation Tasks: Project Setup (Spec 01)

> Task plan for implementing the project scaffolding defined in `.specs/01_project_setup/design.md`.
> Tasks are organized into sequential groups. Each group must be completed before the next begins.

## Task Group 1: Write Failing Spec Tests

Write the Go test suite in `tests/setup/` that verifies all requirements and correctness properties. These tests will fail initially because the implementation does not yet exist, establishing a test-first development approach.

### Task 1.1: Create test module

Create the `tests/setup/` directory with a `go.mod` file.

- Module path: `github.com/rhadp/parking-fee-service/tests/setup`
- Go version: 1.22
- Add dependency on `github.com/stretchr/testify` for assertions
- Add dependency on `gopkg.in/yaml.v3` for docker-compose parsing

**Files created:**
- `tests/setup/go.mod`
- `tests/setup/go.sum` (after `go mod tidy`)

### Task 1.2: Write directory structure tests

Implement tests for TS-01-1 and TS-01-E4.

- `TestDirectoryStructure`: Verify all required directories exist.
- `TestPlaceholderDirectories`: Verify `aaos/` and `android/` contain only `.gitkeep`.
- Helper function to resolve the repository root path (two levels up from `tests/setup/`).

**Files created:**
- `tests/setup/structure_test.go`

### Task 1.3: Write proto definition tests

Implement tests for TS-01-2.

- `TestProtoDefinitions`: Verify proto files exist with correct syntax, package, and go_package.
- Read file contents and check for required string patterns.

**Files created:**
- `tests/setup/proto_test.go`

### Task 1.4: Write code generation tests

Implement tests for TS-01-3 and TS-01-P1.

- `TestGoCodeGeneration`: Run `make proto`, verify generated files exist, verify Go compilation.
- `TestProtoRoundtrip`: Same verification as a correctness property test.

**Files added to:**
- `tests/setup/proto_test.go`

### Task 1.5: Write Rust workspace tests

Implement tests for TS-01-4, TS-01-P2.

- `TestRustWorkspace`: Verify workspace manifest, member crates, and compilation.
- `TestRustWorkspaceConsistency`: Verify member directories match manifest.

**Files created:**
- `tests/setup/rust_test.go`

### Task 1.6: Write Makefile target tests

Implement tests for TS-01-5, TS-01-P3, TS-01-P6, TS-01-P7, TS-01-E2.

- `TestMakefileTargets`: Verify all targets are defined and basic ones execute.
- `TestBuildIdempotency`: Run `make build` twice.
- `TestCleanBuild`: Run `make clean` then `make build`.
- `TestLintClean`: Run `make lint` on skeleton code.
- `TestCleanIdempotent`: Run `make clean` twice.

**Files created:**
- `tests/setup/makefile_test.go`

### Task 1.7: Write infrastructure tests

Implement tests for TS-01-6, TS-01-P4, TS-01-E3.

- `TestDockerComposeServices`: Verify compose file, start services, check ports, stop services.
- `TestInfraLifecycle`: Full start/stop cycle with orphan check.
- `TestInfraDownWhenNotRunning`: Idempotent teardown.
- Helper function for TCP port probing with timeout.

**Files created:**
- `tests/setup/infra_test.go`

### Task 1.8: Write Go workspace tests

Implement tests for TS-01-7, TS-01-P5.

- `TestGoWorkspace`: Verify `go.work` contents and module references.
- `TestGoWorkspaceResolution`: Run `go work sync` and `go build ./...`.

**Files created:**
- `tests/setup/workspace_test.go`

### Task 1.9: Verify tests compile but fail

Run `cd tests/setup && go test ./... -v` and confirm all tests either fail or are skipped (not passed). This validates the test harness is correctly detecting the missing implementation.

**Verification command:**
```bash
cd tests/setup && go test ./... -v 2>&1 | tail -5
# Expected: FAIL or SKIP for all tests
```

## Task Group 2: Repository Structure and Go/Rust Workspaces

Create the directory layout, Go modules, and Rust workspace.

### Task 2.1: Create directory structure

Create all required directories:

```
mkdir -p proto gen/go/commonpb gen/go/updateservicepb gen/go/parkingadaptorpb
mkdir -p rhivos/locking-service/src rhivos/cloud-gateway-client/src
mkdir -p rhivos/update-service/src rhivos/parking-operator-adaptor/src
mkdir -p backend/parking-fee-service backend/cloud-gateway
mkdir -p mock/parking-app-cli mock/companion-app-cli
mkdir -p aaos android
mkdir -p infra tests/setup tests/integration
touch aaos/.gitkeep android/.gitkeep tests/integration/.gitkeep
```

**Verification:** Run `TestDirectoryStructure` and `TestPlaceholderDirectories`.

### Task 2.2: Create Rust workspace

Create `rhivos/Cargo.toml` as a workspace manifest:

```toml
[workspace]
members = [
    "locking-service",
    "cloud-gateway-client",
    "update-service",
    "parking-operator-adaptor",
]
resolver = "2"
```

Create each member crate's `Cargo.toml` and `src/main.rs`:

- `rhivos/locking-service/Cargo.toml` with `name = "locking-service"`, `edition = "2021"`
- `rhivos/cloud-gateway-client/Cargo.toml` with `name = "cloud-gateway-client"`, `edition = "2021"`
- `rhivos/update-service/Cargo.toml` with `name = "update-service"`, `edition = "2021"`
- `rhivos/parking-operator-adaptor/Cargo.toml` with `name = "parking-operator-adaptor"`, `edition = "2021"`

Each `src/main.rs`:
```rust
fn main() {
    println!("Hello from <crate-name>");
}
```

**Verification:**
```bash
cd rhivos && cargo check --workspace
cd tests/setup && go test -run TestRustWorkspace -v
```

### Task 2.3: Create Go modules

Create `go.mod` for each Go module:

- `backend/parking-fee-service/go.mod` -- module `github.com/rhadp/parking-fee-service/backend/parking-fee-service`
- `backend/cloud-gateway/go.mod` -- module `github.com/rhadp/parking-fee-service/backend/cloud-gateway`
- `mock/parking-app-cli/go.mod` -- module `github.com/rhadp/parking-fee-service/mock/parking-app-cli`
- `mock/companion-app-cli/go.mod` -- module `github.com/rhadp/parking-fee-service/mock/companion-app-cli`
- `gen/go/go.mod` -- module `github.com/rhadp/parking-fee-service/gen/go`

Create skeleton `main.go` for each binary module:

- `backend/parking-fee-service/main.go`
- `backend/cloud-gateway/main.go`
- `mock/parking-app-cli/main.go`
- `mock/companion-app-cli/main.go`

Each `main.go`:
```go
package main

func main() {}
```

### Task 2.4: Create Go workspace

Create `go.work` at the repository root:

```
go 1.22

use (
    backend/parking-fee-service
    backend/cloud-gateway
    mock/parking-app-cli
    mock/companion-app-cli
    gen/go
    tests/setup
)
```

Run `go work sync` to validate.

**Verification:**
```bash
go work sync
cd tests/setup && go test -run TestGoWorkspace -v
```

## Task Group 3: Protocol Buffer Definitions and Code Generation

### Task 3.1: Write proto files

Create the three `.proto` files in `proto/` as specified in the design document (Section 5):

- `proto/common.proto` -- AdapterState enum, AdapterInfo message, ErrorDetails message
- `proto/update_service.proto` -- UpdateService with 5 RPCs and all request/response messages
- `proto/parking_adaptor.proto` -- ParkingAdaptorService with 4 RPCs and all request/response messages

**Verification:**
```bash
cd tests/setup && go test -run TestProtoDefinitions -v
```

### Task 3.2: Configure gen/go module for protobuf dependencies

Update `gen/go/go.mod` to include required dependencies:

```
require (
    google.golang.org/protobuf v1.33.0
    google.golang.org/grpc v1.62.0
)
```

Run `go mod tidy` in `gen/go/`.

### Task 3.3: Add proto generation to Makefile

Implement the `proto` target in the Makefile (created in Task Group 4, but the proto-specific logic is defined here):

- Check for `protoc`, `protoc-gen-go`, `protoc-gen-go-grpc` availability
- Run protoc for each `.proto` file:
  ```
  protoc --go_out=gen/go --go_opt=paths=source_relative \
         --go-grpc_out=gen/go --go-grpc_opt=paths=source_relative \
         -I proto proto/<file>.proto
  ```
- Output files go to `gen/go/commonpb/`, `gen/go/updateservicepb/`, `gen/go/parkingadaptorpb/`

### Task 3.4: Generate Go code and verify

Run `make proto` and verify:

1. Generated files exist in all three sub-packages
2. `go build ./...` in `gen/go/` succeeds

**Verification:**
```bash
make proto
cd tests/setup && go test -run TestGoCodeGeneration -v
cd tests/setup && go test -run TestProtoRoundtrip -v
```

## Task Group 4: Build System (Makefile)

### Task 4.1: Create Makefile

Create the top-level `Makefile` with all eight targets as specified in the design document (Section 3).

Key implementation details:

- Use `COMPOSE ?= docker-compose` for compose command override
- Define `GO_MODULES` variable listing all Go module directories
- `build` target: `cargo build --workspace` in `rhivos/`, then loop over Go modules
- `test` target: `cargo test --workspace` in `rhivos/`, then loop over Go modules
- `lint` target: `cargo clippy --workspace -- -D warnings` in `rhivos/`, then `go vet` loop
- `check` target: depends on `build test lint`
- `proto` target: tool checks + protoc invocations (from Task 3.3)
- `infra-up` target: `$(COMPOSE) -f infra/docker-compose.yml up -d`
- `infra-down` target: `$(COMPOSE) -f infra/docker-compose.yml down`
- `clean` target: `cargo clean` in `rhivos/`, `go clean -cache`

**Files created:**
- `Makefile`

### Task 4.2: Verify build target

```bash
make build
cd tests/setup && go test -run TestMakefileTargets -v
```

### Task 4.3: Verify test and lint targets

```bash
make test
make lint
cd tests/setup && go test -run TestLintClean -v
```

### Task 4.4: Verify check target

```bash
make check
```

### Task 4.5: Verify clean and rebuild

```bash
make clean && make build
cd tests/setup && go test -run TestCleanBuild -v
cd tests/setup && go test -run TestCleanIdempotent -v
cd tests/setup && go test -run TestBuildIdempotency -v
```

## Task Group 5: Local Infrastructure (Docker-Compose)

### Task 5.1: Create docker-compose.yml

Create `infra/docker-compose.yml` as specified in the design document (Section 4):

- NATS server on port 4222 with health check
- Kuksa Databroker on port 55556 with health check

**Files created:**
- `infra/docker-compose.yml`

### Task 5.2: Verify infrastructure lifecycle

```bash
make infra-up
# Wait for services to become healthy
make infra-down
cd tests/setup && go test -run TestDockerComposeServices -v -timeout 120s
cd tests/setup && go test -run TestInfraLifecycle -v -timeout 120s
cd tests/setup && go test -run TestInfraDownWhenNotRunning -v
```

## Task Group 6: Checkpoint -- Verify Everything Works Together

### Task 6.1: Run all spec tests

Execute the full test suite and verify all tests pass.

```bash
cd tests/setup && go test ./... -v
```

All tests from TS-01-1 through TS-01-E4 must pass.

### Task 6.2: Run full make check

```bash
make check
```

Must exit with code 0.

### Task 6.3: Verify infrastructure round-trip

```bash
make infra-up
# Verify NATS is reachable on port 4222
# Verify Kuksa Databroker is reachable on port 55556
make infra-down
```

### Task 6.4: Verify clean build from scratch

```bash
make clean
make build
make test
make lint
```

All must succeed.

### Task 6.5: Final review against Definition of Done

Verify every item in the Definition of Done (design.md Section 9) is satisfied:

1. All directories exist with expected files.
2. `make proto` generates Go code from all three proto files.
3. `make build` compiles all Rust and Go components.
4. `make test` passes all unit tests.
5. `make lint` passes with no warnings.
6. `make check` succeeds end-to-end.
7. `make infra-up` starts NATS and Kuksa; both are reachable.
8. `make infra-down` stops all containers cleanly.
9. `make clean && make build` succeeds.
10. `go work sync` succeeds.
11. All tests in `tests/setup/` pass.

## Traceability: Tasks to Requirements

| Task | Requirement(s) | Test(s) Verified |
|------|----------------|------------------|
| 1.1 | -- (test infrastructure) | -- |
| 1.2 | 01-REQ-1 | TS-01-1, TS-01-E4 |
| 1.3 | 01-REQ-2 | TS-01-2 |
| 1.4 | 01-REQ-3 | TS-01-3, TS-01-P1 |
| 1.5 | 01-REQ-4 | TS-01-4, TS-01-P2 |
| 1.6 | 01-REQ-5 | TS-01-5, TS-01-P3, TS-01-P6, TS-01-P7, TS-01-E2 |
| 1.7 | 01-REQ-6 | TS-01-6, TS-01-P4, TS-01-E3 |
| 1.8 | 01-REQ-7 | TS-01-7, TS-01-P5 |
| 2.1 | 01-REQ-1 | TS-01-1, TS-01-E4 |
| 2.2 | 01-REQ-4 | TS-01-4, TS-01-P2 |
| 2.3 | 01-REQ-7 | TS-01-7 |
| 2.4 | 01-REQ-7 | TS-01-7, TS-01-P5 |
| 3.1 | 01-REQ-2 | TS-01-2 |
| 3.2 | 01-REQ-3 | TS-01-3 |
| 3.3 | 01-REQ-3 | TS-01-3, TS-01-P1 |
| 3.4 | 01-REQ-3 | TS-01-3, TS-01-P1 |
| 4.1 | 01-REQ-5 | TS-01-5 |
| 4.2 | 01-REQ-5 | TS-01-5, TS-01-P3 |
| 4.3 | 01-REQ-5 | TS-01-5, TS-01-P7 |
| 4.4 | 01-REQ-5 | TS-01-5 |
| 4.5 | 01-REQ-5 | TS-01-P3, TS-01-P6, TS-01-E2 |
| 5.1 | 01-REQ-6 | TS-01-6 |
| 5.2 | 01-REQ-6 | TS-01-6, TS-01-P4, TS-01-E3 |
| 6.1 | All | All |
| 6.2 | 01-REQ-5 | TS-01-5 |
| 6.3 | 01-REQ-6 | TS-01-6, TS-01-P4 |
| 6.4 | 01-REQ-5 | TS-01-P6 |
| 6.5 | All | All |

## Traceability: Tasks to Correctness Properties

| Task | Correctness Property |
|------|---------------------|
| 3.4 | CP-01 (Proto Compilation Roundtrip) |
| 2.2, 1.5 | CP-02 (Rust Workspace Consistency) |
| 4.5 | CP-03 (Build Idempotency) |
| 5.2 | CP-04 (Infrastructure Lifecycle Symmetry) |
| 2.4 | CP-05 (Go Workspace Module Resolution) |
| 4.5 | CP-06 (Clean Build from Scratch) |
| 4.3 | CP-07 (Lint Clean State) |
