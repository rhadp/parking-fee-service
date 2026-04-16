# Test Spec: Project Setup

## Overview

This test specification validates the project setup for the parking-fee-service monorepo. It covers directory structure, workspace configurations (Rust Cargo and Go), skeleton binaries, proto definitions, Makefile targets, infrastructure configuration, placeholder tests, setup verification tests, and proto code generation. Tests are organized into acceptance tests for individual requirements, property tests for cross-cutting invariants, edge case tests for error handling, and integration smoke tests for end-to-end verification.

## Test Cases

### TS-01-1: Repository contains rhivos directory structure

- **Requirement:** 01-REQ-1.1
- **Type:** Acceptance
- **Description:** Verify the `rhivos/` directory contains all required subdirectories for Rust services.
- **Preconditions:** Repository is checked out.
- **Input:** Filesystem inspection of `rhivos/`.
- **Expected:** Subdirectories `locking-service/`, `cloud-gateway-client/`, `update-service/`, `parking-operator-adaptor/`, `mock-sensors/` all exist.
- **Assertion pseudocode:**
  ```
  for dir in ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
      assert path_exists("rhivos/" + dir)
  ```

### TS-01-2: Repository contains backend directory structure

- **Requirement:** 01-REQ-1.2
- **Type:** Acceptance
- **Description:** Verify the `backend/` directory contains all required subdirectories for Go services.
- **Preconditions:** Repository is checked out.
- **Input:** Filesystem inspection of `backend/`.
- **Expected:** Subdirectories `parking-fee-service/` and `cloud-gateway/` exist.
- **Assertion pseudocode:**
  ```
  for dir in ["parking-fee-service", "cloud-gateway"]:
      assert path_exists("backend/" + dir)
  ```

### TS-01-3: Android and mobile placeholder directories exist

- **Requirement:** 01-REQ-1.3, 01-REQ-1.4
- **Type:** Acceptance
- **Description:** Verify placeholder directories exist with README files.
- **Preconditions:** Repository is checked out.
- **Input:** Filesystem inspection of `android/` and `mobile/`.
- **Expected:** Both directories exist, each containing a `README.md`.
- **Assertion pseudocode:**
  ```
  assert path_exists("android/README.md")
  assert path_exists("mobile/README.md")
  assert file_contains("android/README.md", "PARKING_APP")
  assert file_contains("mobile/README.md", "COMPANION_APP")
  ```

### TS-01-4: Mock directory structure exists

- **Requirement:** 01-REQ-1.5
- **Type:** Acceptance
- **Description:** Verify the `mock/` directory contains all required subdirectories.
- **Preconditions:** Repository is checked out.
- **Input:** Filesystem inspection of `mock/`.
- **Expected:** Subdirectories `parking-app-cli/`, `companion-app-cli/`, `parking-operator/` exist.
- **Assertion pseudocode:**
  ```
  for dir in ["parking-app-cli", "companion-app-cli", "parking-operator"]:
      assert path_exists("mock/" + dir)
  ```

### TS-01-5: Proto and deployments directories exist

- **Requirement:** 01-REQ-1.6, 01-REQ-1.7
- **Type:** Acceptance
- **Description:** Verify `proto/` and `deployments/` directories exist.
- **Preconditions:** Repository is checked out.
- **Input:** Filesystem inspection.
- **Expected:** Both directories exist.
- **Assertion pseudocode:**
  ```
  assert path_exists("proto/")
  assert path_exists("deployments/")
  ```

### TS-01-6: Tests setup directory exists

- **Requirement:** 01-REQ-1.8
- **Type:** Acceptance
- **Description:** Verify `tests/setup/` directory exists.
- **Preconditions:** Repository is checked out.
- **Input:** Filesystem inspection.
- **Expected:** `tests/setup/` exists and contains a Go module.
- **Assertion pseudocode:**
  ```
  assert path_exists("tests/setup/")
  assert path_exists("tests/setup/go.mod")
  ```

### TS-01-7: Cargo workspace is correctly configured

- **Requirement:** 01-REQ-2.1, 01-REQ-2.2
- **Type:** Acceptance
- **Description:** Verify `rhivos/Cargo.toml` declares the correct workspace members and each member is a valid crate.
- **Preconditions:** Rust toolchain installed.
- **Input:** Parse `rhivos/Cargo.toml`.
- **Expected:** Workspace members include all five components. Each member has `Cargo.toml` and `src/main.rs`.
- **Assertion pseudocode:**
  ```
  workspace_toml = parse_toml("rhivos/Cargo.toml")
  expected_members = ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]
  for member in expected_members:
      assert member in workspace_toml.workspace.members
      assert path_exists("rhivos/" + member + "/Cargo.toml")
      assert path_exists("rhivos/" + member + "/src/main.rs")
  ```

