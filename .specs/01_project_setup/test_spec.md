# Test Specification: Project Setup

## Overview

All tests for the project setup spec are integration tests implemented as Go tests in `tests/setup/` that invoke shell commands to verify structural and behavioral invariants. Since this spec produces no runtime logic (only scaffolding, configs, and build targets), there are no unit tests in the traditional sense. Property tests verify structural invariants via exhaustive enumeration rather than random generation.

Test cases map 1:1 to acceptance criteria in requirements.md and correctness properties in design.md.

## Test Cases

### TS-01-1: Top-Level Directories Exist

**Requirement:** 01-REQ-1.1
**Type:** integration
**Description:** Verify that all required top-level directories exist in the repository.

**Preconditions:**
- Repository is checked out at the project root.

**Input:**
- List of expected directories: `rhivos`, `backend`, `android`, `mobile`, `mock`, `proto`, `deployments`, `tests`.

**Expected:**
- Each directory exists and is a directory (not a file).

**Assertion pseudocode:**
```
FOR EACH dir IN ["rhivos", "backend", "android", "mobile", "mock", "proto", "deployments", "tests"]:
    ASSERT path_exists(dir) AND is_directory(dir)
```

### TS-01-2: Rust Component Subdirectories Exist

**Requirement:** 01-REQ-1.2
**Type:** integration
**Description:** Verify that all Rust component subdirectories exist under `rhivos/`.

**Preconditions:**
- Repository is checked out.

**Input:**
- Expected subdirectories: `locking-service`, `cloud-gateway-client`, `update-service`, `parking-operator-adaptor`, `mock-sensors`.

**Expected:**
- Each subdirectory exists under `rhivos/`.

**Assertion pseudocode:**
```
FOR EACH dir IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
    ASSERT path_exists("rhivos/" + dir) AND is_directory("rhivos/" + dir)
```

### TS-01-3: Go Backend Subdirectories Exist

**Requirement:** 01-REQ-1.3
**Type:** integration
**Description:** Verify that all Go backend subdirectories exist under `backend/`.

**Preconditions:**
- Repository is checked out.

**Input:**
- Expected subdirectories: `parking-fee-service`, `cloud-gateway`.

**Expected:**
- Each subdirectory exists under `backend/`.

**Assertion pseudocode:**
```
FOR EACH dir IN ["parking-fee-service", "cloud-gateway"]:
    ASSERT path_exists("backend/" + dir) AND is_directory("backend/" + dir)
```

### TS-01-4: Mock CLI Subdirectories Exist

**Requirement:** 01-REQ-1.4
**Type:** integration
**Description:** Verify that all mock CLI app subdirectories exist under `mock/`.

**Preconditions:**
- Repository is checked out.

**Input:**
- Expected subdirectories: `parking-app-cli`, `companion-app-cli`, `parking-operator`.

**Expected:**
- Each subdirectory exists under `mock/`.

**Assertion pseudocode:**
```
FOR EACH dir IN ["parking-app-cli", "companion-app-cli", "parking-operator"]:
    ASSERT path_exists("mock/" + dir) AND is_directory("mock/" + dir)
```

### TS-01-5: AAOS Placeholder Exists

**Requirement:** 01-REQ-1.5
**Type:** integration
**Description:** Verify that the `android/` placeholder directory exists.

**Preconditions:**
- Repository is checked out.

**Input:**
- Directory path: `android/`.

**Expected:**
- Directory exists and contains at least a README.md.

**Assertion pseudocode:**
```
ASSERT path_exists("android") AND is_directory("android")
ASSERT path_exists("android/README.md")
```

### TS-01-6: Flutter Placeholder Exists

**Requirement:** 01-REQ-1.6
**Type:** integration
**Description:** Verify that the `mobile/` placeholder directory exists.

**Preconditions:**
- Repository is checked out.

**Input:**
- Directory path: `mobile/`.

**Expected:**
- Directory exists and contains at least a README.md.

**Assertion pseudocode:**
```
ASSERT path_exists("mobile") AND is_directory("mobile")
ASSERT path_exists("mobile/README.md")
```

### TS-01-7: Cargo Workspace Configuration

**Requirement:** 01-REQ-2.1
**Type:** integration
**Description:** Verify that the Cargo workspace root defines all expected members.

**Preconditions:**
- `rhivos/Cargo.toml` exists.

**Input:**
- Parse `rhivos/Cargo.toml` for workspace members.

**Expected:**
- Members list contains: `locking-service`, `cloud-gateway-client`, `update-service`, `parking-operator-adaptor`, `mock-sensors`.

**Assertion pseudocode:**
```
content = read_file("rhivos/Cargo.toml")
FOR EACH member IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
    ASSERT member IN content
```

### TS-01-8: Cargo Build Succeeds

