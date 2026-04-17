# Requirements: Project Setup

## Introduction

This document defines the acceptance requirements for **Spec 01 — Project Setup**, the foundation specification for the parking fee service demo system. It covers monorepo directory structure, skeleton implementations, shared protocol buffer definitions, local build orchestration, local infrastructure (NATS, Kuksa Databroker), and test runner configuration. All subsequent component specs (02–09) depend on the artifacts produced here.

Requirements use EARS (Easy Approach to Requirements Syntax) notation. Keywords WHEN, WHILE, WHERE, IF, THEN, and SHALL appear in CAPS.

## Glossary

| Term | Definition |
|------|-----------|
| Monorepo | A single Git repository containing all project components (Rust services, Go services, mock apps, proto definitions, deployment configs). |
| Skeleton | A minimal, compilable/buildable implementation of a component that prints version/usage info and exits with code 0. No business logic or gRPC handler stubs. |
| Cargo Workspace | A Rust project structure where multiple crates share a common `Cargo.lock` and build configuration via a root `Cargo.toml`. |
| Go Workspace | A Go project structure using `go.work` to manage multiple modules under a single build graph. |
| Proto Definition | A Protocol Buffers (proto3) file containing message types, enums, and RPC service signatures that define inter-component contracts. |
| Podman Compose | A container orchestration tool compatible with Docker Compose file format, using Podman as the container runtime. |
| NATS | A lightweight, high-performance messaging system used for vehicle-to-cloud communication. |
| Kuksa Databroker | Eclipse Kuksa Databroker — a VSS-compliant vehicle signal broker providing gRPC pub/sub for vehicle signals. |
| VSS | COVESA Vehicle Signal Specification — a standardized taxonomy for vehicle signals. |
| VSS Overlay | A supplementary VSS definition file that adds custom signals (e.g., `Vehicle.Parking.*`, `Vehicle.Command.*`) to the standard VSS tree. |
| Placeholder Directory | A directory containing only a `README.md` explaining its future purpose. No buildable code. |
| Root Makefile | The top-level `Makefile` at the repository root that orchestrates builds, tests, and infrastructure management across all components. |
| Mock CLI App | A lightweight command-line program that simulates the behavior of an Android application (PARKING_APP or COMPANION_APP) by exposing the same interfaces. |
| Setup Tests | A standalone Go test module under `tests/setup/` that validates the project setup: compilation, infrastructure lifecycle, and proto code generation. |

## Requirements

### [01-REQ-1] Monorepo Directory Structure

**User Story:** As a developer, I need a well-organized monorepo so that each component type has a dedicated location and I can navigate the codebase predictably.

**Acceptance Criteria:**

1. The repository SHALL contain a `rhivos/` directory with subdirectories `locking-service/`, `cloud-gateway-client/`, `update-service/`, `parking-operator-adaptor/`, and `mock-sensors/`.
2. The repository SHALL contain a `backend/` directory with subdirectories `parking-fee-service/` and `cloud-gateway/`.
3. The repository SHALL contain an `android/` placeholder directory with a `README.md` explaining it is reserved for the AAOS PARKING_APP.
4. The repository SHALL contain a `mobile/` placeholder directory with a `README.md` explaining it is reserved for the Flutter COMPANION_APP.
5. The repository SHALL contain a `mock/` directory with subdirectories `parking-app-cli/`, `companion-app-cli/`, and `parking-operator/`.
6. The repository SHALL contain a `proto/` directory for shared Protocol Buffer definitions.
7. The repository SHALL contain a `deployments/` directory for Podman Compose files and infrastructure configuration.
8. The repository SHALL contain a `tests/setup/` directory for setup verification tests.

**Edge Cases:**

- [01-REQ-1.E1] IF a developer creates a file outside the defined directory structure, THEN the build system SHALL still function correctly for all defined components.

---

### [01-REQ-2] Rust Workspace Configuration

**User Story:** As a Rust developer, I need a properly configured Cargo workspace so that all RHIVOS services share dependencies and build together.

**Acceptance Criteria:**