### TS-01-8: Mock sensors declares three binary targets

- **Requirement:** 01-REQ-2.3
- **Type:** Acceptance
- **Description:** Verify `mock-sensors` crate declares three binary targets.
- **Preconditions:** Repository is checked out.
- **Input:** Parse `rhivos/mock-sensors/Cargo.toml`.
- **Expected:** Three `[[bin]]` entries: `location-sensor`, `speed-sensor`, `door-sensor`.
- **Assertion pseudocode:**
  ```
  cargo_toml = parse_toml("rhivos/mock-sensors/Cargo.toml")
  bin_names = [b.name for b in cargo_toml.bin]
  for name in ["location-sensor", "speed-sensor", "door-sensor"]:
      assert name in bin_names
  ```

### TS-01-9: Cargo build succeeds for entire workspace

- **Requirement:** 01-REQ-2.4
- **Type:** Acceptance
- **Description:** Verify `cargo build` compiles all workspace members.
- **Preconditions:** Rust toolchain installed.
- **Input:** Run `cargo build --workspace` in `rhivos/`.
- **Expected:** Exit code 0.
- **Assertion pseudocode:**
  ```
  result = exec("cargo build --workspace", cwd="rhivos/")
  assert result.exit_code == 0
  ```

### TS-01-10: Go workspace file references all modules

- **Requirement:** 01-REQ-3.1
- **Type:** Acceptance
- **Description:** Verify `go.work` references all required Go modules.
- **Preconditions:** Go toolchain installed.
- **Input:** Parse `go.work`.
- **Expected:** Contains `use` directives for all six Go module directories.
- **Assertion pseudocode:**
  ```
  go_work = read_file("go.work")
  for mod_path in ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator", "tests/setup"]:
      assert mod_path in go_work
  ```

### TS-01-11: Each Go module has go.mod and main.go

- **Requirement:** 01-REQ-3.2, 01-REQ-3.3
- **Type:** Acceptance
- **Description:** Verify each Go module has `go.mod` and `main.go`.
- **Preconditions:** Repository is checked out.
- **Input:** Filesystem inspection.
- **Expected:** Each module directory (except tests/setup) has both files.
- **Assertion pseudocode:**
  ```
  for mod in ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator"]:
      assert path_exists(mod + "/go.mod")
      assert path_exists(mod + "/main.go")
  assert path_exists("tests/setup/go.mod")
  ```

### TS-01-12: Go build succeeds for all modules

- **Requirement:** 01-REQ-3.4
- **Type:** Acceptance
- **Description:** Verify `go build ./...` succeeds with the Go workspace.
- **Preconditions:** Go toolchain installed.
- **Input:** Run `go build ./...` from repository root.
- **Expected:** Exit code 0.
- **Assertion pseudocode:**
  ```
  result = exec("go build ./...", cwd=".")
  assert result.exit_code == 0
  ```

### TS-01-13: Rust skeleton prints version and exits 0

- **Requirement:** 01-REQ-4.1, 01-REQ-4.4
- **Type:** Acceptance
- **Description:** Verify each Rust skeleton binary prints version info and exits cleanly.
- **Preconditions:** Rust workspace built.
- **Input:** Execute each Rust binary with no arguments.
- **Expected:** Stdout contains the component name. Exit code 0.
- **Assertion pseudocode:**
  ```
  for bin in ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "location-sensor", "speed-sensor", "door-sensor"]:
      result = exec("rhivos/target/debug/" + bin)
      assert result.exit_code == 0
      assert bin_component_name(bin) in result.stdout
  ```

### TS-01-14: Go skeleton prints version and exits 0

- **Requirement:** 01-REQ-4.2, 01-REQ-4.4
- **Type:** Acceptance
- **Description:** Verify each Go skeleton binary prints version info and exits cleanly.
- **Preconditions:** Go modules built.
- **Input:** Execute each Go binary with no arguments.
- **Expected:** Stdout contains the component name. Exit code 0.
- **Assertion pseudocode:**
  ```
  for mod in ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator"]:
      result = exec("go run " + mod + "/main.go")
      assert result.exit_code == 0
      assert component_name(mod) in result.stdout
  ```

### TS-01-15: Mock sensor binaries print name and version