**Requirement:** 01-REQ-2.2
**Type:** integration
**Description:** Verify that `cargo build` in the Rust workspace completes without errors.

**Preconditions:**
- Rust toolchain 1.75+ installed.
- `rhivos/Cargo.toml` workspace is configured.

**Input:**
- Run `cargo build` from `rhivos/` directory.

**Expected:**
- Command exits with code 0.

**Assertion pseudocode:**
```
exit_code = run_command("cargo build", cwd="rhivos/")
ASSERT exit_code == 0
```

### TS-01-9: Cargo Test Succeeds

**Requirement:** 01-REQ-2.3
**Type:** integration
**Description:** Verify that `cargo test` in the Rust workspace discovers and runs tests.

**Preconditions:**
- Rust workspace compiles.

**Input:**
- Run `cargo test` from `rhivos/` directory.

**Expected:**
- Command exits with code 0.
- Output contains "test result: ok".

**Assertion pseudocode:**
```
exit_code, output = run_command("cargo test", cwd="rhivos/")
ASSERT exit_code == 0
ASSERT "test result: ok" IN output
```

### TS-01-10: Mock Sensors Binary Targets

**Requirement:** 01-REQ-2.4
**Type:** integration
**Description:** Verify that the mock-sensors crate defines three binary targets.

**Preconditions:**
- `rhivos/mock-sensors/Cargo.toml` exists.

**Input:**
- Check for binary source files.

**Expected:**
- Files exist: `src/bin/location-sensor.rs`, `src/bin/speed-sensor.rs`, `src/bin/door-sensor.rs`.

**Assertion pseudocode:**
```
FOR EACH bin IN ["location-sensor", "speed-sensor", "door-sensor"]:
    ASSERT path_exists("rhivos/mock-sensors/src/bin/" + bin + ".rs")
```

### TS-01-11: Go Workspace File

**Requirement:** 01-REQ-3.1
**Type:** integration
**Description:** Verify that `go.work` exists and references all expected modules.

**Preconditions:**
- Repository is checked out.

**Input:**
- Parse `go.work` for `use` directives.

**Expected:**
- `go.work` contains use directives for `backend/`, `mock/`, `tests/setup/`.

**Assertion pseudocode:**
```
content = read_file("go.work")
FOR EACH module IN ["./backend", "./mock", "./tests/setup"]:
    ASSERT module IN content
```

### TS-01-12: Go Build Succeeds

**Requirement:** 01-REQ-3.2
**Type:** integration
**Description:** Verify that `go build ./...` from the repo root completes without errors.

**Preconditions:**
- Go 1.22+ installed.
- `go.work` is configured.

**Input:**
- Run `go build ./...` from repo root.

**Expected:**
- Command exits with code 0.

**Assertion pseudocode:**
```
exit_code = run_command("go build ./...")
ASSERT exit_code == 0
```

### TS-01-13: Go Test Succeeds

**Requirement:** 01-REQ-3.3
**Type:** integration
**Description:** Verify that `go test ./...` discovers and runs tests.

**Preconditions:**
- Go workspace compiles.

**Input:**
- Run `go test ./...` from repo root.

**Expected:**
- Command exits with code 0.
- Output contains "ok" for each test package.

**Assertion pseudocode:**
```
exit_code, output = run_command("go test ./...")
ASSERT exit_code == 0
ASSERT "ok" IN output
```

### TS-01-14: Rust Skeleton Exit Behavior

**Requirement:** 01-REQ-4.1
**Type:** integration
**Description:** Verify that each Rust skeleton binary prints its name and exits 0.

**Preconditions:**
- Rust workspace is built.

**Input:**
- Execute each Rust binary with no arguments.

**Expected:**
- Exit code 0 for each binary.
- Output contains the component name.

**Assertion pseudocode:**
```
FOR EACH binary IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "location-sensor", "speed-sensor", "door-sensor"]:
    exit_code, output = run_command("rhivos/target/debug/" + binary)
    ASSERT exit_code == 0
    ASSERT binary IN output
```

### TS-01-15: Go Skeleton Exit Behavior

**Requirement:** 01-REQ-4.2
**Type:** integration
**Description:** Verify that each Go skeleton binary prints its name and exits 0.

**Preconditions:**
- Go workspace is built.

**Input:**
- Execute each Go binary with no arguments.

**Expected:**
- Exit code 0 for each binary.
- Output contains the component name.

**Assertion pseudocode:**
```
FOR EACH binary IN ["parking-fee-service", "cloud-gateway", "parking-app-cli", "companion-app-cli", "parking-operator"]:
    exit_code, output = run_binary(binary)
    ASSERT exit_code == 0
    ASSERT binary IN output
```

### TS-01-16: Rust Binary List

**Requirement:** 01-REQ-4.3
**Type:** integration
**Description:** Verify that all expected Rust binaries are produced by `cargo build`.

**Preconditions:**
- `cargo build` has run successfully.

