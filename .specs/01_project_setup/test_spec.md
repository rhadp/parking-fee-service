# Test Specification: Project Setup

## Overview

This test specification translates every acceptance criterion and correctness property from the project setup spec into concrete, verifiable test contracts. Tests are organized into three categories: acceptance criterion tests (TS-01-N), property tests (TS-01-PN), and edge case tests (TS-01-EN).

Because this spec covers repository structure, build system, and infrastructure rather than application logic, many tests are shell-script-based or use the build tools themselves as test harnesses. A dedicated `tests/setup/` directory at the repo root contains these verification scripts.

## Test Cases

### TS-01-1: Top-Level Directory Structure Exists

**Requirement:** 01-REQ-1.1
**Type:** integration
**Description:** Verify all required top-level directories exist in the repository.

**Preconditions:**
- Repository is checked out.

**Input:**
- List of required directories: `rhivos/`, `backend/`, `android/`, `mobile/`, `mock/`, `proto/`, `deployments/`

**Expected:**
- All seven directories exist and are accessible.

**Assertion pseudocode:**
```
FOR EACH dir IN ["rhivos", "backend", "android", "mobile", "mock", "proto", "deployments"]:
    ASSERT directory_exists(REPO_ROOT / dir)
```

### TS-01-2: Rust Service Subdirectories Exist

**Requirement:** 01-REQ-1.2
**Type:** integration
**Description:** Verify all required Rust service subdirectories exist under rhivos/.

**Preconditions:**
- Repository is checked out.

**Input:**
- List of required subdirectories: `locking-service/`, `cloud-gateway-client/`, `update-service/`, `parking-operator-adaptor/`, `mock-sensors/`

**Expected:**
- All five subdirectories exist under `rhivos/`.

**Assertion pseudocode:**
```
FOR EACH dir IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
    ASSERT directory_exists(REPO_ROOT / "rhivos" / dir)
```

### TS-01-3: Go Backend Subdirectories Exist

**Requirement:** 01-REQ-1.3
**Type:** integration
**Description:** Verify all required Go backend subdirectories exist under backend/.

**Preconditions:**
- Repository is checked out.

**Input:**
- List of required subdirectories: `parking-fee-service/`, `cloud-gateway/`

**Expected:**
- Both subdirectories exist under `backend/`.

**Assertion pseudocode:**
```
FOR EACH dir IN ["parking-fee-service", "cloud-gateway"]:
    ASSERT directory_exists(REPO_ROOT / "backend" / dir)
```

### TS-01-4: Mock CLI Subdirectories Exist

**Requirement:** 01-REQ-1.4
**Type:** integration
**Description:** Verify all required mock CLI app subdirectories exist under mock/.

**Preconditions:**
- Repository is checked out.

**Input:**
- List of required subdirectories: `parking-app-cli/`, `companion-app-cli/`, `parking-operator/`

**Expected:**
- All three subdirectories exist under `mock/`.

**Assertion pseudocode:**
```
FOR EACH dir IN ["parking-app-cli", "companion-app-cli", "parking-operator"]:
    ASSERT directory_exists(REPO_ROOT / "mock" / dir)
```

### TS-01-5: Rust Workspace Cargo.toml Exists and Lists All Members

**Requirement:** 01-REQ-2.1
**Type:** integration
**Description:** Verify the Rust workspace Cargo.toml exists and references all expected crates.

**Preconditions:**
- Repository is checked out.

**Input:**
- Path: `rhivos/Cargo.toml`
- Expected members: `locking-service`, `cloud-gateway-client`, `update-service`, `parking-operator-adaptor`, `mock-sensors`

**Expected:**
- File exists and contains a `[workspace]` section listing all five members.

**Assertion pseudocode:**
```
content = read_file(REPO_ROOT / "rhivos" / "Cargo.toml")
ASSERT "[workspace]" IN content
FOR EACH member IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
    ASSERT member IN content
```

### TS-01-6: Rust Workspace Builds Successfully

**Requirement:** 01-REQ-2.2
**Type:** integration
**Description:** Verify `cargo build` succeeds in the Rust workspace.

**Preconditions:**
- Rust toolchain installed.
- Repository is checked out.