- **Requirement:** 01-REQ-4.3
- **Type:** Acceptance
- **Description:** Verify each mock sensor binary prints its specific name.
- **Preconditions:** Rust workspace built.
- **Input:** Execute each mock sensor binary.
- **Expected:** Each binary prints its own name (e.g., "location-sensor") in stdout.
- **Assertion pseudocode:**
  ```
  for bin in ["location-sensor", "speed-sensor", "door-sensor"]:
      result = exec("rhivos/target/debug/" + bin)
      assert result.exit_code == 0
      assert bin in result.stdout
  ```

### TS-01-16: Proto files are valid proto3

- **Requirement:** 01-REQ-5.1, 01-REQ-5.2, 01-REQ-5.3
- **Type:** Acceptance
- **Description:** Verify all `.proto` files parse successfully and use proto3 syntax.
- **Preconditions:** protoc installed. Proto files exist in `proto/`.
- **Input:** Run `protoc` lint/parse on each `.proto` file.
- **Expected:** All files parse without errors. Each file contains `syntax = "proto3"`, a `package` declaration, and a `go_package` option.
- **Assertion pseudocode:**
  ```
  for proto_file in glob("proto/**/*.proto"):
      content = read_file(proto_file)
      assert 'syntax = "proto3"' in content
      assert "package " in content
      assert "go_package" in content
      result = exec("protoc --proto_path=proto " + proto_file + " --descriptor_set_out=/dev/null")
      assert result.exit_code == 0
  ```

### TS-01-17: Protoc parses all proto files without errors

- **Requirement:** 01-REQ-5.4
- **Type:** Acceptance
- **Description:** Verify protoc can parse all proto files together (cross-import resolution).
- **Preconditions:** protoc installed.
- **Input:** Run protoc on all proto files simultaneously.
- **Expected:** Exit code 0.
- **Assertion pseudocode:**
  ```
  all_protos = glob("proto/**/*.proto")
  result = exec("protoc --proto_path=proto " + " ".join(all_protos) + " --descriptor_set_out=/dev/null")
  assert result.exit_code == 0
  ```

### TS-01-18: Makefile has all required targets

- **Requirement:** 01-REQ-6.1
- **Type:** Acceptance
- **Description:** Verify the root Makefile declares all required targets.
- **Preconditions:** Repository is checked out.
- **Input:** Parse Makefile for target definitions.
- **Expected:** Targets `build`, `test`, `clean`, `proto`, `infra-up`, `infra-down`, `check` exist.
- **Assertion pseudocode:**
  ```
  makefile = read_file("Makefile")
  for target in ["build", "test", "clean", "proto", "infra-up", "infra-down", "check"]:
      assert regex_match(target + ":", makefile)
  ```

### TS-01-19: make build succeeds

- **Requirement:** 01-REQ-6.2
- **Type:** Acceptance
- **Description:** Verify `make build` compiles all components.
- **Preconditions:** Rust and Go toolchains installed.
- **Input:** Run `make build` from repository root.
- **Expected:** Exit code 0.
- **Assertion pseudocode:**
  ```
  result = exec("make build")
  assert result.exit_code == 0
  ```

### TS-01-20: make test succeeds

- **Requirement:** 01-REQ-6.3
- **Type:** Acceptance
- **Description:** Verify `make test` runs all tests successfully.
- **Preconditions:** Rust and Go toolchains installed.
- **Input:** Run `make test` from repository root.
- **Expected:** Exit code 0.
- **Assertion pseudocode:**
  ```
  result = exec("make test")
  assert result.exit_code == 0
  ```

### TS-01-21: make clean removes build artifacts

- **Requirement:** 01-REQ-6.4
- **Type:** Acceptance
- **Description:** Verify `make clean` removes Rust and Go build artifacts.
- **Preconditions:** `make build` has been run.
- **Input:** Run `make clean`.
- **Expected:** Exit code 0. Rust `target/` directory removed. Go build cache cleaned.
- **Assertion pseudocode:**
  ```
  exec("make build")
  result = exec("make clean")
  assert result.exit_code == 0
  assert not path_exists("rhivos/target")
  ```

### TS-01-22: make check runs lint and tests

- **Requirement:** 01-REQ-6.5
- **Type:** Acceptance
- **Description:** Verify `make check` runs both linting and tests.
- **Preconditions:** Rust and Go toolchains installed.
- **Input:** Run `make check`.
- **Expected:** Exit code 0.
- **Assertion pseudocode:**
  ```
  result = exec("make check")
  assert result.exit_code == 0
  ```

### TS-01-23: compose.yml defines NATS and Kuksa services