**Input:**
- Check for binary files in `rhivos/target/debug/`.

**Expected:**
- All 7 binaries exist: locking-service, cloud-gateway-client, update-service, parking-operator-adaptor, location-sensor, speed-sensor, door-sensor.

**Assertion pseudocode:**
```
FOR EACH binary IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "location-sensor", "speed-sensor", "door-sensor"]:
    ASSERT path_exists("rhivos/target/debug/" + binary)
```

### TS-01-17: Go Binary List

**Requirement:** 01-REQ-4.4
**Type:** integration
**Description:** Verify that all expected Go binaries can be built.

**Preconditions:**
- Go workspace is configured.

**Input:**
- Build each Go binary individually.

**Expected:**
- `go build` succeeds for each: `backend/parking-fee-service`, `backend/cloud-gateway`, `mock/parking-app-cli`, `mock/companion-app-cli`, `mock/parking-operator`.

**Assertion pseudocode:**
```
FOR EACH pkg IN ["./backend/parking-fee-service", "./backend/cloud-gateway", "./mock/parking-app-cli", "./mock/companion-app-cli", "./mock/parking-operator"]:
    exit_code = run_command("go build " + pkg)
    ASSERT exit_code == 0
```

### TS-01-18: Proto Files Exist

**Requirement:** 01-REQ-5.1
**Type:** integration
**Description:** Verify that all required proto files exist in `proto/`.

**Preconditions:**
- Repository is checked out.

**Input:**
- Expected files: `common.proto`, `update_service.proto`, `parking_adaptor.proto`.

**Expected:**
- All three files exist.

**Assertion pseudocode:**
```
FOR EACH file IN ["common.proto", "update_service.proto", "parking_adaptor.proto"]:
    ASSERT path_exists("proto/" + file)
```

### TS-01-19: Common Proto Types

**Requirement:** 01-REQ-5.2
**Type:** integration
**Description:** Verify that `common.proto` defines AdapterState, AdapterInfo, and ErrorDetails.

**Preconditions:**
- `proto/common.proto` exists.

**Input:**
- Parse proto file content.

**Expected:**
- Contains `enum AdapterState`, `message AdapterInfo`, `message ErrorDetails`.

**Assertion pseudocode:**
```
content = read_file("proto/common.proto")
ASSERT "enum AdapterState" IN content
ASSERT "message AdapterInfo" IN content
ASSERT "message ErrorDetails" IN content
```

### TS-01-20: UpdateService Proto RPCs

**Requirement:** 01-REQ-5.3
**Type:** integration
**Description:** Verify that `update_service.proto` defines the UpdateService with all 5 RPCs.

**Preconditions:**
- `proto/update_service.proto` exists.

**Input:**
- Parse proto file content.

**Expected:**
- Contains `service UpdateService` with RPCs: `InstallAdapter`, `WatchAdapterStates`, `ListAdapters`, `RemoveAdapter`, `GetAdapterStatus`.

**Assertion pseudocode:**
```
content = read_file("proto/update_service.proto")
ASSERT "service UpdateService" IN content
FOR EACH rpc IN ["InstallAdapter", "WatchAdapterStates", "ListAdapters", "RemoveAdapter", "GetAdapterStatus"]:
    ASSERT "rpc " + rpc IN content
```

### TS-01-21: ParkingAdaptor Proto RPCs

**Requirement:** 01-REQ-5.4
**Type:** integration
**Description:** Verify that `parking_adaptor.proto` defines the ParkingAdaptor service with all 4 RPCs.

**Preconditions:**
- `proto/parking_adaptor.proto` exists.

**Input:**
- Parse proto file content.

**Expected:**
- Contains `service ParkingAdaptor` with RPCs: `StartSession`, `StopSession`, `GetStatus`, `GetRate`.

**Assertion pseudocode:**
```
content = read_file("proto/parking_adaptor.proto")
ASSERT "service ParkingAdaptor" IN content
FOR EACH rpc IN ["StartSession", "StopSession", "GetStatus", "GetRate"]:
    ASSERT "rpc " + rpc IN content
```

### TS-01-22: Proto Generation Produces Go Code

**Requirement:** 01-REQ-5.5
**Type:** integration
**Description:** Verify that `make proto` generates Go code into the expected packages.

**Preconditions:**
- `protoc`, `protoc-gen-go`, `protoc-gen-go-grpc` installed.
- `gen/go/` directory is empty or does not exist.

**Input:**
- Run `make proto`.

**Expected:**
- Directories exist: `gen/go/commonpb/`, `gen/go/updateservicepb/`, `gen/go/parkingadaptorpb/`.
- Each contains at least one `.go` file.

**Assertion pseudocode:**
```
run_command("make proto")
FOR EACH pkg IN ["commonpb", "updateservicepb", "parkingadaptorpb"]:
    ASSERT path_exists("gen/go/" + pkg)
    ASSERT count_files("gen/go/" + pkg + "/*.go") > 0
```

