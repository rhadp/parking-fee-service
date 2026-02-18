# Requirements Document: Repository Setup

## Introduction

This specification defines the foundational project structure, build system,
protocol buffer definitions, local development infrastructure, and mock CLI
applications for the SDV Parking Demo System. It establishes the scaffolding
upon which all subsequent implementation specs (02–06) build.

## Glossary

| Term | Definition |
|------|-----------|
| AAOS | Android Automotive OS — the IVI platform |
| DATA_BROKER | Eclipse Kuksa Databroker — VSS-compliant signal broker |
| gRPC | Google Remote Procedure Call — the IPC/RPC framework |
| IVI | In-Vehicle Infotainment |
| Kuksa | Eclipse Kuksa — open-source vehicle data management |
| Mock CLI app | A command-line program that implements the same gRPC interface as a real component, used for integration testing |
| OCI | Open Container Initiative — container image standard |
| Proto / Protobuf | Protocol Buffers — Google's interface definition language |
| QM | Quality Management — non-safety-critical ASIL level |
| RHIVOS | Red Hat In-Vehicle OS |
| Skeleton | A compilable but non-functional implementation that establishes project structure, dependencies, and build targets |
| UDS | Unix Domain Socket — local IPC mechanism |
| VIN | Vehicle Identification Number |
| VSS | Vehicle Signal Specification (COVESA) |

## Requirements

### Requirement 1: Project Directory Structure

**User Story:** As a developer, I want a well-organized monorepo with
dedicated directories per technology domain, so that I can navigate the
codebase and work on components independently.

#### Acceptance Criteria

1. **01-REQ-1.1** THE repository SHALL contain a `rhivos/` directory with
   sub-directories for each Rust service: `locking-service/`,
   `cloud-gateway-client/`, `parking-operator-adaptor/`, `update-service/`.

2. **01-REQ-1.2** THE repository SHALL contain a `backend/` directory with
   sub-directories for each Go service: `parking-fee-service/`,
   `cloud-gateway/`.

3. **01-REQ-1.3** THE repository SHALL contain an `android/` directory with
   placeholder sub-directories: `parking-app/` and `companion-app/`.

4. **01-REQ-1.4** THE repository SHALL contain a `proto/` directory with
   sub-directories: `services/` and `common/`.

5. **01-REQ-1.5** THE repository SHALL contain a `mock/` directory with
   sub-directories for mock CLI applications: `parking-app-cli/` and
   `companion-app-cli/`.

6. **01-REQ-1.6** THE repository SHALL contain a `mock/sensors/` directory
   for mock sensor CLI tools.

7. **01-REQ-1.7** THE repository SHALL contain supporting directories:
   `containers/`, `infra/`, `scripts/`, `docs/`, `tests/`.

#### Edge Cases

1. **01-REQ-1.E1** IF a required directory is missing from the repository,
   THEN the build system SHALL report a clear error identifying the missing
   directory.

---

### Requirement 2: Rust Workspace

**User Story:** As a Rust developer, I want all RHIVOS services managed as a
Cargo workspace, so that I can share dependencies and build all services with
a single command.

#### Acceptance Criteria

1. **01-REQ-2.1** THE `rhivos/` directory SHALL contain a `Cargo.toml`
   workspace manifest listing all Rust service crates and the mock sensors
   crate as workspace members.

2. **01-REQ-2.2** WHEN `cargo build` is run from the `rhivos/` directory,
   THE build system SHALL compile all workspace members successfully.

3. **01-REQ-2.3** WHEN `cargo test` is run from the `rhivos/` directory,
   THE build system SHALL execute all unit tests across workspace members.

4. **01-REQ-2.4** Each Rust service crate SHALL produce a named binary
   matching the service name (e.g., `locking-service`, `update-service`).

#### Edge Cases

1. **01-REQ-2.E1** IF a workspace member has an incompatible dependency
   version, THEN `cargo build` SHALL fail with a clear dependency resolution
   error (standard Cargo behavior).

---

### Requirement 3: Go Modules

**User Story:** As a Go developer, I want properly configured Go modules for
backend services and mock CLI apps, so that dependencies are managed
correctly and builds are reproducible.

#### Acceptance Criteria

1. **01-REQ-3.1** Each Go service directory (`backend/parking-fee-service/`,
   `backend/cloud-gateway/`) SHALL contain a valid `go.mod` file.

2. **01-REQ-3.2** Each mock CLI directory (`mock/parking-app-cli/`,
   `mock/companion-app-cli/`) SHALL contain a valid `go.mod` file.