- **Requirement:** 01-REQ-7.1
- **Type:** Acceptance
- **Description:** Verify compose file defines both infrastructure services with correct ports.
- **Preconditions:** Repository is checked out.
- **Input:** Parse `deployments/compose.yml`.
- **Expected:** Services `nats` (port 4222) and Kuksa Databroker (port 55556) are defined.
- **Assertion pseudocode:**
  ```
  compose = parse_yaml("deployments/compose.yml")
  assert "nats" in compose.services
  assert "4222" in str(compose.services.nats.ports)
  kuksa_service = find_service_by_port(compose, 55556)
  assert kuksa_service is not None
  ```

### TS-01-24: NATS configuration file exists

- **Requirement:** 01-REQ-7.2
- **Type:** Acceptance
- **Description:** Verify NATS server configuration file exists.
- **Preconditions:** Repository is checked out.
- **Input:** Check filesystem.
- **Expected:** `deployments/nats/nats-server.conf` exists and is non-empty.
- **Assertion pseudocode:**
  ```
  assert path_exists("deployments/nats/nats-server.conf")
  assert file_size("deployments/nats/nats-server.conf") > 0
  ```

### TS-01-25: VSS overlay defines custom signals

- **Requirement:** 01-REQ-7.3
- **Type:** Acceptance
- **Description:** Verify VSS overlay file defines the required custom signals.
- **Preconditions:** Repository is checked out.
- **Input:** Parse VSS overlay file in `deployments/`.
- **Expected:** Contains definitions for `Vehicle.Parking.SessionActive`, `Vehicle.Command.Door.Lock`, `Vehicle.Command.Door.Response`.
- **Assertion pseudocode:**
  ```
  overlay = read_file("deployments/vss-overlay.json")
  assert "Vehicle.Parking.SessionActive" in overlay
  assert "Vehicle.Command.Door.Lock" in overlay
  assert "Vehicle.Command.Door.Response" in overlay
  ```

### TS-01-26: Rust crates have placeholder tests

- **Requirement:** 01-REQ-8.1
- **Type:** Acceptance
- **Description:** Verify each Rust crate has at least one unit test.
- **Preconditions:** Repository is checked out.
- **Input:** Search for `#[test]` annotations in Rust source files.
- **Expected:** Each crate directory contains at least one `#[test]` function.
- **Assertion pseudocode:**
  ```
  for crate in ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"]:
      source_files = glob("rhivos/" + crate + "/src/**/*.rs")
      test_found = any("#[test]" in read_file(f) for f in source_files)
      assert test_found
  ```

### TS-01-27: Go modules have placeholder tests

- **Requirement:** 01-REQ-8.2
- **Type:** Acceptance
- **Description:** Verify each Go module has at least one test function.
- **Preconditions:** Repository is checked out.
- **Input:** Search for `func Test` in Go test files.
- **Expected:** Each module directory contains at least one `_test.go` file with a `func Test` function.
- **Assertion pseudocode:**
  ```
  for mod in ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli", "mock/parking-operator"]:
      test_files = glob(mod + "/*_test.go")
      assert len(test_files) > 0
      test_found = any("func Test" in read_file(f) for f in test_files)
      assert test_found
  ```

### TS-01-28: cargo test passes for all Rust crates

- **Requirement:** 01-REQ-8.3
- **Type:** Acceptance
- **Description:** Verify `cargo test` passes in the Rust workspace.
- **Preconditions:** Rust toolchain installed.
- **Input:** Run `cargo test --workspace` in `rhivos/`.
- **Expected:** Exit code 0. All tests pass.
- **Assertion pseudocode:**
  ```
  result = exec("cargo test --workspace", cwd="rhivos/")
  assert result.exit_code == 0
  ```

### TS-01-29: go test passes for all Go modules

- **Requirement:** 01-REQ-8.4
- **Type:** Acceptance
- **Description:** Verify `go test ./...` passes for all Go modules.
- **Preconditions:** Go toolchain installed.
- **Input:** Run `go test ./...` from repository root.
- **Expected:** Exit code 0. All tests pass.
- **Assertion pseudocode:**
  ```
  result = exec("go test ./...", cwd=".")
  assert result.exit_code == 0
  ```

### TS-01-30: Setup verification tests exist and are runnable

- **Requirement:** 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.3
- **Type:** Acceptance
- **Description:** Verify setup verification tests exist and pass via `make test-setup`.
- **Preconditions:** Rust, Go, and protoc installed.
- **Input:** Run `make test-setup`.
- **Expected:** Exit code 0. Tests verify Rust build, Go build, and proto parsing.
- **Assertion pseudocode:**
  ```
  result = exec("make test-setup")
  assert result.exit_code == 0
  assert "PASS" in result.stdout
  ```