**Input:**
- Command: `cargo build` in `rhivos/`

**Expected:**
- Exit code 0, no error output.

**Assertion pseudocode:**
```
result = run_command("cargo build", cwd=REPO_ROOT / "rhivos")
ASSERT result.exit_code == 0
ASSERT "error" NOT IN result.stderr
```

### TS-01-7: Rust Workspace Tests Pass

**Requirement:** 01-REQ-2.3
**Type:** integration
**Description:** Verify `cargo test` succeeds in the Rust workspace.

**Preconditions:**
- Rust toolchain installed.
- Repository is checked out.

**Input:**
- Command: `cargo test` in `rhivos/`

**Expected:**
- Exit code 0, all tests pass.

**Assertion pseudocode:**
```
result = run_command("cargo test", cwd=REPO_ROOT / "rhivos")
ASSERT result.exit_code == 0
ASSERT "test result: ok" IN result.stdout
```

### TS-01-8: Go Backend Workspace File Exists

**Requirement:** 01-REQ-3.1
**Type:** integration
**Description:** Verify the Go workspace file exists in backend/ and references all modules.

**Preconditions:**
- Repository is checked out.

**Input:**
- Path: `backend/go.work`
- Expected modules: `parking-fee-service`, `cloud-gateway`

**Expected:**
- File exists and references both modules.

**Assertion pseudocode:**
```
content = read_file(REPO_ROOT / "backend" / "go.work")
FOR EACH module IN ["parking-fee-service", "cloud-gateway"]:
    ASSERT module IN content
```

### TS-01-9: Go Mock Workspace File Exists

**Requirement:** 01-REQ-3.2
**Type:** integration
**Description:** Verify the Go workspace file exists in mock/ and references all modules.

**Preconditions:**
- Repository is checked out.

**Input:**
- Path: `mock/go.work`
- Expected modules: `parking-app-cli`, `companion-app-cli`, `parking-operator`

**Expected:**
- File exists and references all three modules.

**Assertion pseudocode:**
```
content = read_file(REPO_ROOT / "mock" / "go.work")
FOR EACH module IN ["parking-app-cli", "companion-app-cli", "parking-operator"]:
    ASSERT module IN content
```

### TS-01-10: Go Modules Build Successfully

**Requirement:** 01-REQ-3.3
**Type:** integration
**Description:** Verify `go build ./...` succeeds for each Go module.

**Preconditions:**
- Go toolchain installed.
- Repository is checked out.

**Input:**
- Commands: `go build ./...` in each Go module directory

**Expected:**
- Exit code 0 for each module.

**Assertion pseudocode:**
```
FOR EACH module_dir IN ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator"]:
    result = run_command("go build ./...", cwd=REPO_ROOT / module_dir)
    ASSERT result.exit_code == 0
```

### TS-01-11: Go Module Tests Pass

**Requirement:** 01-REQ-3.4
**Type:** integration
**Description:** Verify `go test ./...` succeeds for each Go module.

**Preconditions:**
- Go toolchain installed.
- Repository is checked out.

**Input:**
- Commands: `go test ./...` in each Go module directory

**Expected:**
- Exit code 0 for each module, tests pass.

**Assertion pseudocode:**
```
FOR EACH module_dir IN ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator"]:
    result = run_command("go test ./...", cwd=REPO_ROOT / module_dir)
    ASSERT result.exit_code == 0
```

### TS-01-12: Rust Skeleton Binaries Exit with Code 0

**Requirement:** 01-REQ-4.1
**Type:** integration
**Description:** Verify each Rust skeleton binary runs and exits with code 0.

**Preconditions:**
- Rust workspace has been built (`cargo build` in `rhivos/`).

**Input:**
- Execute each binary: locking-service, cloud-gateway-client, update-service, parking-operator-adaptor

**Expected:**
- Each exits with code 0 and produces stdout output.

**Assertion pseudocode:**
```
FOR EACH binary IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor"]:
    result = run_command(REPO_ROOT / "rhivos" / "target" / "debug" / binary)
    ASSERT result.exit_code == 0
    ASSERT len(result.stdout) > 0
```

### TS-01-13: Go Skeleton Binaries Exit with Code 0

