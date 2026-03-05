# Test Specification: Project Setup (Spec 01)

> Test specifications for verifying the project scaffolding described in `.specs/01_project_setup/requirements.md` and the correctness properties in `.specs/01_project_setup/design.md`.
> All tests are implemented in Go under `tests/setup/`.

## Test ID Convention

| Prefix | Category |
|--------|----------|
| TS-01-N | Requirement verification (N = requirement number) |
| TS-01-PN | Correctness property verification (N = property number) |
| TS-01-EN | Edge case / error path verification |

## Requirement Tests

### TS-01-1: Repository Directory Structure

**Traces to:** 01-REQ-1

**Objective:** Verify that all required top-level directories exist.

**Preconditions:** Repository is checked out.

**Steps:**

1. For each required directory (`proto/`, `gen/go/`, `rhivos/`, `backend/`, `mock/`, `aaos/`, `android/`, `infra/`, `tests/`), check that the path exists and is a directory.
2. For `aaos/` and `android/`, verify that a `.gitkeep` file exists inside.
3. Verify `tests/setup/` and `tests/integration/` subdirectories exist.
4. Verify `backend/parking-fee-service/` and `backend/cloud-gateway/` subdirectories exist.
5. Verify `mock/parking-app-cli/` and `mock/companion-app-cli/` subdirectories exist.

**Expected result:** All directories exist. Placeholder directories contain `.gitkeep`.

**Test command:**
```bash
cd tests/setup && go test -run TestDirectoryStructure -v
```

### TS-01-2: Protocol Buffer Definitions

**Traces to:** 01-REQ-2

**Objective:** Verify that proto files exist with correct syntax and package declarations.

**Preconditions:** Repository is checked out.

**Steps:**

1. Verify `proto/common.proto` exists and is non-empty.
2. Verify `proto/update_service.proto` exists and is non-empty.
3. Verify `proto/parking_adaptor.proto` exists and is non-empty.
4. For each proto file, read contents and verify it contains `syntax = "proto3";`.
5. For each proto file, verify it contains `package sdv.parking.v1;`.
6. Verify `common.proto` contains the `go_package` option targeting `commonpb`.
7. Verify `update_service.proto` contains the `go_package` option targeting `updateservicepb`.
8. Verify `parking_adaptor.proto` contains the `go_package` option targeting `parkingadaptorpb`.

**Expected result:** All three proto files exist with correct syntax, package, and go_package declarations.

**Test command:**
```bash
cd tests/setup && go test -run TestProtoDefinitions -v
```

### TS-01-3: Go Code Generation

**Traces to:** 01-REQ-3

**Objective:** Verify that `make proto` generates valid Go code.

**Preconditions:** `protoc`, `protoc-gen-go`, and `protoc-gen-go-grpc` are installed.

**Steps:**

1. Run `make proto` from the repository root.
2. Verify exit code is 0.
3. Verify `gen/go/commonpb/` contains at least one `.pb.go` file.
4. Verify `gen/go/updateservicepb/` contains at least one `.pb.go` file and at least one `_grpc.pb.go` file.
5. Verify `gen/go/parkingadaptorpb/` contains at least one `.pb.go` file and at least one `_grpc.pb.go` file.
6. Verify `gen/go/go.mod` exists.
7. Run `go build ./...` in `gen/go/` and verify exit code is 0.

**Expected result:** Proto generation succeeds and output compiles as valid Go.

**Test command:**
```bash
cd tests/setup && go test -run TestGoCodeGeneration -v
```

### TS-01-4: Rust Workspace Compilation

**Traces to:** 01-REQ-4

**Objective:** Verify that the Rust workspace compiles successfully.

**Preconditions:** Rust 1.75+ toolchain is installed.

**Steps:**

1. Verify `rhivos/Cargo.toml` exists and is a valid workspace manifest.
2. Verify workspace members include: `locking-service`, `cloud-gateway-client`, `update-service`, `parking-operator-adaptor`.
3. For each member crate, verify a `Cargo.toml` and `src/main.rs` exist.
4. Run `cargo check --workspace` in `rhivos/` and verify exit code is 0.

**Expected result:** All four skeleton crates are valid workspace members and compile.

**Test command:**
```bash
cd tests/setup && go test -run TestRustWorkspace -v
```

### TS-01-5: Makefile Targets

**Traces to:** 01-REQ-5

**Objective:** Verify that all required Makefile targets exist and execute.

**Preconditions:** GNU Make, Rust toolchain, Go toolchain installed.

**Steps:**

