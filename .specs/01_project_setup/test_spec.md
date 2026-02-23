# Test Specification: Project Setup (Phase 1.2)

## Overview

This test specification translates every acceptance criterion and correctness
property from the requirements and design documents into concrete, executable
test contracts. Tests are organized into three categories:

- **Acceptance criterion tests (TS-01-N):** One per acceptance criterion.
  Mostly structural or build-verification tests implemented as Go test
  functions in a dedicated `tests/setup/` package or as Makefile-driven
  script checks.
- **Property tests (TS-01-PN):** One per correctness property. Verify
  invariants that must hold across the repository.
- **Edge case tests (TS-01-EN):** One per edge case requirement. Verify
  error handling and boundary behavior.

Since this spec covers project scaffolding rather than business logic, most
tests are structural assertions (directory exists, command succeeds, output
matches). These are implemented as Go tests using `os.Stat`, `os/exec`, and
standard assertions.

## Test Cases

### TS-01-1: Rust component directory exists

**Requirement:** 01-REQ-1.1
**Type:** unit
**Description:** Verify the top-level directory for Rust components exists.

**Preconditions:**
- Fresh checkout of the repository.

**Input:**
- Path: `rhivos/`

**Expected:**
- Directory exists and contains `Cargo.toml`.

**Assertion pseudocode:**
```
ASSERT directory_exists("rhivos/")
ASSERT file_exists("rhivos/Cargo.toml")
```

---

### TS-01-2: Go component directory exists

**Requirement:** 01-REQ-1.2
**Type:** unit
**Description:** Verify the top-level directory for Go backend components exists.

**Preconditions:**
- Fresh checkout of the repository.

**Input:**
- Path: `backend/`

**Expected:**
- Directory exists and contains subdirectories for each Go service.

**Assertion pseudocode:**
```
ASSERT directory_exists("backend/")
ASSERT directory_exists("backend/parking-fee-service/")
ASSERT directory_exists("backend/cloud-gateway/")
```

---

### TS-01-3: Proto directory exists

**Requirement:** 01-REQ-1.3
**Type:** unit
**Description:** Verify the shared protocol buffer directory exists.

**Preconditions:**
- Fresh checkout of the repository.

**Input:**
- Path: `proto/`

**Expected:**
- Directory exists and contains `.proto` files.

**Assertion pseudocode:**
```
ASSERT directory_exists("proto/")
files = glob("proto/*.proto")
ASSERT len(files) >= 3
```

---

### TS-01-4: Mock directory exists

**Requirement:** 01-REQ-1.4
**Type:** unit
**Description:** Verify the top-level directory for mock CLI applications exists.

**Preconditions:**
- Fresh checkout of the repository.

**Input:**
- Path: `mock/`

**Expected:**
- Directory exists with subdirectories for each mock app.

**Assertion pseudocode:**
```
ASSERT directory_exists("mock/")
ASSERT directory_exists("mock/parking-app-cli/")
ASSERT directory_exists("mock/companion-app-cli/")
```

---

### TS-01-5: Android placeholder directories exist

**Requirement:** 01-REQ-1.5
**Type:** unit
**Description:** Verify placeholder directories for AAOS and Android apps exist.

**Preconditions:**
- Fresh checkout of the repository.

**Input:**
- Paths: `aaos/parking-app/`, `android/companion-app/`

**Expected:**
- Both directories exist with at least a README.md.

**Assertion pseudocode:**
```
ASSERT directory_exists("aaos/parking-app/")
ASSERT file_exists("aaos/parking-app/README.md")
ASSERT directory_exists("android/companion-app/")
ASSERT file_exists("android/companion-app/README.md")
```

---

### TS-01-6: Infrastructure directory exists

**Requirement:** 01-REQ-1.6
**Type:** unit
**Description:** Verify the local infrastructure configuration directory exists.

**Preconditions:**
- Fresh checkout of the repository.

**Input:**
- Path: `infra/`

**Expected:**
- Directory exists with docker-compose.yml and mosquitto config.

**Assertion pseudocode:**
```
ASSERT directory_exists("infra/")
ASSERT file_exists("infra/docker-compose.yml")
ASSERT file_exists("infra/mosquitto/mosquitto.conf")
```

---

### TS-01-7: UPDATE_SERVICE proto definition

**Requirement:** 01-REQ-2.1
**Type:** unit
**Description:** Verify the UPDATE_SERVICE proto file defines all required RPCs.

**Preconditions:**
- Proto directory exists.

**Input:**
- File: `proto/update_service.proto`

**Expected:**
- File contains service definition with RPCs: InstallAdapter,
  WatchAdapterStates, ListAdapters, RemoveAdapter, GetAdapterStatus.

**Assertion pseudocode:**
```
content = read_file("proto/update_service.proto")
ASSERT contains(content, "service UpdateService")
ASSERT contains(content, "rpc InstallAdapter")
ASSERT contains(content, "rpc WatchAdapterStates")
ASSERT contains(content, "rpc ListAdapters")
ASSERT contains(content, "rpc RemoveAdapter")
ASSERT contains(content, "rpc GetAdapterStatus")
```

---

### TS-01-8: PARKING_OPERATOR_ADAPTOR proto definition

**Requirement:** 01-REQ-2.2
**Type:** unit
**Description:** Verify the PARKING_OPERATOR_ADAPTOR proto file defines all
required RPCs.