### TS-01-23: Generated Go Code Compiles

**Requirement:** 01-REQ-5.6
**Type:** integration
**Description:** Verify that generated Go code compiles without errors.

**Preconditions:**
- `make proto` has been run.

**Input:**
- Run `go build ./gen/go/...`.

**Expected:**
- Command exits with code 0.

**Assertion pseudocode:**
```
exit_code = run_command("go build ./gen/go/...")
ASSERT exit_code == 0
```

### TS-01-24: Compose File Defines Services

**Requirement:** 01-REQ-6.1
**Type:** integration
**Description:** Verify that `deployments/compose.yml` defines NATS and Databroker services.

**Preconditions:**
- Repository is checked out.

**Input:**
- Parse `deployments/compose.yml`.

**Expected:**
- File contains service definitions for `nats` (port 4222) and `databroker` (port 55556).

**Assertion pseudocode:**
```
content = read_file("deployments/compose.yml")
ASSERT "nats" IN content
ASSERT "4222" IN content
ASSERT "databroker" IN content
ASSERT "55556" IN content
```

### TS-01-25: Infrastructure Starts

**Requirement:** 01-REQ-6.2
**Type:** integration
**Description:** Verify that `make infra-up` starts NATS and Databroker containers.

**Preconditions:**
- Podman is running.

**Input:**
- Run `make infra-up`.

**Expected:**
- Command exits with code 0.
- `podman ps` shows containers for NATS and Databroker.

**Assertion pseudocode:**
```
exit_code = run_command("make infra-up")
ASSERT exit_code == 0
output = run_command("podman ps")
ASSERT "nats" IN output
ASSERT "databroker" IN output OR "kuksa" IN output
```

### TS-01-26: Infrastructure Stops

**Requirement:** 01-REQ-6.3
**Type:** integration
**Description:** Verify that `make infra-down` stops and removes infrastructure containers.

**Preconditions:**
- Infrastructure is running via `make infra-up`.

**Input:**
- Run `make infra-down`.

**Expected:**
- Command exits with code 0.
- `podman ps` no longer shows NATS or Databroker containers.

**Assertion pseudocode:**
```
run_command("make infra-up")
exit_code = run_command("make infra-down")
ASSERT exit_code == 0
output = run_command("podman ps")
ASSERT "nats" NOT IN output
ASSERT "databroker" NOT IN output AND "kuksa" NOT IN output
```

### TS-01-27: NATS Config Exists

**Requirement:** 01-REQ-6.4
**Type:** integration
**Description:** Verify that the NATS server configuration file exists.

**Preconditions:**
- Repository is checked out.

**Input:**
- Check for `deployments/nats/nats-server.conf`.

**Expected:**
- File exists and contains `port: 4222`.

**Assertion pseudocode:**
```
ASSERT path_exists("deployments/nats/nats-server.conf")
content = read_file("deployments/nats/nats-server.conf")
ASSERT "4222" IN content
```

### TS-01-28: VSS Overlay Exists

**Requirement:** 01-REQ-6.5
**Type:** integration
**Description:** Verify that the VSS overlay defines custom signals.

**Preconditions:**
- Repository is checked out.

**Input:**
- Parse `deployments/vss-overlay.json`.

**Expected:**
- Contains definitions for `SessionActive`, `Lock`, `Response` signals.

**Assertion pseudocode:**
```
content = read_file("deployments/vss-overlay.json")
ASSERT "SessionActive" IN content
ASSERT "Lock" IN content
ASSERT "Response" IN content
```

### TS-01-29: Makefile Build Target

**Requirement:** 01-REQ-7.1
**Type:** integration
**Description:** Verify that `make build` compiles all Rust and Go components.

**Preconditions:**
- Rust and Go toolchains installed.

**Input:**
- Run `make build`.

**Expected:**
- Command exits with code 0.

**Assertion pseudocode:**
```
exit_code = run_command("make build")
ASSERT exit_code == 0
```

### TS-01-30: Makefile Test Target

**Requirement:** 01-REQ-7.2
**Type:** integration
**Description:** Verify that `make test` runs all unit tests.

**Preconditions:**
- Project builds successfully.

**Input:**
- Run `make test`.

**Expected:**
- Command exits with code 0.

**Assertion pseudocode:**
```
exit_code = run_command("make test")
ASSERT exit_code == 0
```

### TS-01-31: Makefile Lint Target

**Requirement:** 01-REQ-7.3
**Type:** integration
**Description:** Verify that `make lint` runs cargo clippy and go vet.

**Preconditions:**
- Project builds successfully.

**Input:**
- Run `make lint`.

**Expected:**
- Command exits with code 0.