### TS-01-31: Setup tests report clear pass/fail

- **Requirement:** 01-REQ-9.4
- **Type:** Acceptance
- **Description:** Verify each setup test produces a named pass/fail result.
- **Preconditions:** Go toolchain installed.
- **Input:** Run `go test -v ./...` in `tests/setup/`.
- **Expected:** Verbose output shows individual test names and PASS/FAIL status.
- **Assertion pseudocode:**
  ```
  result = exec("go test -v ./...", cwd="tests/setup/")
  assert "TestRustBuild" in result.stdout or "TestRustCompile" in result.stdout
  assert "TestGoBuild" in result.stdout or "TestGoCompile" in result.stdout
  assert "TestProto" in result.stdout
  ```

### TS-01-32: make proto generates Go code

- **Requirement:** 01-REQ-10.1, 01-REQ-10.2, 01-REQ-10.3
- **Type:** Acceptance
- **Description:** Verify `make proto` generates compilable Go code from proto definitions.
- **Preconditions:** protoc and Go proto plugins installed.
- **Input:** Run `make proto` then `go build ./...`.
- **Expected:** Proto generation succeeds (exit code 0). Generated Go code compiles.
- **Assertion pseudocode:**
  ```
  result_proto = exec("make proto")
  assert result_proto.exit_code == 0
  result_build = exec("go build ./...")
  assert result_build.exit_code == 0
  ```

## Property Test Cases

### TS-01-P1: Build completeness across all components

- **Property:** Property 1 (Build Completeness)
- **Type:** Property
- **Description:** Every defined component compiles when the root build target is invoked.
- **Preconditions:** Rust and Go toolchains installed.
- **Input:** Run `make build`.
- **Expected:** Exit code 0. All components produce build artifacts.
- **Assertion pseudocode:**
  ```
  result = exec("make build")
  assert result.exit_code == 0
  for bin in ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "location-sensor", "speed-sensor", "door-sensor"]:
      assert path_exists("rhivos/target/debug/" + bin)
  ```

### TS-01-P2: Skeleton determinism across invocations

- **Property:** Property 2 (Skeleton Determinism)
- **Type:** Property
- **Description:** Skeleton binaries produce identical output across multiple invocations.
- **Preconditions:** Workspace built.
- **Input:** Execute each skeleton binary twice.
- **Expected:** stdout is identical across both invocations. Exit code is 0 both times.
- **Assertion pseudocode:**
  ```
  for bin in all_skeleton_binaries():
      result1 = exec(bin)
      result2 = exec(bin)
      assert result1.stdout == result2.stdout
      assert result1.exit_code == 0
      assert result2.exit_code == 0
  ```

### TS-01-P3: Infrastructure idempotency

- **Property:** Property 3 (Infrastructure Idempotency)
- **Type:** Property
- **Description:** Repeated infra-up/infra-down cycles leave the system in a consistent state.
- **Preconditions:** Podman installed.
- **Input:** Run `make infra-up`, `make infra-down`, `make infra-up`, `make infra-down`.
- **Expected:** Each command exits 0. After final `infra-down`, no infrastructure containers remain.
- **Assertion pseudocode:**
  ```
  assert exec("make infra-up").exit_code == 0
  assert exec("make infra-down").exit_code == 0
  assert exec("make infra-up").exit_code == 0
  assert exec("make infra-down").exit_code == 0
  containers = exec("podman ps --filter name=nats --filter name=kuksa -q")
  assert containers.stdout.strip() == ""
  ```

### TS-01-P4: Test isolation

- **Property:** Property 4 (Test Isolation)
- **Type:** Property
- **Description:** All tests pass without any infrastructure running and without inter-test dependencies.
- **Preconditions:** Rust and Go toolchains installed. No containers running.
- **Input:** Run `make infra-down` then `make test`.
- **Expected:** All tests pass (exit code 0) regardless of infrastructure state.
- **Assertion pseudocode:**
  ```
  exec("make infra-down")
  result = exec("make test")
  assert result.exit_code == 0
  ```

### TS-01-P5: Proto consistency across all proto files