**Preconditions:**
- Proto directory exists.

**Input:**
- File: `proto/parking_adaptor.proto`

**Expected:**
- File contains service definition with RPCs: StartSession, StopSession,
  GetStatus, GetRate.

**Assertion pseudocode:**
```
content = read_file("proto/parking_adaptor.proto")
ASSERT contains(content, "service ParkingAdaptor")
ASSERT contains(content, "rpc StartSession")
ASSERT contains(content, "rpc StopSession")
ASSERT contains(content, "rpc GetStatus")
ASSERT contains(content, "rpc GetRate")
```

---

### TS-01-9: Common proto types

**Requirement:** 01-REQ-2.3
**Type:** unit
**Description:** Verify the common proto file defines shared types.

**Preconditions:**
- Proto directory exists.

**Input:**
- File: `proto/common.proto`

**Expected:**
- File defines AdapterState enum and ErrorDetails message.

**Assertion pseudocode:**
```
content = read_file("proto/common.proto")
ASSERT contains(content, "enum AdapterState")
ASSERT contains(content, "message ErrorDetails")
ASSERT contains(content, "message AdapterInfo")
```

---

### TS-01-10: Proto files compile with protoc

**Requirement:** 01-REQ-2.4
**Type:** unit
**Description:** Verify all proto files compile without errors.

**Preconditions:**
- `protoc` is installed.

**Input:**
- All `.proto` files in `proto/`.

**Expected:**
- `protoc` exits with code 0 for each file.

**Assertion pseudocode:**
```
result = exec("protoc --proto_path=proto/ --descriptor_set_out=/dev/null proto/*.proto")
ASSERT result.exit_code == 0
```

---

### TS-01-11: Proto files use proto3 syntax

**Requirement:** 01-REQ-2.5
**Type:** unit
**Description:** Verify all proto files declare proto3 syntax.

**Preconditions:**
- Proto directory exists.

**Input:**
- All `.proto` files in `proto/`.

**Expected:**
- Each file's first non-comment, non-empty line is `syntax = "proto3";`.

**Assertion pseudocode:**
```
FOR EACH file IN glob("proto/*.proto"):
    content = read_file(file)
    ASSERT contains(content, 'syntax = "proto3"')
```

---

### TS-01-12: Rust Cargo workspace members

**Requirement:** 01-REQ-3.1
**Type:** unit
**Description:** Verify the Cargo workspace includes all required members.

**Preconditions:**
- `rhivos/Cargo.toml` exists.

**Input:**
- File: `rhivos/Cargo.toml`

**Expected:**
- Workspace members include locking-service, cloud-gateway-client,
  update-service, parking-operator-adaptor.

**Assertion pseudocode:**
```
content = read_file("rhivos/Cargo.toml")
ASSERT contains(content, "locking-service")
ASSERT contains(content, "cloud-gateway-client")
ASSERT contains(content, "update-service")
ASSERT contains(content, "parking-operator-adaptor")
```

---

### TS-01-13: Rust workspace builds successfully

**Requirement:** 01-REQ-3.2
**Type:** integration
**Description:** Verify `cargo build` succeeds for the entire workspace.

**Preconditions:**
- Rust toolchain installed. Proto files present.

**Input:**
- Command: `cargo build` in `rhivos/`.

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
result = exec("cargo build", cwd="rhivos/")
ASSERT result.exit_code == 0
```

---

### TS-01-14: Rust workspace tests pass

**Requirement:** 01-REQ-3.3
**Type:** integration
**Description:** Verify `cargo test` succeeds for the entire workspace.

**Preconditions:**
- Rust toolchain installed. Workspace builds.

**Input:**
- Command: `cargo test` in `rhivos/`.

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
result = exec("cargo test", cwd="rhivos/")
ASSERT result.exit_code == 0
```

---

### TS-01-15: Rust skeletons include generated proto code

**Requirement:** 01-REQ-3.4
**Type:** unit
**Description:** Verify each Rust service crate has a build.rs that references
proto files.

**Preconditions:**
- Rust workspace exists.

**Input:**
- `build.rs` files in each crate.

**Expected:**
- Each gRPC crate's build.rs references a proto file.

**Assertion pseudocode:**
```
FOR EACH crate IN ["update-service", "parking-operator-adaptor"]:
    content = read_file("rhivos/" + crate + "/build.rs")
    ASSERT contains(content, ".proto")
    ASSERT contains(content, "tonic_build")
```

---

### TS-01-16: Rust stubs return unimplemented

**Requirement:** 01-REQ-3.5
**Type:** unit
**Description:** Verify Rust gRPC service stubs return unimplemented status.

**Preconditions:**
- Rust workspace exists.

**Input:**
- Source files in gRPC service crates.

**Expected:**
- Stub implementations contain `Status::unimplemented` or equivalent.

**Assertion pseudocode:**
```
FOR EACH crate IN ["update-service", "parking-operator-adaptor"]:
    content = read_file("rhivos/" + crate + "/src/lib.rs")
    ASSERT contains(content, "unimplemented")
```

---

### TS-01-17: Go modules for backend services

**Requirement:** 01-REQ-4.1
**Type:** unit
**Description:** Verify Go modules exist for PARKING_FEE_SERVICE and
CLOUD_GATEWAY.