1. Verify `Makefile` exists at the repository root.
2. For each target (`build`, `test`, `lint`, `check`, `proto`, `infra-up`, `infra-down`, `clean`), run `make -n <target>` and verify exit code is 0 (target is defined).
3. Run `make build` and verify exit code is 0.
4. Run `make test` and verify exit code is 0.
5. Run `make lint` and verify exit code is 0.

**Expected result:** All eight targets are defined. Build, test, and lint succeed on skeleton code.

**Test command:**
```bash
cd tests/setup && go test -run TestMakefileTargets -v
```

### TS-01-6: Docker-Compose Services

**Traces to:** 01-REQ-6

**Objective:** Verify that docker-compose defines the required services and they start correctly.

**Preconditions:** Docker or Podman with compose support is installed.

**Steps:**

1. Verify `infra/docker-compose.yml` exists.
2. Parse the YAML and verify a service named `nats` (or containing "nats") is defined.
3. Parse the YAML and verify a service named `kuksa-databroker` (or containing "kuksa") is defined.
4. Verify the NATS service maps port 4222.
5. Verify the Kuksa Databroker service maps port 55556.
6. Run `make infra-up` and wait up to 30 seconds.
7. Verify TCP connectivity to `localhost:4222`.
8. Verify TCP connectivity to `localhost:55556`.
9. Run `make infra-down` and verify containers are stopped.

**Expected result:** Both services start and are reachable on their designated ports.

**Test command:**
```bash
cd tests/setup && go test -run TestDockerComposeServices -v -timeout 120s
```

### TS-01-7: Go Workspace

**Traces to:** 01-REQ-7

**Objective:** Verify that `go.work` references all Go modules and enables cross-module resolution.

**Preconditions:** Go 1.22+ installed.

**Steps:**

1. Verify `go.work` exists at the repository root.
2. Read `go.work` and verify it contains `use` directives for:
   - `backend/parking-fee-service`
   - `backend/cloud-gateway`
   - `mock/parking-app-cli`
   - `mock/companion-app-cli`
   - `gen/go`
   - `tests/setup`
3. Run `go work sync` from the repository root and verify exit code is 0.
4. For each listed module, verify the corresponding `go.mod` exists.

**Expected result:** Go workspace is valid and all modules are resolvable.

**Test command:**
```bash
cd tests/setup && go test -run TestGoWorkspace -v
```

## Correctness Property Tests

### TS-01-P1: Proto Compilation Roundtrip

**Traces to:** CP-01

**Objective:** Verify that generated Go code compiles after proto regeneration.

**Steps:**

1. Run `make proto`.
2. Run `go build ./...` in `gen/go/`.
3. Verify both commands exit with code 0.

**Expected result:** Full roundtrip from proto to compilable Go code succeeds.

**Test command:**
```bash
cd tests/setup && go test -run TestProtoRoundtrip -v
```

### TS-01-P2: Rust Workspace Consistency

**Traces to:** CP-02

**Objective:** Verify all workspace members exist and the workspace checks cleanly.

**Steps:**

1. Parse `rhivos/Cargo.toml` for member list.
2. For each member, verify its directory and `Cargo.toml` exist.
3. Run `cargo check --workspace` in `rhivos/`.
4. Verify exit code is 0.

**Expected result:** All workspace members are present and the workspace compiles.

**Test command:**
```bash
cd tests/setup && go test -run TestRustWorkspaceConsistency -v
```

### TS-01-P3: Build Idempotency

**Traces to:** CP-03

**Objective:** Verify that running `make build` twice succeeds.

**Steps:**

1. Run `make build` and record exit code.
2. Run `make build` again and record exit code.
3. Both exit codes SHALL be 0.

**Expected result:** Build is idempotent.

**Test command:**
```bash
cd tests/setup && go test -run TestBuildIdempotency -v
```

### TS-01-P4: Infrastructure Lifecycle Symmetry

**Traces to:** CP-04

**Objective:** Verify clean start/stop cycle leaves no orphan containers.

**Steps:**

1. Run `make infra-up`.
2. Verify containers are running (via `docker-compose ps`).
3. Run `make infra-down`.
4. Verify no containers from the compose file are running.
5. Run `make infra-down` again (idempotent teardown).
6. Verify exit code is 0.

**Expected result:** Infrastructure starts and stops cleanly, and teardown is idempotent.

**Test command:**
```bash
cd tests/setup && go test -run TestInfraLifecycle -v -timeout 120s
```

### TS-01-P5: Go Workspace Module Resolution

**Traces to:** CP-05

**Objective:** Verify cross-module imports resolve within the workspace.

**Steps:**