- **Property:** Property 5 (Proto Consistency)
- **Type:** Property
- **Description:** All proto files are syntactically valid and contain required metadata fields.
- **Preconditions:** protoc installed.
- **Input:** Parse every `.proto` file.
- **Expected:** All files have proto3 syntax, package declaration, and go_package option.
- **Assertion pseudocode:**
  ```
  for proto_file in glob("proto/**/*.proto"):
      content = read_file(proto_file)
      assert 'syntax = "proto3"' in content
      assert re.search(r"^package\s+\w+", content, re.MULTILINE)
      assert re.search(r'option\s+go_package\s*=', content)
      result = exec("protoc --proto_path=proto --descriptor_set_out=/dev/null " + proto_file)
      assert result.exit_code == 0
  ```

## Edge Case Tests

### TS-01-E1: Build succeeds with extraneous files in repo

- **Requirement:** 01-REQ-1.E1
- **Type:** Edge Case
- **Description:** Build system works even when files exist outside the defined structure.
- **Preconditions:** Workspace built. A temporary file exists at repository root.
- **Input:** Create a file `stray_file.txt` at repo root, then run `make build`.
- **Expected:** Exit code 0. Stray file does not interfere with build.
- **Assertion pseudocode:**
  ```
  write_file("stray_file.txt", "test content")
  result = exec("make build")
  assert result.exit_code == 0
  delete_file("stray_file.txt")
  ```

### TS-01-E2: Cargo reports failing crate by name

- **Requirement:** 01-REQ-2.E1
- **Type:** Edge Case
- **Description:** When a Rust crate has a compile error, the error output identifies the crate.
- **Preconditions:** Workspace is in a valid state.
- **Input:** Temporarily introduce a syntax error in one crate, run `cargo build`.
- **Expected:** Error output contains the crate name. Exit code non-zero.
- **Assertion pseudocode:**
  ```
  inject_error("rhivos/locking-service/src/main.rs")
  result = exec("cargo build --workspace", cwd="rhivos/")
  assert result.exit_code != 0
  assert "locking-service" in result.stderr
  restore_file("rhivos/locking-service/src/main.rs")
  ```

### TS-01-E3: Go build fails with missing dependency

- **Requirement:** 01-REQ-3.E1
- **Type:** Edge Case
- **Description:** Go build reports clear error when a module has undeclared imports.
- **Preconditions:** Go workspace valid.
- **Input:** Add an undeclared import to a Go file, run `go build`.
- **Expected:** Error output identifies the module and missing import. Exit code non-zero.
- **Assertion pseudocode:**
  ```
  inject_import("backend/parking-fee-service/main.go", "unknown/package")
  result = exec("go build ./...")
  assert result.exit_code != 0
  assert "unknown/package" in result.stderr
  restore_file("backend/parking-fee-service/main.go")
  ```

### TS-01-E4: Skeleton exits non-zero on unknown flag

- **Requirement:** 01-REQ-4.E1
- **Type:** Edge Case
- **Description:** Skeleton binaries handle unknown flags gracefully.
- **Preconditions:** Workspace built.
- **Input:** Execute a skeleton binary with `--invalid-flag`.
- **Expected:** Stderr contains usage information. Exit code non-zero.
- **Assertion pseudocode:**
  ```
  result = exec("rhivos/target/debug/locking-service --invalid-flag")
  assert result.exit_code != 0
  assert len(result.stderr) > 0
  ```

### TS-01-E5: Protoc fails on missing import

- **Requirement:** 01-REQ-5.E1
- **Type:** Edge Case
- **Description:** Protoc reports clear error when a proto file references a missing import.
- **Preconditions:** protoc installed.
- **Input:** Create a temp proto file with a missing import, run protoc.
- **Expected:** Error identifies the missing import. Exit code non-zero.
- **Assertion pseudocode:**
  ```
  write_file("proto/temp_test.proto", 'syntax = "proto3";\nimport "nonexistent.proto";\npackage test;')
  result = exec("protoc --proto_path=proto proto/temp_test.proto --descriptor_set_out=/dev/null")
  assert result.exit_code != 0
  assert "nonexistent.proto" in result.stderr
  delete_file("proto/temp_test.proto")
  ```

### TS-01-E6: make build reports failing toolchain

- **Requirement:** 01-REQ-6.E1
- **Type:** Edge Case
- **Description:** When one toolchain fails, `make build` identifies which one.
- **Preconditions:** Workspace is valid.
- **Input:** Temporarily break a Rust crate, run `make build`.
- **Expected:** Exit code non-zero. Output indicates the Rust build failed.
- **Assertion pseudocode:**
  ```
  inject_error("rhivos/locking-service/src/main.rs")
  result = exec("make build")
  assert result.exit_code != 0
  assert "rust" in result.stderr.lower() or "cargo" in result.stderr.lower()
  restore_file("rhivos/locking-service/src/main.rs")
  ```