1. The `rhivos/` directory SHALL contain a root `Cargo.toml` that declares a Cargo workspace with members `locking-service`, `cloud-gateway-client`, `update-service`, `parking-operator-adaptor`, and `mock-sensors`.
2. Each Rust workspace member SHALL be a valid Cargo crate with its own `Cargo.toml` and `src/main.rs`.
3. The `mock-sensors` crate SHALL declare three binary targets: `location-sensor`, `speed-sensor`, and `door-sensor`.
4. WHEN `cargo build` is invoked in the `rhivos/` directory, THEN all workspace members SHALL compile without errors and return exit code 0.

**Edge Cases:**

- [01-REQ-2.E1] IF a single workspace member has a compilation error, THEN `cargo build` SHALL report the failing crate by name in its error output.

---

### [01-REQ-3] Go Workspace Configuration

**User Story:** As a Go developer, I need a properly configured Go workspace so that backend services and mock apps share a consistent build environment.

**Acceptance Criteria:**

1. The repository root SHALL contain a `go.work` file that references the Go modules: `backend/parking-fee-service`, `backend/cloud-gateway`, `mock/parking-app-cli`, `mock/companion-app-cli`, `mock/parking-operator`, and `tests/setup`.
2. Each Go module SHALL contain a `go.mod` file with a module path matching its directory location under the repository.
3. Each Go module (except `tests/setup`) SHALL contain a `main.go` file with a `main` function.
4. WHEN `go build ./...` is invoked from the repository root using the Go workspace, THEN all modules SHALL compile without errors.

**Edge Cases:**

- [01-REQ-3.E1] IF a Go module declares a dependency not listed in `go.mod`, THEN `go build` SHALL fail with a clear import error for that module.

---

### [01-REQ-4] Skeleton Implementations

**User Story:** As a developer, I need every component skeleton to be runnable so that I can verify the build pipeline works end-to-end before adding business logic.

**Acceptance Criteria:**

1. WHEN a Rust skeleton binary is executed, THEN it SHALL print a version string to stdout and exit with code 0.
2. WHEN a Go skeleton binary is executed, THEN it SHALL print a version string to stdout and exit with code 0.
3. Each mock-sensor binary (`location-sensor`, `speed-sensor`, `door-sensor`) SHALL print its name and version when executed and exit with code 0.
4. The version string printed by each skeleton SHALL contain the component name.

**Edge Cases:**

- [01-REQ-4.E1] IF a skeleton binary is invoked with an unrecognized flag, THEN it SHALL print a usage message to stderr and exit with a non-zero exit code.

---

### [01-REQ-5] Protocol Buffer Definitions

**User Story:** As a developer, I need shared proto definitions so that all components use the same message types and service interfaces.

**Acceptance Criteria:**

1. The `proto/` directory SHALL contain `.proto` files defining message types and RPC service signatures for: Kuksa Databroker value types, UPDATE_SERVICE operations (InstallAdapter, WatchAdapterStates, ListAdapters, RemoveAdapter, GetAdapterStatus), PARKING_OPERATOR_ADAPTOR operations (StartSession, StopSession, GetStatus, GetRate), and CLOUD_GATEWAY_CLIENT command relay types.
2. All `.proto` files SHALL use `syntax = "proto3"`.
3. All `.proto` files SHALL specify a `go_package` option and a `package` declaration.
4. WHEN `protoc` is invoked on any `.proto` file in the `proto/` directory, THEN it SHALL parse without errors, returning exit code 0.

**Edge Cases:**

- [01-REQ-5.E1] IF a `.proto` file references an import that does not exist in the `proto/` directory, THEN `protoc` SHALL fail with a clear error identifying the missing import.

---

### [01-REQ-6] Root Makefile Build Orchestration

**User Story:** As a developer, I need a single entry point to build, test, and manage all components so that I do not have to remember per-component commands.

**Acceptance Criteria:**

1. The root `Makefile` SHALL provide the following targets: `build`, `test`, `clean`, `proto`, `infra-up`, `infra-down`, and `check`.
2. WHEN `make build` is invoked, THEN it SHALL build all Rust workspace members and all Go modules, returning exit code 0 on success.
3. WHEN `make test` is invoked, THEN it SHALL run `cargo test` in `rhivos/` and `go test ./...` for all Go modules, returning exit code 0 when all tests pass.
4. WHEN `make clean` is invoked, THEN it SHALL remove build artifacts for both Rust and Go toolchains.
5. WHEN `make check` is invoked, THEN it SHALL run linting and all tests, returning exit code 0 when everything passes.