3. **01-REQ-3.3** WHEN `go build ./...` is run from a Go service directory,
   THE build system SHALL compile the service binary successfully.

4. **01-REQ-3.4** WHEN `go test ./...` is run from a Go service directory,
   THE build system SHALL execute all unit tests.

#### Edge Cases

1. **01-REQ-3.E1** IF a Go module's dependencies cannot be resolved, THEN
   `go build` SHALL fail with a clear module resolution error (standard Go
   behavior).

---

### Requirement 4: Protocol Buffer Definitions

**User Story:** As a developer, I want shared `.proto` files that define
service interfaces, so that all components use consistent message types and
RPC contracts.

#### Acceptance Criteria

1. **01-REQ-4.1** THE `proto/services/update_service.proto` file SHALL define
   the UPDATE_SERVICE gRPC interface with RPCs: `InstallAdapter`,
   `WatchAdapterStates`, `ListAdapters`, `RemoveAdapter`, `GetAdapterStatus`.

2. **01-REQ-4.2** THE `proto/services/parking_adapter.proto` file SHALL define
   the PARKING_OPERATOR_ADAPTOR gRPC interface with RPCs: `StartSession`,
   `StopSession`, `GetStatus`, `GetRate`.

3. **01-REQ-4.3** THE `proto/common/common.proto` file SHALL define shared
   message types used across services (e.g., `Location`, `VehicleId`,
   `AdapterInfo`, `AdapterState` enum, `ErrorDetails`).

4. **01-REQ-4.4** WHEN the proto generation target is run, THE build system
   SHALL produce valid Go bindings under each Go service's `gen/` or `pb/`
   directory.

5. **01-REQ-4.5** WHEN the proto generation target is run, THE build system
   SHALL produce valid Rust bindings consumable by the Rust workspace.

6. **01-REQ-4.6** THE proto files SHALL use `proto3` syntax.

#### Edge Cases

1. **01-REQ-4.E1** IF a `.proto` file contains a syntax error, THEN `protoc`
   SHALL fail with a line-specific error message (standard protoc behavior).

2. **01-REQ-4.E2** IF generated bindings are out of date with respect to the
   `.proto` source, THEN the build system SHALL regenerate them when the proto
   target is invoked.

---

### Requirement 5: Root Build System

**User Story:** As a developer, I want a single root Makefile that
orchestrates builds, tests, proto generation, and infrastructure across all
technology domains, so that I do not need to know per-component build
commands.

#### Acceptance Criteria

1. **01-REQ-5.1** THE root `Makefile` SHALL provide a `make build` target that
   builds all Rust and Go components.

2. **01-REQ-5.2** THE root `Makefile` SHALL provide a `make test` target that
   runs all unit tests across Rust and Go components.

3. **01-REQ-5.3** THE root `Makefile` SHALL provide a `make proto` target that
   generates language bindings from all `.proto` files.

4. **01-REQ-5.4** THE root `Makefile` SHALL provide `make infra-up` and
   `make infra-down` targets to start and stop local development
   infrastructure.

5. **01-REQ-5.5** THE root `Makefile` SHALL provide a `make lint` target that
   runs linters for all components (clippy for Rust, golangci-lint or go vet
   for Go).

6. **01-REQ-5.6** THE root `Makefile` SHALL provide a `make clean` target that
   removes all build artifacts.

#### Edge Cases

1. **01-REQ-5.E1** IF a required tool (cargo, go, protoc, podman) is not
   installed, THEN the relevant Makefile target SHALL fail with a message
   identifying the missing tool.

---

### Requirement 6: Local Development Infrastructure

**User Story:** As a developer, I want containerized local instances of
Eclipse Kuksa Databroker and Eclipse Mosquitto, so that I can run and test
services locally without cloud dependencies.

#### Acceptance Criteria

1. **01-REQ-6.1** THE `infra/` directory SHALL contain a container
   composition file (Podman Compose or Docker Compose) that starts Eclipse
   Kuksa Databroker and Eclipse Mosquitto.

2. **01-REQ-6.2** WHEN `make infra-up` is run, THE infrastructure SHALL start
   Kuksa Databroker accessible on a documented gRPC port.

3. **01-REQ-6.3** WHEN `make infra-up` is run, THE infrastructure SHALL start
   Mosquitto accessible on a documented MQTT port.

4. **01-REQ-6.4** WHEN `make infra-down` is run, THE infrastructure SHALL
   stop and remove all infrastructure containers.

5. **01-REQ-6.5** THE `infra/` directory SHALL contain configuration files
   for Mosquitto (listener, authentication) and Kuksa (VSS model, access
   control) suitable for local development.