**Assertion pseudocode:**
```
exit_code = run_command("make lint")
ASSERT exit_code == 0
```

### TS-01-32: Makefile Check Target

**Requirement:** 01-REQ-7.4
**Type:** integration
**Description:** Verify that `make check` runs build, test, and lint in sequence.

**Preconditions:**
- Rust and Go toolchains installed.

**Input:**
- Run `make check`.

**Expected:**
- Command exits with code 0.

**Assertion pseudocode:**
```
exit_code = run_command("make check")
ASSERT exit_code == 0
```

### TS-01-33: Makefile Clean Target

**Requirement:** 01-REQ-7.5
**Type:** integration
**Description:** Verify that `make clean` removes build artifacts.

**Preconditions:**
- `make build` has been run.

**Input:**
- Run `make clean`.

**Expected:**
- Command exits with code 0.
- `rhivos/target/` directory does not exist.
- Go binaries are removed.

**Assertion pseudocode:**
```
run_command("make build")
exit_code = run_command("make clean")
ASSERT exit_code == 0
ASSERT NOT path_exists("rhivos/target")
```

### TS-01-34: Makefile Proto and Infra Targets Exist

**Requirement:** 01-REQ-7.6
**Type:** integration
**Description:** Verify that proto, infra-up, and infra-down targets are defined.

**Preconditions:**
- Root Makefile exists.

**Input:**
- Parse Makefile for target definitions.

**Expected:**
- Makefile contains targets: `proto`, `infra-up`, `infra-down`.

**Assertion pseudocode:**
```
content = read_file("Makefile")
ASSERT "proto:" IN content OR "proto :" IN content
ASSERT "infra-up:" IN content OR "infra-up :" IN content
ASSERT "infra-down:" IN content OR "infra-down :" IN content
```

### TS-01-35: Cargo Test Discovers Tests

**Requirement:** 01-REQ-8.1
**Type:** integration
**Description:** Verify that each Rust crate has at least one test discovered by cargo test.

**Preconditions:**
- Rust workspace compiles.

**Input:**
- Run `cargo test` from `rhivos/` and parse output.

**Expected:**
- Output shows test execution for each crate (non-zero test count).

**Assertion pseudocode:**
```
exit_code, output = run_command("cargo test", cwd="rhivos/")
ASSERT exit_code == 0
FOR EACH crate IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
    ASSERT crate IN output
```

### TS-01-36: Go Test Discovers Tests

**Requirement:** 01-REQ-8.2
**Type:** integration
**Description:** Verify that each Go module has at least one test discovered by go test.

**Preconditions:**
- Go workspace compiles.

**Input:**
- Run `go test ./...` from repo root and parse output.

**Expected:**
- Output shows "ok" for test packages in backend, mock, and tests/setup.

**Assertion pseudocode:**
```
exit_code, output = run_command("go test ./...")
ASSERT exit_code == 0
ASSERT "ok" IN output
```

### TS-01-37: Setup Tests Module

**Requirement:** 01-REQ-8.3
**Type:** integration
**Description:** Verify that `tests/setup/` contains a Go module with test files.

**Preconditions:**
- Repository is checked out.

**Input:**
- Check for `tests/setup/go.mod` and `tests/setup/*_test.go`.

**Expected:**
- go.mod exists and at least one test file exists.

**Assertion pseudocode:**
```
ASSERT path_exists("tests/setup/go.mod")
ASSERT count_files("tests/setup/*_test.go") > 0
```

### TS-01-38: Make Test Runs All Tests

**Requirement:** 01-REQ-8.4
**Type:** integration
**Description:** Verify that `make test` executes both Rust and Go tests.

**Preconditions:**
- Project builds successfully.

**Input:**
- Run `make test` and capture output.

**Expected:**
- Output contains evidence of both `cargo test` and `go test` execution.

**Assertion pseudocode:**
```
exit_code, output = run_command("make test")
ASSERT exit_code == 0
ASSERT "test result" IN output  # cargo test output
ASSERT "ok" IN output           # go test output
```

## Edge Case Tests

### TS-01-E1: Missing Directory Detection

**Requirement:** 01-REQ-1.E1
**Type:** integration
**Description:** Verify that the build system detects a missing required directory.

**Preconditions:**
- Repository is checked out.
- One required directory is temporarily renamed.

**Input:**
- Rename `proto/` to `proto_backup/`.
- Run setup verification tests.

**Expected:**
- Test reports the missing `proto/` directory.

**Assertion pseudocode:**
```
rename("proto", "proto_backup")
exit_code, output = run_command("go test ./tests/setup/ -run TestDirectories")
ASSERT exit_code != 0
ASSERT "proto" IN output
rename("proto_backup", "proto")
```

### TS-01-E2: Missing Cargo Member

**Requirement:** 01-REQ-2.E1
**Type:** integration
**Description:** Verify that a missing Cargo workspace member causes a build failure.