**Edge Cases:**

- [01-REQ-6.E1] IF `make build` is invoked and one toolchain fails, THEN the Makefile SHALL report which toolchain failed and return a non-zero exit code.

---

### [01-REQ-7] Local Infrastructure

**User Story:** As a developer, I need containerized NATS and Kuksa Databroker instances so that I can run integration tests locally without cloud dependencies.

**Acceptance Criteria:**

1. The `deployments/` directory SHALL contain a `compose.yml` file that defines services for NATS (port 4222) and Kuksa Databroker (port 55556).
2. The `deployments/` directory SHALL contain a `nats/nats-server.conf` configuration file for the NATS service.
3. The `deployments/` directory SHALL contain a VSS overlay file that defines custom signals: `Vehicle.Parking.SessionActive`, `Vehicle.Command.Door.Lock`, and `Vehicle.Command.Door.Response`.
4. WHEN `make infra-up` is invoked, THEN Podman Compose SHALL start the NATS and Kuksa Databroker containers and return exit code 0.
5. WHEN `make infra-down` is invoked, THEN Podman Compose SHALL stop and remove the infrastructure containers and return exit code 0.

**Edge Cases:**

- [01-REQ-7.E1] IF port 4222 or 55556 is already in use WHEN `make infra-up` is invoked, THEN Podman Compose SHALL report a port conflict error and the container SHALL not start.
- [01-REQ-7.E2] IF `make infra-down` is invoked when no infrastructure containers are running, THEN it SHALL complete without error, returning exit code 0.

---

### [01-REQ-8] Test Runner Configuration

**User Story:** As a developer, I need test runners configured for all toolchains so that I can run tests from day one and incrementally add test cases.

**Acceptance Criteria:**

1. Each Rust crate in the workspace SHALL contain at least one unit test that asserts the crate compiles successfully (e.g., `#[test] fn it_compiles() { assert!(true); }`).
2. Each Go module (except `tests/setup`) SHALL contain at least one test function (e.g., `func TestMain(t *testing.T) { }`) that passes.
3. WHEN `cargo test` is invoked in `rhivos/`, THEN all Rust placeholder tests SHALL pass and return exit code 0.
4. WHEN `go test ./...` is invoked using the Go workspace, THEN all Go placeholder tests SHALL pass and return exit code 0.

**Edge Cases:**

- [01-REQ-8.E1] IF a test file has a syntax error, THEN the test runner SHALL report the file and line number and return a non-zero exit code.

---

### [01-REQ-9] Setup Verification Tests

**User Story:** As a developer, I need automated verification tests that confirm the entire project setup is correct so that I can catch structural regressions early.

**Acceptance Criteria:**

1. The `tests/setup/` directory SHALL contain a Go test module with tests that verify: all Rust binaries compile successfully, all Go binaries compile successfully, and proto files parse without errors.
2. WHEN the setup verification tests are run, THEN each test SHALL invoke the relevant build or parse command as a subprocess and assert exit code 0.
3. The setup verification tests SHALL be runnable via `make test-setup` from the repository root.
4. Each setup verification test SHALL return a clear pass/fail result with the name of what was verified.

**Edge Cases:**

- [01-REQ-9.E1] IF a required toolchain (cargo, go, protoc) is not installed, THEN the setup verification test SHALL skip with a message indicating the missing tool rather than failing with an obscure error.

---

### [01-REQ-10] Proto Code Generation

**User Story:** As a developer, I need a Make target to generate Go and Rust code from proto definitions so that proto changes are reflected in all components automatically.

**Acceptance Criteria:**

1. WHEN `make proto` is invoked, THEN it SHALL invoke `protoc` to generate Go code from all `.proto` files in the `proto/` directory.
2. The generated Go code SHALL be placed in a location importable by the Go modules that depend on it.
3. WHEN `make proto` succeeds, THEN the generated code SHALL be compilable by `go build` without additional manual steps.

**Edge Cases:**

- [01-REQ-10.E1] IF `protoc` is not installed WHEN `make proto` is invoked, THEN the Makefile SHALL print an error message stating that `protoc` is required and return a non-zero exit code.