1. Run `go work sync` from the repository root.
2. Run `go build ./...` from the repository root (workspace-aware).
3. Verify exit code is 0.

**Expected result:** All workspace modules build successfully with local resolution.

**Test command:**
```bash
cd tests/setup && go test -run TestGoWorkspaceResolution -v
```

### TS-01-P6: Clean Build from Scratch

**Traces to:** CP-06

**Objective:** Verify that clean followed by build succeeds.

**Steps:**

1. Run `make clean`.
2. Verify exit code is 0.
3. Run `make build`.
4. Verify exit code is 0.
5. Verify source files (`.go`, `.rs`, `.proto`) still exist (clean did not delete them).

**Expected result:** Clean + build roundtrip succeeds without data loss.

**Test command:**
```bash
cd tests/setup && go test -run TestCleanBuild -v
```

### TS-01-P7: Lint Clean State

**Traces to:** CP-07

**Objective:** Verify skeleton code passes all linters.

**Steps:**

1. Run `make lint`.
2. Verify exit code is 0.
3. Verify no warnings on stdout/stderr.

**Expected result:** Skeleton code is lint-clean.

**Test command:**
```bash
cd tests/setup && go test -run TestLintClean -v
```

## Edge Case / Error Path Tests

### TS-01-E1: Missing Proto Tool Detection

**Traces to:** 01-REQ-3 edge case, Error Handling table

**Objective:** Verify `make proto` detects missing tools.

**Steps:**

1. Temporarily rename `protoc-gen-go` binary (or adjust PATH to exclude it).
2. Run `make proto`.
3. Verify exit code is non-zero.
4. Verify stderr contains a message about the missing tool.
5. Restore the original state.

**Expected result:** Clear error message when proto tools are missing.

**Note:** This test may be skipped in CI environments where tool installation cannot be manipulated.

**Test command:**
```bash
cd tests/setup && go test -run TestMissingProtoTool -v
```

### TS-01-E2: Make Clean on Empty Build

**Traces to:** 01-REQ-5 edge case

**Objective:** Verify `make clean` succeeds when no build artifacts exist.

**Steps:**

1. Run `make clean` (to ensure clean state).
2. Run `make clean` again.
3. Verify exit code is 0.

**Expected result:** Clean is idempotent and never fails on empty state.

**Test command:**
```bash
cd tests/setup && go test -run TestCleanIdempotent -v
```

### TS-01-E3: Infra Down When Not Running

**Traces to:** 01-REQ-6 edge case

**Objective:** Verify `make infra-down` succeeds when no containers are running.

**Steps:**

1. Ensure no infrastructure containers are running.
2. Run `make infra-down`.
3. Verify exit code is 0.

**Expected result:** Teardown is safe to call at any time.

**Test command:**
```bash
cd tests/setup && go test -run TestInfraDownWhenNotRunning -v
```

### TS-01-E4: Placeholder Directories

**Traces to:** 01-REQ-1 edge case

**Objective:** Verify placeholder directories contain only `.gitkeep`.

**Steps:**

1. List contents of `aaos/`.
2. Verify only `.gitkeep` is present.
3. List contents of `android/`.
4. Verify only `.gitkeep` is present.

**Expected result:** Placeholder directories are minimal.

**Test command:**
```bash
cd tests/setup && go test -run TestPlaceholderDirectories -v
```

## Test Summary Matrix

| Test ID | Traces To | Category | Requires Infra |
|---------|-----------|----------|----------------|
| TS-01-1 | 01-REQ-1 | Structure | No |
| TS-01-2 | 01-REQ-2 | Proto | No |
| TS-01-3 | 01-REQ-3 | Code Gen | No |
| TS-01-4 | 01-REQ-4 | Rust | No |
| TS-01-5 | 01-REQ-5 | Build | No |
| TS-01-6 | 01-REQ-6 | Infra | Yes |
| TS-01-7 | 01-REQ-7 | Go Workspace | No |
| TS-01-P1 | CP-01 | Proto | No |
| TS-01-P2 | CP-02 | Rust | No |
| TS-01-P3 | CP-03 | Build | No |
| TS-01-P4 | CP-04 | Infra | Yes |
| TS-01-P5 | CP-05 | Go Workspace | No |
| TS-01-P6 | CP-06 | Build | No |
| TS-01-P7 | CP-07 | Lint | No |
| TS-01-E1 | 01-REQ-3 | Error Path | No |
| TS-01-E2 | 01-REQ-5 | Edge Case | No |
| TS-01-E3 | 01-REQ-6 | Edge Case | Yes |
| TS-01-E4 | 01-REQ-1 | Edge Case | No |