**Requirement:** 01-REQ-4.2
**Type:** integration
**Description:** Verify each Go skeleton binary runs and exits with code 0.

**Preconditions:**
- Go modules have been built.

**Input:**
- Execute `go run .` in each Go service directory

**Expected:**
- Each exits with code 0 and produces stdout output.

**Assertion pseudocode:**
```
FOR EACH module_dir IN ["backend/parking-fee-service", "backend/cloud-gateway"]:
    result = run_command("go run .", cwd=REPO_ROOT / module_dir)
    ASSERT result.exit_code == 0
    ASSERT len(result.stdout) > 0
```

### TS-01-14: Each Skeleton Has At Least One Passing Test

**Requirement:** 01-REQ-4.3
**Type:** integration
**Description:** Verify every skeleton component has at least one test that passes.

**Preconditions:**
- Toolchains installed.

**Input:**
- Run tests per component and count test results.

**Expected:**
- Each component reports at least one passing test.

**Assertion pseudocode:**
```
FOR EACH rust_crate IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
    result = run_command("cargo test -p " + rust_crate, cwd=REPO_ROOT / "rhivos")
    ASSERT "1 passed" IN result.stdout OR "test result: ok" IN result.stdout

FOR EACH go_module IN ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator"]:
    result = run_command("go test -v ./...", cwd=REPO_ROOT / go_module)
    ASSERT "PASS" IN result.stdout
```

### TS-01-15: Proto Directory Contains Valid Proto3 File

**Requirement:** 01-REQ-5.1, 01-REQ-5.2
**Type:** integration
**Description:** Verify proto/ contains at least one valid proto3 file.

**Preconditions:**
- protoc is installed.

**Input:**
- Search for .proto files in `proto/`
- Run protoc validation on each

**Expected:**
- At least one .proto file exists with `syntax = "proto3"` and passes protoc validation.

**Assertion pseudocode:**
```
proto_files = glob(REPO_ROOT / "proto" / "**" / "*.proto")
ASSERT len(proto_files) >= 1
FOR EACH proto_file IN proto_files:
    content = read_file(proto_file)
    ASSERT 'syntax = "proto3"' IN content
    result = run_command("protoc --proto_path=proto --lint_out=. " + proto_file)
    ASSERT result.exit_code == 0
```

### TS-01-16: Root Makefile Has All Required Targets

**Requirement:** 01-REQ-6.1
**Type:** integration
**Description:** Verify the root Makefile defines all required targets.

**Preconditions:**
- Repository is checked out.

**Input:**
- Path: `Makefile`
- Required targets: `build`, `test`, `lint`, `clean`, `infra-up`, `infra-down`

**Expected:**
- All six targets are defined in the Makefile.

**Assertion pseudocode:**
```
content = read_file(REPO_ROOT / "Makefile")
FOR EACH target IN ["build", "test", "lint", "clean", "infra-up", "infra-down"]:
    ASSERT target + ":" IN content
```

### TS-01-17: make build Succeeds

**Requirement:** 01-REQ-6.2
**Type:** integration
**Description:** Verify `make build` compiles all components successfully.

**Preconditions:**
- Rust and Go toolchains installed.

**Input:**
- Command: `make build`

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
result = run_command("make build", cwd=REPO_ROOT)
ASSERT result.exit_code == 0
```

### TS-01-18: make test Succeeds

**Requirement:** 01-REQ-6.3
**Type:** integration
**Description:** Verify `make test` runs all tests successfully.

**Preconditions:**
- Rust and Go toolchains installed.

**Input:**
- Command: `make test`

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
result = run_command("make test", cwd=REPO_ROOT)
ASSERT result.exit_code == 0
```

### TS-01-19: make clean Removes Build Artifacts

**Requirement:** 01-REQ-6.4
**Type:** integration
**Description:** Verify `make clean` removes all build artifacts.

**Preconditions:**
- `make build` has been run first.

**Input:**
- Command: `make build` then `make clean`

**Expected:**
- Rust `target/` directory is removed. Go build cache is cleared.