**Preconditions:**
- Cargo workspace is configured.
- One member crate's Cargo.toml is temporarily removed.

**Input:**
- Remove `rhivos/locking-service/Cargo.toml`.
- Run `cargo build` from `rhivos/`.

**Expected:**
- Command exits with non-zero code.
- Error message references the missing member.

**Assertion pseudocode:**
```
rename("rhivos/locking-service/Cargo.toml", "rhivos/locking-service/Cargo.toml.bak")
exit_code, output = run_command("cargo build", cwd="rhivos/")
ASSERT exit_code != 0
rename("rhivos/locking-service/Cargo.toml.bak", "rhivos/locking-service/Cargo.toml")
```

### TS-01-E3: Missing Go Module

**Requirement:** 01-REQ-3.E1
**Type:** integration
**Description:** Verify that a missing Go module causes a workspace build failure.

**Preconditions:**
- Go workspace is configured.
- One module's go.mod is temporarily removed.

**Input:**
- Remove `backend/go.mod`.
- Run `go build ./...`.

**Expected:**
- Command exits with non-zero code.

**Assertion pseudocode:**
```
rename("backend/go.mod", "backend/go.mod.bak")
exit_code = run_command("go build ./...")
ASSERT exit_code != 0
rename("backend/go.mod.bak", "backend/go.mod")
```

### TS-01-E4: Skeleton Unrecognized Flag

**Requirement:** 01-REQ-4.E1
**Type:** integration
**Description:** Verify that skeleton binaries exit 0 even with unrecognized flags.

**Preconditions:**
- Binaries are built.

**Input:**
- Run each binary with `--unknown-flag`.

**Expected:**
- Exit code 0 for all binaries.

**Assertion pseudocode:**
```
FOR EACH binary IN all_binaries:
    exit_code = run_command(binary + " --unknown-flag")
    ASSERT exit_code == 0
```

### TS-01-E5: Missing Protoc

**Requirement:** 01-REQ-5.E1
**Type:** integration
**Description:** Verify that `make proto` fails gracefully when protoc is not in PATH.

**Preconditions:**
- `protoc` is temporarily removed from PATH.

**Input:**
- Run `make proto` with modified PATH.

**Expected:**
- Command exits with non-zero code.
- Output contains error about missing `protoc`.

**Assertion pseudocode:**
```
exit_code, output = run_command("PATH=/usr/bin make proto")
ASSERT exit_code != 0
ASSERT "protoc" IN output
```

### TS-01-E6: Missing Podman

**Requirement:** 01-REQ-6.E1
**Type:** integration
**Description:** Verify that `make infra-up` fails gracefully when Podman is unavailable.

**Preconditions:**
- Podman is temporarily removed from PATH.

**Input:**
- Run `make infra-up` with modified PATH.

**Expected:**
- Command exits with non-zero code.
- Output indicates Podman is required.

**Assertion pseudocode:**
```
exit_code, output = run_command("PATH=/usr/bin make infra-up")
ASSERT exit_code != 0
ASSERT "podman" IN output OR "Podman" IN output
```

### TS-01-E7: Idempotent Infrastructure Start

**Requirement:** 01-REQ-6.E2
**Type:** integration
**Description:** Verify that running `make infra-up` twice does not create duplicate containers.

**Preconditions:**
- Podman is running.

**Input:**
- Run `make infra-up` twice.
- Count running containers.

**Expected:**
- Exactly one NATS container and one Databroker container.

**Assertion pseudocode:**
```
run_command("make infra-up")
run_command("make infra-up")
output = run_command("podman ps")
ASSERT count_occurrences(output, "nats") == 1
ASSERT count_occurrences(output, "databroker") <= 1 OR count_occurrences(output, "kuksa") <= 1
run_command("make infra-down")
```

### TS-01-E8: Missing Toolchain

**Requirement:** 01-REQ-7.E1
**Type:** integration
**Description:** Verify that Makefile targets fail with a clear error when a toolchain is missing.

**Preconditions:**
- One toolchain is temporarily removed from PATH.

**Input:**
- Run `make build` with `cargo` removed from PATH.

**Expected:**
- Command exits with non-zero code.
- Output references the missing toolchain.

**Assertion pseudocode:**
```
exit_code, output = run_command("PATH=$(echo $PATH | sed 's|.*cargo.*||') make build")
ASSERT exit_code != 0
```

### TS-01-E9: No Tests Warning

**Requirement:** 01-REQ-8.E1
**Type:** integration
**Description:** Verify that the test runner warns when no tests exist in a component.

**Preconditions:**
- A component's test file is temporarily removed.

**Input:**
- Remove test file from one Go module.
- Run `go test` for that module.

**Expected:**
- Output contains "no test files" or similar warning.