### TS-01-E7: Port conflict on infra-up

- **Requirement:** 01-REQ-7.E1
- **Type:** Edge Case
- **Description:** Infra-up reports port conflict when ports are in use.
- **Preconditions:** Podman installed. Port 4222 bound by another process.
- **Input:** Bind port 4222, run `make infra-up`.
- **Expected:** Podman Compose reports port conflict. Exit code non-zero.
- **Assertion pseudocode:**
  ```
  blocker = bind_port(4222)
  result = exec("make infra-up")
  assert result.exit_code != 0
  release_port(blocker)
  ```

### TS-01-E8: infra-down with no running containers

- **Requirement:** 01-REQ-7.E2
- **Type:** Edge Case
- **Description:** infra-down succeeds even when no containers are running.
- **Preconditions:** Podman installed. No infrastructure containers running.
- **Input:** Run `make infra-down`.
- **Expected:** Exit code 0.
- **Assertion pseudocode:**
  ```
  result = exec("make infra-down")
  assert result.exit_code == 0
  ```

### TS-01-E9: Test runner reports syntax errors

- **Requirement:** 01-REQ-8.E1
- **Type:** Edge Case
- **Description:** Test runner identifies file and line for syntax errors.
- **Preconditions:** Workspace valid.
- **Input:** Introduce syntax error in a test file, run test command.
- **Expected:** Error output identifies file and line. Exit code non-zero.
- **Assertion pseudocode:**
  ```
  inject_syntax_error("rhivos/locking-service/src/main.rs")
  result = exec("cargo test --workspace", cwd="rhivos/")
  assert result.exit_code != 0
  assert "main.rs" in result.stderr
  restore_file("rhivos/locking-service/src/main.rs")
  ```

### TS-01-E10: Setup test skips on missing toolchain

- **Requirement:** 01-REQ-9.E1
- **Type:** Edge Case
- **Description:** Setup verification tests skip gracefully when a toolchain is missing.
- **Preconditions:** Go installed. Path modified to hide a toolchain.
- **Input:** Run setup tests with `cargo` not on PATH.
- **Expected:** Rust-related tests skip with a message. Exit code 0 (skips are not failures).
- **Assertion pseudocode:**
  ```
  result = exec("PATH=/usr/bin go test -v ./...", cwd="tests/setup/")
  assert "SKIP" in result.stdout or "skip" in result.stdout
  ```

### TS-01-E11: make proto fails when protoc missing

- **Requirement:** 01-REQ-10.E1
- **Type:** Edge Case
- **Description:** Proto target reports error when protoc is not installed.
- **Preconditions:** protoc not on PATH.
- **Input:** Run `make proto` with protoc absent from PATH.
- **Expected:** Error message mentions protoc. Exit code non-zero.
- **Assertion pseudocode:**
  ```
  result = exec("PATH=/usr/bin make proto")
  assert result.exit_code != 0
  assert "protoc" in result.stderr or "protoc" in result.stdout
  ```

## Integration Smoke Tests

### TS-01-SMOKE-1: Full build-test cycle

- **Type:** Integration Smoke
- **Description:** Verify the complete build and test cycle works end-to-end from a clean state.
- **What must NOT be mocked:** Rust toolchain (cargo), Go toolchain (go), filesystem operations, Make.
- **Preconditions:** Clean repository checkout. Rust, Go, and Make installed.
- **Input:** Run `make clean && make build && make test`.
- **Expected:** All three commands succeed with exit code 0.
- **Assertion pseudocode:**
  ```
  assert exec("make clean").exit_code == 0
  assert exec("make build").exit_code == 0
  assert exec("make test").exit_code == 0
  ```

### TS-01-SMOKE-2: Infrastructure lifecycle

- **Type:** Integration Smoke
- **Description:** Verify NATS and Kuksa Databroker containers start, are reachable, and stop cleanly.
- **What must NOT be mocked:** Podman, Podman Compose, NATS container image, Kuksa container image, network ports.
- **Preconditions:** Podman installed. Ports 4222 and 55556 available.
- **Input:** Run `make infra-up`, verify ports, run `make infra-down`.
- **Expected:** After infra-up, ports 4222 and 55556 accept connections. After infra-down, containers are removed.
- **Assertion pseudocode:**
  ```
  assert exec("make infra-up").exit_code == 0
  assert tcp_connect("localhost", 4222).success
  assert tcp_connect("localhost", 55556).success
  assert exec("make infra-down").exit_code == 0
  containers = exec("podman ps -q --filter name=nats --filter name=kuksa")
  assert containers.stdout.strip() == ""
  ```