**Assertion pseudocode:**
```
run_command("make build", cwd=REPO_ROOT)
ASSERT directory_exists(REPO_ROOT / "rhivos" / "target")
run_command("make clean", cwd=REPO_ROOT)
ASSERT NOT directory_exists(REPO_ROOT / "rhivos" / "target")
```

### TS-01-20: Compose File Defines NATS and Kuksa Services

**Requirement:** 01-REQ-7.1
**Type:** integration
**Description:** Verify the compose file defines both required infrastructure services.

**Preconditions:**
- Repository is checked out.

**Input:**
- Path: `deployments/compose.yml`

**Expected:**
- File exists and defines services for NATS and Kuksa Databroker.

**Assertion pseudocode:**
```
content = read_file(REPO_ROOT / "deployments" / "compose.yml")
ASSERT "nats" IN content
ASSERT "kuksa" IN content OR "databroker" IN content
ASSERT "4222" IN content
ASSERT "55555" IN content
```

### TS-01-21: Infrastructure Starts and Services Are Reachable

**Requirement:** 01-REQ-7.2
**Type:** integration
**Description:** Verify `make infra-up` starts containers and services are reachable.

**Preconditions:**
- Podman is installed and running.

**Input:**
- Command: `make infra-up`

**Expected:**
- Both containers are running. NATS is reachable on port 4222. Kuksa is reachable on port 55555.

**Assertion pseudocode:**
```
run_command("make infra-up", cwd=REPO_ROOT)
sleep(10)  // Allow containers to start
ASSERT tcp_connect("localhost", 4222) == SUCCESS
ASSERT tcp_connect("localhost", 55555) == SUCCESS
run_command("make infra-down", cwd=REPO_ROOT)  // Cleanup
```

### TS-01-22: Infrastructure Stops Cleanly

**Requirement:** 01-REQ-7.3
**Type:** integration
**Description:** Verify `make infra-down` stops and removes all containers.

**Preconditions:**
- `make infra-up` has been run.

**Input:**
- Command: `make infra-down`

**Expected:**
- No containers from the compose file remain running.

**Assertion pseudocode:**
```
run_command("make infra-up", cwd=REPO_ROOT)
run_command("make infra-down", cwd=REPO_ROOT)
result = run_command("podman compose -f deployments/compose.yml ps -q")
ASSERT result.stdout.strip() == ""
```

### TS-01-23: Mock CLI Apps Build Successfully

**Requirement:** 01-REQ-8.1, 01-REQ-8.2, 01-REQ-8.3
**Type:** integration
**Description:** Verify all mock CLI apps compile to named binaries.

**Preconditions:**
- Go toolchain installed.

**Input:**
- Build each mock app.

**Expected:**
- Each produces a binary.

**Assertion pseudocode:**
```
FOR EACH app IN ["parking-app-cli", "companion-app-cli", "parking-operator"]:
    result = run_command("go build -o " + app + " .", cwd=REPO_ROOT / "mock" / app)
    ASSERT result.exit_code == 0
    ASSERT file_exists(REPO_ROOT / "mock" / app / app)
```

### TS-01-24: Mock CLI Apps Print Usage Without Arguments

**Requirement:** 01-REQ-8.4
**Type:** integration
**Description:** Verify each mock CLI app prints usage and exits 0 when run without arguments.

**Preconditions:**
- Mock apps have been built.

**Input:**
- Execute each mock app binary without arguments.

**Expected:**
- Each prints usage text and exits with code 0.

**Assertion pseudocode:**
```
FOR EACH app IN ["parking-app-cli", "companion-app-cli", "parking-operator"]:
    result = run_command("go run .", cwd=REPO_ROOT / "mock" / app)
    ASSERT result.exit_code == 0
    ASSERT "usage" IN result.stdout.lower() OR "Usage" IN result.stdout
```

### TS-01-25: make test Runs All Component Tests

**Requirement:** 01-REQ-9.3
**Type:** integration
**Description:** Verify `make test` runs tests across all Rust and Go components.

**Preconditions:**
- All toolchains installed.

**Input:**
- Command: `make test`

**Expected:**
- Exit code 0, output includes test results from both Rust and Go.