**Assertion pseudocode:**
```
rename("backend/parking-fee-service/main_test.go", "backend/parking-fee-service/main_test.go.bak")
exit_code, output = run_command("go test ./backend/parking-fee-service/")
ASSERT "no test files" IN output
rename("backend/parking-fee-service/main_test.go.bak", "backend/parking-fee-service/main_test.go")
```

## Property Test Cases

### TS-01-P1: Rust Workspace Completeness

**Property:** Property 1 from design.md
**Validates:** 01-REQ-2.1, 01-REQ-2.2, 01-REQ-4.3
**Type:** property
**Description:** For every Rust component in the PRD, verify it is a workspace member and produces a binary.

**For any:** Rust component name in the set {locking-service, cloud-gateway-client, update-service, parking-operator-adaptor, mock-sensors}
**Invariant:** The component is listed as a workspace member AND `cargo build -p {component}` succeeds.

**Assertion pseudocode:**
```
FOR ANY component IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
    workspace = read_file("rhivos/Cargo.toml")
    ASSERT component IN workspace
    exit_code = run_command("cargo build -p " + component, cwd="rhivos/")
    ASSERT exit_code == 0
```

### TS-01-P2: Go Workspace Completeness

**Property:** Property 2 from design.md
**Validates:** 01-REQ-3.1, 01-REQ-3.2, 01-REQ-4.4
**Type:** property
**Description:** For every Go component in the PRD, verify it is in the workspace and builds.

**For any:** Go binary path in the set {backend/parking-fee-service, backend/cloud-gateway, mock/parking-app-cli, mock/companion-app-cli, mock/parking-operator}
**Invariant:** The component's module is in `go.work` AND `go build ./{path}` succeeds.

**Assertion pseudocode:**
```
FOR ANY path IN ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator"]:
    exit_code = run_command("go build ./" + path)
    ASSERT exit_code == 0
```

### TS-01-P3: Skeleton Exit Behavior

**Property:** Property 3 from design.md
**Validates:** 01-REQ-4.1, 01-REQ-4.2, 01-REQ-4.E1
**Type:** property
**Description:** For any skeleton binary, invoking it with any arguments exits 0.

**For any:** Binary in the full set of skeleton binaries, argument list in {[], ["--help"], ["--unknown"], ["foo", "bar"]}
**Invariant:** Exit code is always 0.

**Assertion pseudocode:**
```
FOR ANY binary IN all_skeleton_binaries:
    FOR ANY args IN [[], ["--help"], ["--unknown"], ["foo", "bar"]]:
        exit_code = run_command(binary, args)
        ASSERT exit_code == 0
```

### TS-01-P4: Proto Generation Idempotency

**Property:** Property 4 from design.md
**Validates:** 01-REQ-5.5, 01-REQ-5.6
**Type:** property
**Description:** Running `make proto` twice produces byte-identical output.

**For any:** Sequence of 2 consecutive `make proto` invocations
**Invariant:** SHA-256 checksums of all generated files are identical after each invocation.

**Assertion pseudocode:**
```
run_command("make proto")
checksums_1 = sha256_all_files("gen/go/")
run_command("make proto")
checksums_2 = sha256_all_files("gen/go/")
ASSERT checksums_1 == checksums_2
```

### TS-01-P5: Infrastructure Idempotency

**Property:** Property 5 from design.md
**Validates:** 01-REQ-6.2, 01-REQ-6.E2
**Type:** property
**Description:** Multiple infra-up invocations result in exactly one container per service.

**For any:** Number of consecutive `make infra-up` calls in {1, 2, 3}
**Invariant:** Exactly 1 NATS container and 1 Databroker container are running.

**Assertion pseudocode:**
```
FOR ANY n IN [1, 2, 3]:
    FOR i IN range(n):
        run_command("make infra-up")
    output = run_command("podman ps")
    ASSERT count_containers(output, "nats") == 1
    ASSERT count_containers(output, "databroker") == 1
    run_command("make infra-down")
```

### TS-01-P6: Directory Structure Completeness

**Property:** Property 6 from design.md
**Validates:** 01-REQ-1.1, 01-REQ-1.2, 01-REQ-1.3, 01-REQ-1.4, 01-REQ-1.5, 01-REQ-1.6
**Type:** property
**Description:** Every directory in the PRD structure exists and contains at least one file.

**For any:** Directory path in the full set of expected directories
**Invariant:** The path exists, is a directory, and contains at least one non-.gitkeep file.

**Assertion pseudocode:**
```
FOR ANY dir IN all_expected_directories:
    ASSERT path_exists(dir) AND is_directory(dir)
    files = list_files(dir)
    non_gitkeep = [f for f in files if f != ".gitkeep"]
    ASSERT len(non_gitkeep) > 0
```

### TS-01-P7: Test Runner Discovery