### TS-01-SMOKE-3: Proto generation and build integration

- **Type:** Integration Smoke
- **Description:** Verify proto generation produces code that integrates with the Go build.
- **What must NOT be mocked:** protoc, protoc-gen-go, protoc-gen-go-grpc, Go toolchain.
- **Preconditions:** protoc and Go proto plugins installed.
- **Input:** Run `make proto && go build ./...`.
- **Expected:** Proto generation succeeds. Generated code compiles with Go build.
- **Assertion pseudocode:**
  ```
  assert exec("make proto").exit_code == 0
  assert exec("go build ./...").exit_code == 0
  ```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|------------|----------------|------|
| 01-REQ-1.1 | TS-01-1 | Acceptance |
| 01-REQ-1.2 | TS-01-2 | Acceptance |
| 01-REQ-1.3, 01-REQ-1.4 | TS-01-3 | Acceptance |
| 01-REQ-1.5 | TS-01-4 | Acceptance |
| 01-REQ-1.6, 01-REQ-1.7 | TS-01-5 | Acceptance |
| 01-REQ-1.8 | TS-01-6 | Acceptance |
| 01-REQ-1.E1 | TS-01-E1 | Edge Case |
| 01-REQ-2.1, 01-REQ-2.2 | TS-01-7 | Acceptance |
| 01-REQ-2.3 | TS-01-8 | Acceptance |
| 01-REQ-2.4 | TS-01-9 | Acceptance |
| 01-REQ-2.E1 | TS-01-E2 | Edge Case |
| 01-REQ-3.1 | TS-01-10 | Acceptance |
| 01-REQ-3.2, 01-REQ-3.3 | TS-01-11 | Acceptance |
| 01-REQ-3.4 | TS-01-12 | Acceptance |
| 01-REQ-3.E1 | TS-01-E3 | Edge Case |
| 01-REQ-4.1, 01-REQ-4.4 | TS-01-13 | Acceptance |
| 01-REQ-4.2, 01-REQ-4.4 | TS-01-14 | Acceptance |
| 01-REQ-4.3 | TS-01-15 | Acceptance |
| 01-REQ-4.E1 | TS-01-E4 | Edge Case |
| 01-REQ-5.1, 01-REQ-5.2, 01-REQ-5.3 | TS-01-16 | Acceptance |
| 01-REQ-5.4 | TS-01-17 | Acceptance |
| 01-REQ-5.E1 | TS-01-E5 | Edge Case |
| 01-REQ-6.1 | TS-01-18 | Acceptance |
| 01-REQ-6.2 | TS-01-19 | Acceptance |
| 01-REQ-6.3 | TS-01-20 | Acceptance |
| 01-REQ-6.4 | TS-01-21 | Acceptance |
| 01-REQ-6.5 | TS-01-22 | Acceptance |
| 01-REQ-6.E1 | TS-01-E6 | Edge Case |
| 01-REQ-7.1 | TS-01-23 | Acceptance |
| 01-REQ-7.2 | TS-01-24 | Acceptance |
| 01-REQ-7.3 | TS-01-25 | Acceptance |
| 01-REQ-7.4 | TS-01-SMOKE-2 | Integration Smoke |
| 01-REQ-7.5 | TS-01-SMOKE-2 | Integration Smoke |
| 01-REQ-7.E1 | TS-01-E7 | Edge Case |
| 01-REQ-7.E2 | TS-01-E8 | Edge Case |
| 01-REQ-8.1 | TS-01-26 | Acceptance |
| 01-REQ-8.2 | TS-01-27 | Acceptance |
| 01-REQ-8.3 | TS-01-28 | Acceptance |
| 01-REQ-8.4 | TS-01-29 | Acceptance |
| 01-REQ-8.E1 | TS-01-E9 | Edge Case |
| 01-REQ-9.1, 01-REQ-9.2, 01-REQ-9.3 | TS-01-30 | Acceptance |
| 01-REQ-9.4 | TS-01-31 | Acceptance |
| 01-REQ-9.E1 | TS-01-E10 | Edge Case |
| 01-REQ-10.1, 01-REQ-10.2, 01-REQ-10.3 | TS-01-32 | Acceptance |
| 01-REQ-10.E1 | TS-01-E11 | Edge Case |
| Property 1 | TS-01-P1 | Property |
| Property 2 | TS-01-P2 | Property |
| Property 3 | TS-01-P3 | Property |
| Property 4 | TS-01-P4 | Property |
| Property 5 | TS-01-P5 | Property |