**Preconditions:**
- Backend directory exists.

**Input:**
- Paths: `backend/parking-fee-service/go.mod`,
  `backend/cloud-gateway/go.mod`.

**Expected:**
- Both go.mod files exist and declare valid module paths.

**Assertion pseudocode:**
```
FOR EACH svc IN ["parking-fee-service", "cloud-gateway"]:
    content = read_file("backend/" + svc + "/go.mod")
    ASSERT contains(content, "module github.com/rhadp/parking-fee-service/backend/" + svc)
```

---

### TS-01-18: Go backend builds successfully

**Requirement:** 01-REQ-4.2
**Type:** integration
**Description:** Verify `go build` succeeds for each backend module.

**Preconditions:**
- Go toolchain installed. Proto code generated.

**Input:**
- Command: `go build ./...` in each backend module.

**Expected:**
- Exit code 0 for each.

**Assertion pseudocode:**
```
FOR EACH svc IN ["parking-fee-service", "cloud-gateway"]:
    result = exec("go build ./...", cwd="backend/" + svc)
    ASSERT result.exit_code == 0
```

---

### TS-01-19: Go backend tests pass

**Requirement:** 01-REQ-4.3
**Type:** integration
**Description:** Verify `go test` succeeds for each backend module.

**Preconditions:**
- Go toolchain installed. Modules build.

**Input:**
- Command: `go test ./...` in each backend module.

**Expected:**
- Exit code 0 for each.

**Assertion pseudocode:**
```
FOR EACH svc IN ["parking-fee-service", "cloud-gateway"]:
    result = exec("go test ./...", cwd="backend/" + svc)
    ASSERT result.exit_code == 0
```

---

### TS-01-20: Go skeletons include generated proto code

**Requirement:** 01-REQ-4.4
**Type:** unit
**Description:** Verify generated Go proto code exists and is importable.

**Preconditions:**
- `make proto` has been run.

**Input:**
- Paths: `gen/go/updateservicepb/`, `gen/go/parkingadaptorpb/`,
  `gen/go/commonpb/`.

**Expected:**
- Each directory contains `.pb.go` files.

**Assertion pseudocode:**
```
FOR EACH pkg IN ["commonpb", "updateservicepb", "parkingadaptorpb"]:
    files = glob("gen/go/" + pkg + "/*.pb.go")
    ASSERT len(files) >= 1
```

---

### TS-01-21: PARKING_FEE_SERVICE health endpoint

**Requirement:** 01-REQ-4.5
**Type:** integration
**Description:** Verify the PARKING_FEE_SERVICE stub responds to health checks.

**Preconditions:**
- Binary built. No other service on port 8080.

**Input:**
- Start `parking-fee-service` binary, send `GET /health`.

**Expected:**
- HTTP 200 with body containing `{"status": "ok"}`.

**Assertion pseudocode:**
```
process = start("backend/parking-fee-service/parking-fee-service")
wait_for_port(8080)
response = http_get("http://localhost:8080/health")
ASSERT response.status_code == 200
ASSERT response.body contains '"status"' AND '"ok"'
stop(process)
```

---

### TS-01-22: CLOUD_GATEWAY stub entry points

**Requirement:** 01-REQ-4.6
**Type:** integration
**Description:** Verify the CLOUD_GATEWAY stub starts HTTP and logs MQTT status.

**Preconditions:**
- Binary built. No other service on port 8081.

**Input:**
- Start `cloud-gateway` binary.

**Expected:**
- HTTP server reachable on port 8081. Startup output mentions MQTT.

**Assertion pseudocode:**
```
process = start("backend/cloud-gateway/cloud-gateway")
wait_for_port(8081)
response = http_get("http://localhost:8081/health")
ASSERT response.status_code == 200
ASSERT process.stdout contains "MQTT"
stop(process)
```

---

### TS-01-23: Mock PARKING_APP CLI exists

**Requirement:** 01-REQ-5.1
**Type:** unit
**Description:** Verify the mock PARKING_APP CLI application exists as a Go module.

**Preconditions:**
- Mock directory exists.

**Input:**
- Path: `mock/parking-app-cli/go.mod`.

**Expected:**
- go.mod exists and declares valid module path.

**Assertion pseudocode:**
```
content = read_file("mock/parking-app-cli/go.mod")
ASSERT contains(content, "module github.com/rhadp/parking-fee-service/mock/parking-app-cli")
```

---

### TS-01-24: Mock COMPANION_APP CLI exists

**Requirement:** 01-REQ-5.2
**Type:** unit
**Description:** Verify the mock COMPANION_APP CLI application exists as a Go module.

**Preconditions:**
- Mock directory exists.

**Input:**
- Path: `mock/companion-app-cli/go.mod`.

**Expected:**
- go.mod exists and declares valid module path.

**Assertion pseudocode:**
```
content = read_file("mock/companion-app-cli/go.mod")
ASSERT contains(content, "module github.com/rhadp/parking-fee-service/mock/companion-app-cli")
```

---

### TS-01-25: Mock CLI apps produce single binary

**Requirement:** 01-REQ-5.3
**Type:** integration
**Description:** Verify each mock CLI app builds to a single executable.

**Preconditions:**
- Go toolchain installed.

**Input:**
- Build each mock app.