**Property:** Property 7 from design.md
**Validates:** 01-REQ-8.1, 01-REQ-8.2, 01-REQ-8.4
**Type:** property
**Description:** Every component has at least one test discoverable by its test runner.

**For any:** Component (Rust crate or Go module) in the project
**Invariant:** The test runner discovers and reports at least one test.

**Assertion pseudocode:**
```
# Rust crates
exit_code, output = run_command("cargo test", cwd="rhivos/")
ASSERT exit_code == 0
ASSERT "test result: ok" IN output

# Go modules
exit_code, output = run_command("go test ./...")
ASSERT exit_code == 0
```

### TS-01-P8: Proto Service Completeness

**Property:** Property 8 from design.md
**Validates:** 01-REQ-5.2, 01-REQ-5.3, 01-REQ-5.4
**Type:** property
**Description:** Every gRPC service and its RPCs are defined in the proto files.

**For any:** (service_name, rpc_list) pair in the expected set
**Invariant:** The corresponding proto file defines the service with all listed RPCs.

**Assertion pseudocode:**
```
expected = {
    "UpdateService": ["InstallAdapter", "WatchAdapterStates", "ListAdapters", "RemoveAdapter", "GetAdapterStatus"],
    "ParkingAdaptor": ["StartSession", "StopSession", "GetStatus", "GetRate"]
}
FOR ANY (service, rpcs) IN expected:
    content = read_proto_for_service(service)
    ASSERT "service " + service IN content
    FOR ANY rpc IN rpcs:
        ASSERT "rpc " + rpc IN content
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 01-REQ-1.1 | TS-01-1 | integration |
| 01-REQ-1.2 | TS-01-2 | integration |
| 01-REQ-1.3 | TS-01-3 | integration |
| 01-REQ-1.4 | TS-01-4 | integration |
| 01-REQ-1.5 | TS-01-5 | integration |
| 01-REQ-1.6 | TS-01-6 | integration |
| 01-REQ-1.E1 | TS-01-E1 | integration |
| 01-REQ-2.1 | TS-01-7 | integration |
| 01-REQ-2.2 | TS-01-8 | integration |
| 01-REQ-2.3 | TS-01-9 | integration |
| 01-REQ-2.4 | TS-01-10 | integration |
| 01-REQ-2.E1 | TS-01-E2 | integration |
| 01-REQ-3.1 | TS-01-11 | integration |
| 01-REQ-3.2 | TS-01-12 | integration |
| 01-REQ-3.3 | TS-01-13 | integration |
| 01-REQ-3.E1 | TS-01-E3 | integration |
| 01-REQ-4.1 | TS-01-14 | integration |
| 01-REQ-4.2 | TS-01-15 | integration |
| 01-REQ-4.3 | TS-01-16 | integration |
| 01-REQ-4.4 | TS-01-17 | integration |
| 01-REQ-4.E1 | TS-01-E4 | integration |
| 01-REQ-5.1 | TS-01-18 | integration |
| 01-REQ-5.2 | TS-01-19 | integration |
| 01-REQ-5.3 | TS-01-20 | integration |
| 01-REQ-5.4 | TS-01-21 | integration |
| 01-REQ-5.5 | TS-01-22 | integration |
| 01-REQ-5.6 | TS-01-23 | integration |
| 01-REQ-5.E1 | TS-01-E5 | integration |
| 01-REQ-6.1 | TS-01-24 | integration |
| 01-REQ-6.2 | TS-01-25 | integration |
| 01-REQ-6.3 | TS-01-26 | integration |
| 01-REQ-6.4 | TS-01-27 | integration |
| 01-REQ-6.5 | TS-01-28 | integration |
| 01-REQ-6.E1 | TS-01-E6 | integration |
| 01-REQ-6.E2 | TS-01-E7 | integration |
| 01-REQ-7.1 | TS-01-29 | integration |
| 01-REQ-7.2 | TS-01-30 | integration |
| 01-REQ-7.3 | TS-01-31 | integration |
| 01-REQ-7.4 | TS-01-32 | integration |
| 01-REQ-7.5 | TS-01-33 | integration |
| 01-REQ-7.6 | TS-01-34 | integration |
| 01-REQ-7.E1 | TS-01-E8 | integration |
| 01-REQ-8.1 | TS-01-35 | integration |
| 01-REQ-8.2 | TS-01-36 | integration |
| 01-REQ-8.3 | TS-01-37 | integration |
| 01-REQ-8.4 | TS-01-38 | integration |
| 01-REQ-8.E1 | TS-01-E9 | integration |
| Property 1 | TS-01-P1 | property |
| Property 2 | TS-01-P2 | property |
| Property 3 | TS-01-P3 | property |
| Property 4 | TS-01-P4 | property |
| Property 5 | TS-01-P5 | property |
| Property 6 | TS-01-P6 | property |
| Property 7 | TS-01-P7 | property |
| Property 8 | TS-01-P8 | property |