**Assertion pseudocode:**
```
result = run_command("make test", cwd=REPO_ROOT)
ASSERT result.exit_code == 0
ASSERT "test result" IN result.stdout OR "ok" IN result.stdout  // Rust output
ASSERT "PASS" IN result.stdout  // Go output
```

### TS-01-26: Mock Sensors Crate Builds

**Requirement:** 01-REQ-10.1, 01-REQ-10.2
**Type:** integration
**Description:** Verify mock-sensors crate compiles as part of the Rust workspace.

**Preconditions:**
- Rust toolchain installed.

**Input:**
- Command: `cargo build -p mock-sensors` in `rhivos/`

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
result = run_command("cargo build -p mock-sensors", cwd=REPO_ROOT / "rhivos")
ASSERT result.exit_code == 0
```

## Property Test Cases

### TS-01-P1: Directory Completeness

**Property:** Property 1 from design.md
**Validates:** 01-REQ-1.1, 01-REQ-1.2, 01-REQ-1.3, 01-REQ-1.4
**Type:** property
**Description:** Every required directory exists and is non-empty.

**For any:** directory in the full list of required directories (top-level and nested)
**Invariant:** The directory exists and contains at least one file or subdirectory.

**Assertion pseudocode:**
```
all_dirs = [
    "rhivos", "backend", "android", "mobile", "mock", "proto", "deployments",
    "rhivos/locking-service", "rhivos/cloud-gateway-client", "rhivos/update-service",
    "rhivos/parking-operator-adaptor", "rhivos/mock-sensors",
    "backend/parking-fee-service", "backend/cloud-gateway",
    "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator"
]
FOR EACH dir IN all_dirs:
    ASSERT directory_exists(REPO_ROOT / dir)
    ASSERT count_entries(REPO_ROOT / dir) > 0
```

### TS-01-P2: Build Determinism

**Property:** Property 2 from design.md
**Validates:** 01-REQ-2.2, 01-REQ-3.3, 01-REQ-6.2
**Type:** property
**Description:** Two consecutive `make build` runs both succeed.

**For any:** clean repository state
**Invariant:** `make build` returns exit code 0 on two consecutive runs.

**Assertion pseudocode:**
```
run_command("make clean", cwd=REPO_ROOT)
result1 = run_command("make build", cwd=REPO_ROOT)
result2 = run_command("make build", cwd=REPO_ROOT)
ASSERT result1.exit_code == 0
ASSERT result2.exit_code == 0
```

### TS-01-P3: Test Discoverability

**Property:** Property 3 from design.md
**Validates:** 01-REQ-2.3, 01-REQ-3.4, 01-REQ-4.3, 01-REQ-9.1, 01-REQ-9.2
**Type:** property
**Description:** Every component has at least one discoverable, passing test.

**For any:** component directory (Rust crate or Go module)
**Invariant:** The test runner discovers and passes at least one test.

**Assertion pseudocode:**
```
FOR EACH rust_crate IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
    result = run_command("cargo test -p " + rust_crate, cwd=REPO_ROOT / "rhivos")
    ASSERT result.exit_code == 0
    ASSERT "0 passed" NOT IN result.stdout

FOR EACH go_module IN ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator"]:
    result = run_command("go test -v ./...", cwd=REPO_ROOT / go_module)
    ASSERT result.exit_code == 0
    ASSERT "PASS" IN result.stdout
```

### TS-01-P4: Skeleton Exit Behavior

**Property:** Property 4 from design.md
**Validates:** 01-REQ-4.1, 01-REQ-4.2, 01-REQ-4.E1
**Type:** property
**Description:** Every skeleton binary exits with code 0 and produces stdout output.

**For any:** skeleton binary in the project
**Invariant:** Exit code is 0 and stdout is non-empty.

**Assertion pseudocode:**
```
FOR EACH binary IN all_skeleton_binaries:
    result = run_binary(binary)
    ASSERT result.exit_code == 0
    ASSERT len(result.stdout) > 0