**Expected:**
- Each produces exactly one executable binary.

**Assertion pseudocode:**
```
FOR EACH app IN ["parking-app-cli", "companion-app-cli"]:
    result = exec("go build -o " + app, cwd="mock/" + app)
    ASSERT result.exit_code == 0
    ASSERT file_exists("mock/" + app + "/" + app)
    ASSERT is_executable("mock/" + app + "/" + app)
```

---

### TS-01-26: Mock CLI apps show help

**Requirement:** 01-REQ-5.4
**Type:** integration
**Description:** Verify each mock CLI app displays help when run without
arguments.

**Preconditions:**
- Mock CLI apps built.

**Input:**
- Run each binary with no arguments.

**Expected:**
- Exit code 0. Non-empty stdout listing available commands.

**Assertion pseudocode:**
```
FOR EACH app IN ["parking-app-cli", "companion-app-cli"]:
    result = exec("mock/" + app + "/" + app)
    ASSERT result.exit_code == 0
    ASSERT len(result.stdout) > 0
    ASSERT contains(result.stdout, "Available Commands") OR contains(result.stdout, "Usage")
```

---

### TS-01-27: Mock CLI apps share proto definitions

**Requirement:** 01-REQ-5.5
**Type:** unit
**Description:** Verify mock CLI apps import generated proto packages.

**Preconditions:**
- Mock CLI source exists.

**Input:**
- go.mod of each mock app.

**Expected:**
- Each go.mod (or go source) references the gen/go proto module.

**Assertion pseudocode:**
```
FOR EACH app IN ["parking-app-cli", "companion-app-cli"]:
    source_files = glob("mock/" + app + "/**/*.go")
    all_content = concat(read_file(f) for f in source_files)
    ASSERT contains(all_content, "github.com/rhadp/parking-fee-service/gen/go/")
```

---

### TS-01-28: Top-level Makefile exists

**Requirement:** 01-REQ-6.1
**Type:** unit
**Description:** Verify a top-level Makefile exists at repository root.

**Preconditions:**
- Fresh checkout.

**Input:**
- Path: `Makefile`.

**Expected:**
- File exists.

**Assertion pseudocode:**
```
ASSERT file_exists("Makefile")
```

---

### TS-01-29: make build succeeds

**Requirement:** 01-REQ-6.2
**Type:** integration
**Description:** Verify `make build` compiles all components.

**Preconditions:**
- Rust and Go toolchains installed. Proto code generated.

**Input:**
- Command: `make build`.

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
result = exec("make build")
ASSERT result.exit_code == 0
```

---

### TS-01-30: make test succeeds

**Requirement:** 01-REQ-6.3
**Type:** integration
**Description:** Verify `make test` runs all tests.

**Preconditions:**
- Components build successfully.

**Input:**
- Command: `make test`.

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
result = exec("make test")
ASSERT result.exit_code == 0
```

---

### TS-01-31: make lint succeeds

**Requirement:** 01-REQ-6.4
**Type:** integration
**Description:** Verify `make lint` runs linters without errors.

**Preconditions:**
- Components build successfully.

**Input:**
- Command: `make lint`.

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
result = exec("make lint")
ASSERT result.exit_code == 0
```

---

### TS-01-32: make proto generates Go code

**Requirement:** 01-REQ-6.5
**Type:** integration
**Description:** Verify `make proto` regenerates Go code from proto files.

**Preconditions:**
- `protoc`, `protoc-gen-go`, `protoc-gen-go-grpc` installed.

**Input:**
- Delete `gen/go/` contents, then run `make proto`.

**Expected:**
- Exit code 0. Generated `.pb.go` files restored.

**Assertion pseudocode:**
```
delete_contents("gen/go/commonpb/")
delete_contents("gen/go/updateservicepb/")
delete_contents("gen/go/parkingadaptorpb/")
result = exec("make proto")
ASSERT result.exit_code == 0
ASSERT len(glob("gen/go/commonpb/*.pb.go")) >= 1
ASSERT len(glob("gen/go/updateservicepb/*.pb.go")) >= 1
ASSERT len(glob("gen/go/parkingadaptorpb/*.pb.go")) >= 1
```

---

### TS-01-33: make clean removes artifacts

**Requirement:** 01-REQ-6.6
**Type:** integration
**Description:** Verify `make clean` removes build artifacts.

**Preconditions:**
- `make build` has been run (artifacts exist).

**Input:**
- Command: `make clean`.

**Expected:**
- Exit code 0. Rust `target/` directory removed. Go binaries removed.

**Assertion pseudocode:**
```
exec("make build")
result = exec("make clean")
ASSERT result.exit_code == 0
ASSERT NOT directory_exists("rhivos/target/")
FOR EACH app IN ["parking-app-cli", "companion-app-cli"]:
    ASSERT NOT file_exists("mock/" + app + "/" + app)