#### Edge Cases

1. **01-REQ-6.E1** IF a port required by infrastructure is already in use,
   THEN `make infra-up` SHALL fail with a message identifying the port
   conflict (standard container runtime behavior).

2. **01-REQ-6.E2** IF the container runtime (podman/docker) is not available,
   THEN `make infra-up` SHALL fail with a message identifying the missing
   runtime.

---

### Requirement 7: Skeleton Implementations

**User Story:** As a developer, I want compilable skeleton implementations for
each service, so that the project structure is established and I can start
implementing real functionality in subsequent specs.

#### Acceptance Criteria

1. **01-REQ-7.1** Each Rust service skeleton SHALL contain a `main.rs` that
   starts a gRPC server on a configurable address and registers the service
   (with unimplemented/stub handlers).

2. **01-REQ-7.2** Each Go service skeleton SHALL contain a `main.go` that
   starts an HTTP server (for REST services) or gRPC server on a configurable
   address with stub handlers.

3. **01-REQ-7.3** THE skeleton implementations SHALL use the generated
   protobuf bindings for their respective service interfaces.

4. **01-REQ-7.4** WHEN a skeleton service receives any RPC call, THE service
   SHALL respond with a gRPC `UNIMPLEMENTED` status code (or HTTP 501 for
   REST endpoints).

#### Edge Cases

1. **01-REQ-7.E1** IF a skeleton service cannot bind to its configured
   address, THEN the service SHALL exit with a non-zero exit code and log an
   error message.

---

### Requirement 8: Mock CLI Applications

**User Story:** As a developer, I want mock CLI applications that simulate the
PARKING_APP and COMPANION_APP, so that I can integration-test backend and
RHIVOS services without real Android builds.

#### Acceptance Criteria

1. **01-REQ-8.1** THE mock `parking-app-cli` SHALL be a Go CLI application
   that can invoke gRPC calls against UPDATE_SERVICE and
   PARKING_OPERATOR_ADAPTOR using the shared proto definitions.

2. **01-REQ-8.2** THE mock `companion-app-cli` SHALL be a Go CLI application
   that can invoke REST calls against CLOUD_GATEWAY.

3. **01-REQ-8.3** THE mock CLI applications SHALL accept target service
   addresses as command-line flags or environment variables.

4. **01-REQ-8.4** THE mock CLI applications SHALL use the same `.proto`
   definitions and message schemas as the real Android applications will.

#### Edge Cases

1. **01-REQ-8.E1** IF the target service is unreachable, THEN the mock CLI
   SHALL print an error message and exit with a non-zero exit code.

---

### Requirement 9: Mock Sensor CLI Tools

**User Story:** As a developer, I want CLI tools that publish mock sensor data
(location, speed, door status) to the DATA_BROKER, so that I can simulate
vehicle state for testing without real hardware.

#### Acceptance Criteria

1. **01-REQ-9.1** THE `mock/sensors/` directory SHALL contain a Rust
   application that can publish mock values to Kuksa Databroker via gRPC for:
   `Vehicle.CurrentLocation.Latitude`,
   `Vehicle.CurrentLocation.Longitude`,
   `Vehicle.Speed`,
   `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`.

2. **01-REQ-9.2** THE mock sensor tool SHALL accept the DATA_BROKER address
   as a command-line flag or environment variable.

3. **01-REQ-9.3** THE mock sensor tool SHALL support setting individual signal
   values via command-line subcommands or arguments.

#### Edge Cases

1. **01-REQ-9.E1** IF the DATA_BROKER is unreachable, THEN the mock sensor
   tool SHALL print an error message and exit with a non-zero exit code.

---

### Requirement 10: Container Build Definitions

**User Story:** As a developer, I want Containerfiles for each service, so
that I can build OCI-compliant container images for local testing and
deployment.

#### Acceptance Criteria

1. **01-REQ-10.1** THE `containers/` directory SHALL contain Containerfiles
   for each Rust service and each Go service.

2. **01-REQ-10.2** WHEN a Containerfile is built, THE resulting image SHALL
   contain only the compiled binary and minimal runtime dependencies
   (multi-stage build).

3. **01-REQ-10.3** THE root `Makefile` SHALL provide a `make build-containers`
   target that builds all container images.

4. **01-REQ-10.4** Each container image SHALL be tagged with the service name
   and a `latest` tag by default.

#### Edge Cases

1. **01-REQ-10.E1** IF the container runtime is not available, THEN
   `make build-containers` SHALL fail with a message identifying the missing
   runtime.