```

### TS-01-P5: Infrastructure Lifecycle

**Property:** Property 5 from design.md
**Validates:** 01-REQ-7.2, 01-REQ-7.3
**Type:** property
**Description:** Infrastructure up/down cycle leaves no orphaned containers.

**For any:** execution of infra-up followed by infra-down
**Invariant:** No containers from the compose project remain after infra-down.

**Assertion pseudocode:**
```
run_command("make infra-up", cwd=REPO_ROOT)
run_command("make infra-down", cwd=REPO_ROOT)
result = run_command("podman compose -f deployments/compose.yml ps -q")
ASSERT result.stdout.strip() == ""
```

### TS-01-P6: Proto Validity

**Property:** Property 6 from design.md
**Validates:** 01-REQ-5.1, 01-REQ-5.2
**Type:** property
**Description:** All .proto files are syntactically valid proto3.

**For any:** .proto file in the proto/ directory
**Invariant:** The file declares proto3 syntax and passes protoc validation.

**Assertion pseudocode:**
```
proto_files = glob(REPO_ROOT / "proto" / "**" / "*.proto")
ASSERT len(proto_files) >= 1
FOR EACH f IN proto_files:
    content = read_file(f)
    ASSERT 'syntax = "proto3"' IN content
```

### TS-01-P7: Mock CLI Usage Output

**Property:** Property 7 from design.md
**Validates:** 01-REQ-8.1, 01-REQ-8.2, 01-REQ-8.3, 01-REQ-8.4
**Type:** property
**Description:** Every mock CLI app prints usage when run without arguments.

**For any:** mock CLI app binary
**Invariant:** Exit code is 0 and stdout contains usage information.

**Assertion pseudocode:**
```
FOR EACH app IN ["parking-app-cli", "companion-app-cli", "parking-operator"]:
    result = run_command("go run .", cwd=REPO_ROOT / "mock" / app)
    ASSERT result.exit_code == 0
    ASSERT len(result.stdout) > 0
```

## Edge Case Tests

### TS-01-E1: Android Placeholder Directory

**Requirement:** 01-REQ-1.E1
**Type:** unit
**Description:** Verify android/ is a placeholder with only a README.

**Preconditions:**
- Repository is checked out.

**Input:**
- Directory: `android/`

**Expected:**
- Directory exists and contains only README.md.

**Assertion pseudocode:**
```
entries = list_directory(REPO_ROOT / "android")
ASSERT "README.md" IN entries
ASSERT len(entries) == 1
```

### TS-01-E2: Mobile Placeholder Directory

**Requirement:** 01-REQ-1.E2
**Type:** unit
**Description:** Verify mobile/ is a placeholder with only a README.

**Preconditions:**
- Repository is checked out.

**Input:**
- Directory: `mobile/`

**Expected:**
- Directory exists and contains only README.md.

**Assertion pseudocode:**
```
entries = list_directory(REPO_ROOT / "mobile")
ASSERT "README.md" IN entries
ASSERT len(entries) == 1
```

### TS-01-E3: Skeleton Binary Without Config

**Requirement:** 01-REQ-4.E1
**Type:** integration
**Description:** Verify skeleton binaries exit cleanly without configuration.

**Preconditions:**
- Binaries built. No config files present.

**Input:**
- Run each binary in an empty environment.

**Expected:**
- Exit code 0, no panic or crash.

**Assertion pseudocode:**
```
FOR EACH binary IN all_skeleton_binaries:
    result = run_binary(binary, env={})
    ASSERT result.exit_code == 0
    ASSERT "panic" NOT IN result.stderr
```

### TS-01-E4: make build Reports Failure Clearly

**Requirement:** 01-REQ-6.E1
**Type:** integration
**Description:** Verify `make build` exits non-zero when a component fails to build.

**Preconditions:**
- Introduce a syntax error in one component source file.

**Input:**
- Command: `make build` with a broken component.

**Expected:**
- Non-zero exit code and error message in output.

**Assertion pseudocode:**
```
inject_syntax_error(REPO_ROOT / "rhivos" / "locking-service" / "src" / "main.rs")
result = run_command("make build", cwd=REPO_ROOT)
ASSERT result.exit_code != 0
ASSERT "error" IN result.stderr OR "error" IN result.stdout
restore_file(REPO_ROOT / "rhivos" / "locking-service" / "src" / "main.rs")
```

### TS-01-E5: Mock CLI Unknown Subcommand

**Requirement:** 01-REQ-8.E1
**Type:** integration
**Description:** Verify mock CLI apps report error for unknown subcommands.

**Preconditions:**
- Mock apps are built.

**Input:**
- Run each mock app with an unknown subcommand: `./app nonexistent`

**Expected:**
- Non-zero exit code and error message listing valid subcommands.

**Assertion pseudocode:**
```
FOR EACH app IN ["parking-app-cli", "companion-app-cli", "parking-operator"]:
    result = run_command("go run . nonexistent", cwd=REPO_ROOT / "mock" / app)
    ASSERT result.exit_code != 0
    ASSERT "unknown" IN result.stderr.lower() OR "invalid" IN result.stderr.lower()