```

---

### TS-01-34: Infrastructure composition file exists

**Requirement:** 01-REQ-7.1
**Type:** unit
**Description:** Verify a Podman-compatible docker-compose.yml exists.

**Preconditions:**
- Infra directory exists.

**Input:**
- Path: `infra/docker-compose.yml`.

**Expected:**
- File exists and defines `services`.

**Assertion pseudocode:**
```
content = read_file("infra/docker-compose.yml")
ASSERT contains(content, "services:")
```

---

### TS-01-35: Infrastructure includes Mosquitto

**Requirement:** 01-REQ-7.2
**Type:** unit
**Description:** Verify docker-compose defines a Mosquitto service.

**Preconditions:**
- `infra/docker-compose.yml` exists.

**Input:**
- File: `infra/docker-compose.yml`.

**Expected:**
- Contains a service using the `eclipse-mosquitto` image.

**Assertion pseudocode:**
```
content = read_file("infra/docker-compose.yml")
ASSERT contains(content, "eclipse-mosquitto")
ASSERT contains(content, "1883")
```

---

### TS-01-36: Infrastructure includes Kuksa Databroker

**Requirement:** 01-REQ-7.3
**Type:** unit
**Description:** Verify docker-compose defines a Kuksa Databroker service.

**Preconditions:**
- `infra/docker-compose.yml` exists.

**Input:**
- File: `infra/docker-compose.yml`.

**Expected:**
- Contains a service using the Kuksa Databroker image.

**Assertion pseudocode:**
```
content = read_file("infra/docker-compose.yml")
ASSERT contains(content, "kuksa-databroker")
ASSERT contains(content, "55556")
```

---

### TS-01-37: make infra-up starts services

**Requirement:** 01-REQ-7.4
**Type:** integration
**Description:** Verify `make infra-up` starts infrastructure and services
become reachable.

**Preconditions:**
- Container runtime (Podman/Docker) installed. Ports 1883 and 55556 free.

**Input:**
- Command: `make infra-up`.

**Expected:**
- Exit code 0. MQTT port 1883 reachable. Kuksa gRPC port 55556 reachable.

**Assertion pseudocode:**
```
result = exec("make infra-up")
ASSERT result.exit_code == 0
wait_for_port(1883, timeout=30s)
wait_for_port(55556, timeout=30s)
ASSERT port_is_open(1883)
ASSERT port_is_open(55556)
exec("make infra-down")
```

---

### TS-01-38: make infra-down stops services

**Requirement:** 01-REQ-7.5
**Type:** integration
**Description:** Verify `make infra-down` stops infrastructure and releases
ports.

**Preconditions:**
- Infrastructure is running via `make infra-up`.

**Input:**
- Command: `make infra-down`.

**Expected:**
- Exit code 0. Ports 1883 and 55556 no longer in use.

**Assertion pseudocode:**
```
exec("make infra-up")
wait_for_port(1883, timeout=30s)
result = exec("make infra-down")
ASSERT result.exit_code == 0
sleep(2s)
ASSERT NOT port_is_open(1883)
ASSERT NOT port_is_open(55556)
```

---

### TS-01-39: Rust placeholder tests

**Requirement:** 01-REQ-8.1
**Type:** unit
**Description:** Verify each Rust crate has at least one passing test.

**Preconditions:**
- Rust workspace builds.

**Input:**
- Run `cargo test` and parse output.

**Expected:**
- Each crate reports at least one test run (not zero tests).

**Assertion pseudocode:**
```
result = exec("cargo test", cwd="rhivos/")
ASSERT result.exit_code == 0
FOR EACH crate IN ["locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor"]:
    ASSERT result.stdout contains crate AND "test result: ok"
```

---

### TS-01-40: Go placeholder tests

**Requirement:** 01-REQ-8.2
**Type:** unit
**Description:** Verify each Go module has at least one passing test.

**Preconditions:**
- Go modules build.

**Input:**
- Run `go test ./...` in each module.

**Expected:**
- Each module reports passing tests (exit code 0, output indicates test ran).

**Assertion pseudocode:**
```
FOR EACH mod IN ["backend/parking-fee-service", "backend/cloud-gateway", "mock/parking-app-cli", "mock/companion-app-cli"]:
    result = exec("go test -v ./...", cwd=mod)
    ASSERT result.exit_code == 0
    ASSERT contains(result.stdout, "PASS")
```

---

### TS-01-41: Unit tests pass without infrastructure

**Requirement:** 01-REQ-8.3
**Type:** integration
**Description:** Verify all unit tests pass with infrastructure services stopped.

**Preconditions:**
- No Mosquitto or Kuksa running.

**Input:**
- Command: `make infra-down && make test`.

**Expected:**
- `make test` exits with code 0.

**Assertion pseudocode:**
```
exec("make infra-down")
result = exec("make test")
ASSERT result.exit_code == 0
```

---

### TS-01-42: Integration test directory structure

**Requirement:** 01-REQ-8.4
**Type:** unit
**Description:** Verify a directory for integration tests exists and is
separate from unit tests.

**Preconditions:**
- Fresh checkout.

**Input:**
- Path: `tests/integration/`.

**Expected:**
- Directory exists.

**Assertion pseudocode:**
```
ASSERT directory_exists("tests/")
ASSERT directory_exists("tests/integration/")
```

---

## Property Test Cases

### TS-01-P1: Build Completeness

**Property:** Property 1 from design.md
**Validates:** 01-REQ-3.2, 01-REQ-4.2
**Type:** property
**Description:** For any component in the repository, building it produces
zero errors.

**For any:** Component C in {all Rust crates, all Go modules}
**Invariant:** `build(C)` exits with code 0.

**Assertion pseudocode:**
```
FOR ANY component IN all_components():
    result = build(component)
    ASSERT result.exit_code == 0