```

### TS-01-E6: No Tests Reported Gracefully

**Requirement:** 01-REQ-9.E1
**Type:** unit
**Description:** Verify test runners handle packages with no tests gracefully.

**Preconditions:**
- A Go package or Rust module with no test functions.

**Input:**
- Run `go test` on a package with no test files.

**Expected:**
- Exit code 0, output indicates no test files (not a failure).

**Assertion pseudocode:**
```
// Go reports "no test files" for packages without tests - this is expected behavior
// Verified by the fact that `go test ./...` succeeds even when some subpackages lack tests
result = run_command("go test ./...", cwd=REPO_ROOT / "backend/parking-fee-service")
ASSERT result.exit_code == 0
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 01-REQ-1.1 | TS-01-1 | integration |
| 01-REQ-1.2 | TS-01-2 | integration |
| 01-REQ-1.3 | TS-01-3 | integration |
| 01-REQ-1.4 | TS-01-4 | integration |
| 01-REQ-1.E1 | TS-01-E1 | unit |
| 01-REQ-1.E2 | TS-01-E2 | unit |
| 01-REQ-2.1 | TS-01-5 | integration |
| 01-REQ-2.2 | TS-01-6 | integration |
| 01-REQ-2.3 | TS-01-7 | integration |
| 01-REQ-3.1 | TS-01-8 | integration |
| 01-REQ-3.2 | TS-01-9 | integration |
| 01-REQ-3.3 | TS-01-10 | integration |
| 01-REQ-3.4 | TS-01-11 | integration |
| 01-REQ-4.1 | TS-01-12 | integration |
| 01-REQ-4.2 | TS-01-13 | integration |
| 01-REQ-4.3 | TS-01-14 | integration |
| 01-REQ-4.E1 | TS-01-E3 | integration |
| 01-REQ-5.1 | TS-01-15 | integration |
| 01-REQ-5.2 | TS-01-15 | integration |
| 01-REQ-5.E1 | TS-01-E4 | integration |
| 01-REQ-6.1 | TS-01-16 | integration |
| 01-REQ-6.2 | TS-01-17 | integration |
| 01-REQ-6.3 | TS-01-18 | integration |
| 01-REQ-6.4 | TS-01-19 | integration |
| 01-REQ-6.E1 | TS-01-E4 | integration |
| 01-REQ-7.1 | TS-01-20 | integration |
| 01-REQ-7.2 | TS-01-21 | integration |
| 01-REQ-7.3 | TS-01-22 | integration |
| 01-REQ-8.1 | TS-01-23 | integration |
| 01-REQ-8.2 | TS-01-23 | integration |
| 01-REQ-8.3 | TS-01-23 | integration |
| 01-REQ-8.4 | TS-01-24 | integration |
| 01-REQ-8.E1 | TS-01-E5 | integration |
| 01-REQ-9.1 | TS-01-7 | integration |
| 01-REQ-9.2 | TS-01-11 | integration |
| 01-REQ-9.3 | TS-01-25 | integration |
| 01-REQ-9.E1 | TS-01-E6 | unit |
| 01-REQ-10.1 | TS-01-26 | integration |
| 01-REQ-10.2 | TS-01-26 | integration |
| Property 1 | TS-01-P1 | property |
| Property 2 | TS-01-P2 | property |
| Property 3 | TS-01-P3 | property |
| Property 4 | TS-01-P4 | property |
| Property 5 | TS-01-P5 | property |
| Property 6 | TS-01-P6 | property |
| Property 7 | TS-01-P7 | property |