```

---

### TS-01-P2: Proto-to-Code Consistency

**Property:** Property 2 from design.md
**Validates:** 01-REQ-2.4, 01-REQ-3.4, 01-REQ-4.4
**Type:** property
**Description:** For any proto file, generated Rust and Go code both compile,
and define the same set of services.

**For any:** Proto file P in `proto/*.proto` that defines a `service`
**Invariant:** The set of RPC method names in the generated Rust code equals
the set of RPC method names in the generated Go code.

**Assertion pseudocode:**
```
FOR ANY proto_file IN glob("proto/*.proto"):
    IF defines_service(proto_file):
        rust_methods = extract_rpc_methods_from_rust(proto_file)
        go_methods = extract_rpc_methods_from_go(proto_file)
        ASSERT rust_methods == go_methods
        ASSERT len(rust_methods) > 0
```

---

### TS-01-P3: Test Isolation

**Property:** Property 3 from design.md
**Validates:** 01-REQ-8.3
**Type:** property
**Description:** For any unit test, running without infrastructure produces
no failures.

**For any:** Unit test T in the repository
**Invariant:** `run_test(T)` passes regardless of whether Mosquitto or Kuksa
is running.

**Assertion pseudocode:**
```
exec("make infra-down")
FOR ANY test IN all_unit_tests():
    result = run_test(test)
    ASSERT result == PASS
```

---

### TS-01-P4: Mock CLI Usability

**Property:** Property 4 from design.md
**Validates:** 01-REQ-5.3, 01-REQ-5.4
**Type:** property
**Description:** For any mock CLI app, invoking with --help exits 0 with
non-empty output.

**For any:** Mock CLI binary B in {parking-app-cli, companion-app-cli}
**Invariant:** `exec(B, "--help")` exits with code 0 and stdout length > 0.

**Assertion pseudocode:**
```
FOR ANY binary IN mock_cli_binaries():
    result = exec(binary, "--help")
    ASSERT result.exit_code == 0
    ASSERT len(result.stdout) > 0
```

---

### TS-01-P5: Infrastructure Lifecycle Idempotency

**Property:** Property 5 from design.md
**Validates:** 01-REQ-7.4, 01-REQ-7.5
**Type:** property
**Description:** For any sequence of infra-up/down operations, the system
reaches a consistent state.

**For any:** Sequence S of `infra-up` and `infra-down` operations (length 1-4)
**Invariant:** After the sequence completes, if the last operation was
`infra-up` then all ports are reachable; if `infra-down` then all ports
are free.

**Assertion pseudocode:**
```
FOR ANY sequence IN [["up"], ["up", "up"], ["down", "up"], ["up", "down", "up"]]:
    FOR op IN sequence:
        exec("make infra-" + op)
        sleep(3s)
    last = sequence[-1]
    IF last == "up":
        ASSERT port_is_open(1883)
        ASSERT port_is_open(55556)
    ELSE:
        ASSERT NOT port_is_open(1883)
        ASSERT NOT port_is_open(55556)
exec("make infra-down")
```

---

### TS-01-P6: Clean Build Reproducibility

**Property:** Property 6 from design.md
**Validates:** 01-REQ-6.2, 01-REQ-6.6
**Type:** property
**Description:** For any component, clean then build produces the same
successful result as a fresh build.

**For any:** Component C in {all buildable components}
**Invariant:** `clean(C); build(C)` exits 0 and produces identical binary
checksums as a fresh build.

**Assertion pseudocode:**
```
exec("make clean")
exec("make build")
checksums_1 = collect_binary_checksums()
exec("make clean")
exec("make build")
checksums_2 = collect_binary_checksums()
ASSERT checksums_1 == checksums_2
```

---

### TS-01-P7: Toolchain Detection

**Property:** Property 7 from design.md
**Validates:** 01-REQ-6.E1, 01-REQ-7.E2
**Type:** property
**Description:** For any missing required tool, the build system names it in
the error message.

**For any:** Tool T in {rustc, cargo, go, protoc, podman/docker}
**Invariant:** When T is not on PATH, the Makefile error output contains the
string T.

**Assertion pseudocode:**
```
FOR ANY tool IN ["rustc", "go", "protoc"]:
    modified_path = remove_from_path(tool)
    result = exec("make build", env={"PATH": modified_path})
    ASSERT result.exit_code != 0
    ASSERT contains(result.stderr, tool)
```

---

## Edge Case Tests

### TS-01-E1: Missing required directory

**Requirement:** 01-REQ-1.E1
**Type:** unit
**Description:** Verify build reports clear error when a required directory
is missing.

**Preconditions:**
- Temporarily rename `proto/` to `proto_bak/`.

**Input:**
- Command: `make build`.

**Expected:**
- Build fails. Error output references the missing directory or proto files.

**Assertion pseudocode:**
```
rename("proto/", "proto_bak/")
result = exec("make build")
ASSERT result.exit_code != 0
ASSERT contains(result.stderr, "proto") OR contains(result.stdout, "proto")
rename("proto_bak/", "proto/")
```

---

### TS-01-E2: Proto import paths relative to root

**Requirement:** 01-REQ-2.E1
**Type:** unit
**Description:** Verify proto files use import paths relative to the proto
directory root.

**Preconditions:**
- Proto files exist.

**Input:**
- Check import statements in proto files.

**Expected:**
- All imports reference files by name only (not absolute or relative paths
  outside `proto/`).

**Assertion pseudocode:**
```
FOR EACH file IN glob("proto/*.proto"):
    content = read_file(file)
    FOR EACH import_line IN extract_imports(content):
        ASSERT NOT starts_with(import_line, "/")
        ASSERT NOT starts_with(import_line, "../")
```

---

### TS-01-E3: Missing proto for Rust build

**Requirement:** 01-REQ-3.E1
**Type:** integration
**Description:** Verify Rust build fails clearly when a proto file is missing.

**Preconditions:**
- Temporarily rename `proto/update_service.proto`.

**Input:**
- Command: `cargo build` in `rhivos/`.

**Expected:**
- Build fails. Error references the missing proto file.

**Assertion pseudocode:**
```
rename("proto/update_service.proto", "proto/update_service.proto.bak")
result = exec("cargo build", cwd="rhivos/")
ASSERT result.exit_code != 0
ASSERT contains(result.stderr, "update_service.proto") OR contains(result.stderr, "proto")
rename("proto/update_service.proto.bak", "proto/update_service.proto")
```

---

### TS-01-E4: Missing proto for Go generation

**Requirement:** 01-REQ-4.E1
**Type:** integration
**Description:** Verify `make proto` fails clearly when a proto file is
malformed.

**Preconditions:**
- Temporarily corrupt a proto file.

**Input:**
- Write invalid syntax to `proto/common.proto`, then run `make proto`.

**Expected:**
- `make proto` fails with exit code != 0. Error references the proto file.

**Assertion pseudocode:**
```
backup = read_file("proto/common.proto")
write_file("proto/common.proto", "invalid proto content {{{")
result = exec("make proto")
ASSERT result.exit_code != 0
ASSERT contains(result.stderr, "common.proto") OR contains(result.stderr, "proto")
write_file("proto/common.proto", backup)
```

---

### TS-01-E5: Unknown CLI command

**Requirement:** 01-REQ-5.E1
**Type:** integration
**Description:** Verify mock CLI apps handle unknown commands gracefully.

**Preconditions:**
- Mock CLI apps built.

**Input:**
- Run each mock with an unknown command: `<app> nonexistent-command`.

**Expected:**
- Non-zero exit code. Stderr contains an error message.

**Assertion pseudocode:**
```
FOR EACH app IN ["parking-app-cli", "companion-app-cli"]:
    result = exec("mock/" + app + "/" + app, "nonexistent-command")
    ASSERT result.exit_code != 0
    ASSERT len(result.stderr) > 0
```

---

### TS-01-E6: Missing Rust toolchain

**Requirement:** 01-REQ-6.E1
**Type:** integration
**Description:** Verify Makefile reports missing Rust toolchain clearly.

**Preconditions:**
- `rustc` removed from PATH (simulated).

**Input:**
- Command: `make build` with modified PATH.

**Expected:**
- Exit code != 0. Error names `rustc` or `cargo`.

**Assertion pseudocode:**
```
result = exec("make build", env={"PATH": path_without("rustc")})
ASSERT result.exit_code != 0
ASSERT contains(result.stderr, "rustc") OR contains(result.stderr, "cargo")
```

---

### TS-01-E7: Missing Go toolchain

**Requirement:** 01-REQ-6.E1
**Type:** integration
**Description:** Verify Makefile reports missing Go toolchain clearly.

**Preconditions:**
- `go` removed from PATH (simulated).

**Input:**
- Command: `make build` with modified PATH.

**Expected:**
- Exit code != 0. Error names `go`.

**Assertion pseudocode:**
```
result = exec("make build", env={"PATH": path_without("go")})
ASSERT result.exit_code != 0
ASSERT contains(result.stderr, "go")
```

---

### TS-01-E8: Partial build failure continues

**Requirement:** 01-REQ-6.E2
**Type:** integration
**Description:** Verify build continues after one component fails (where
dependencies allow).

**Preconditions:**
- Temporarily break one Go module's source.

**Input:**
- Corrupt `backend/parking-fee-service/main.go`, run `make build`.

**Expected:**
- Make reports the failure. Other independent components still build.

**Assertion pseudocode:**
```
backup = read_file("backend/parking-fee-service/main.go")
write_file("backend/parking-fee-service/main.go", "package main\nfunc main() { invalid }")
result = exec("make build")
ASSERT result.exit_code != 0
ASSERT contains(result.output, "parking-fee-service") AND contains(result.output, "error")
# Verify Rust components still built
ASSERT directory_exists("rhivos/target/debug/")
write_file("backend/parking-fee-service/main.go", backup)
```

---

### TS-01-E9: Port conflict on infra-up

**Requirement:** 01-REQ-7.E1
**Type:** integration
**Description:** Verify `make infra-up` reports port conflict when a port
is already in use.

**Preconditions:**
- Port 1883 is occupied by another process.

**Input:**
- Occupy port 1883, then run `make infra-up`.

**Expected:**
- Exit code != 0 or container fails to start. Error output references the
  conflicting port.

**Assertion pseudocode:**
```
blocker = occupy_port(1883)
result = exec("make infra-up")
ASSERT result.exit_code != 0 OR NOT port_is_serving_mosquitto(1883)
ASSERT contains(result.output, "1883") OR contains(result.output, "bind")
release_port(blocker)
```

---

### TS-01-E10: Missing container runtime

**Requirement:** 01-REQ-7.E2
**Type:** integration
**Description:** Verify `make infra-up` reports missing container runtime.

**Preconditions:**
- Neither `podman` nor `docker` on PATH (simulated).

**Input:**
- Command: `make infra-up` with modified PATH.

**Expected:**
- Exit code != 0. Error names `podman` or `docker`.

**Assertion pseudocode:**
```
result = exec("make infra-up", env={"PATH": path_without("podman", "docker")})
ASSERT result.exit_code != 0
ASSERT contains(result.stderr, "podman") OR contains(result.stderr, "docker")
```

---

### TS-01-E11: Empty test file is not a failure

**Requirement:** 01-REQ-8.E1
**Type:** unit
**Description:** Verify that test files without test functions do not cause
test runner failures.

**Preconditions:**
- Components build.

**Input:**
- Create an empty test file (Go: `empty_test.go` with only package line;
  Rust: empty `#[cfg(test)]` module).

**Expected:**
- `make test` still exits with code 0.

**Assertion pseudocode:**
```
write_file("backend/parking-fee-service/empty_test.go", "package main\n")
result = exec("make test")
ASSERT result.exit_code == 0
delete_file("backend/parking-fee-service/empty_test.go")
```

---

## Coverage Matrix

| Requirement    | Test Spec Entry | Type        |
|----------------|-----------------|-------------|
| 01-REQ-1.1     | TS-01-1         | unit        |
| 01-REQ-1.2     | TS-01-2         | unit        |
| 01-REQ-1.3     | TS-01-3         | unit        |
| 01-REQ-1.4     | TS-01-4         | unit        |
| 01-REQ-1.5     | TS-01-5         | unit        |
| 01-REQ-1.6     | TS-01-6         | unit        |
| 01-REQ-1.E1    | TS-01-E1        | unit        |
| 01-REQ-2.1     | TS-01-7         | unit        |
| 01-REQ-2.2     | TS-01-8         | unit        |
| 01-REQ-2.3     | TS-01-9         | unit        |
| 01-REQ-2.4     | TS-01-10        | unit        |
| 01-REQ-2.5     | TS-01-11        | unit        |
| 01-REQ-2.E1    | TS-01-E2        | unit        |
| 01-REQ-3.1     | TS-01-12        | unit        |
| 01-REQ-3.2     | TS-01-13        | integration |
| 01-REQ-3.3     | TS-01-14        | integration |
| 01-REQ-3.4     | TS-01-15        | unit        |
| 01-REQ-3.5     | TS-01-16        | unit        |
| 01-REQ-3.E1    | TS-01-E3        | integration |
| 01-REQ-4.1     | TS-01-17        | unit        |
| 01-REQ-4.2     | TS-01-18        | integration |
| 01-REQ-4.3     | TS-01-19        | integration |
| 01-REQ-4.4     | TS-01-20        | unit        |
| 01-REQ-4.5     | TS-01-21        | integration |
| 01-REQ-4.6     | TS-01-22        | integration |
| 01-REQ-4.E1    | TS-01-E4        | integration |
| 01-REQ-5.1     | TS-01-23        | unit        |
| 01-REQ-5.2     | TS-01-24        | unit        |
| 01-REQ-5.3     | TS-01-25        | integration |
| 01-REQ-5.4     | TS-01-26        | integration |
| 01-REQ-5.5     | TS-01-27        | unit        |
| 01-REQ-5.E1    | TS-01-E5        | integration |
| 01-REQ-6.1     | TS-01-28        | unit        |
| 01-REQ-6.2     | TS-01-29        | integration |
| 01-REQ-6.3     | TS-01-30        | integration |
| 01-REQ-6.4     | TS-01-31        | integration |
| 01-REQ-6.5     | TS-01-32        | integration |
| 01-REQ-6.6     | TS-01-33        | integration |
| 01-REQ-6.E1    | TS-01-E6, E7    | integration |
| 01-REQ-6.E2    | TS-01-E8        | integration |
| 01-REQ-7.1     | TS-01-34        | unit        |
| 01-REQ-7.2     | TS-01-35        | unit        |
| 01-REQ-7.3     | TS-01-36        | unit        |
| 01-REQ-7.4     | TS-01-37        | integration |
| 01-REQ-7.5     | TS-01-38        | integration |
| 01-REQ-7.E1    | TS-01-E9        | integration |
| 01-REQ-7.E2    | TS-01-E10       | integration |
| 01-REQ-8.1     | TS-01-39        | unit        |
| 01-REQ-8.2     | TS-01-40        | unit        |
| 01-REQ-8.3     | TS-01-41        | integration |
| 01-REQ-8.4     | TS-01-42        | unit        |
| 01-REQ-8.E1    | TS-01-E11       | unit        |
| Property 1     | TS-01-P1        | property    |
| Property 2     | TS-01-P2        | property    |
| Property 3     | TS-01-P3        | property    |
| Property 4     | TS-01-P4        | property    |
| Property 5     | TS-01-P5        | property    |
| Property 6     | TS-01-P6        | property    |
| Property 7     | TS-01-P7        | property    |
