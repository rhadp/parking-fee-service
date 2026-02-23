# Requirements Document: Project Setup (Phase 1.2)

## Introduction

This document specifies the requirements for the initial project setup of the
SDV Parking Demo System. The setup phase establishes the repository structure,
shared protocol definitions, skeleton implementations, mock CLI applications,
build system, local development infrastructure, and testing framework. The goal
is to enable Phase 2 component implementation to proceed without structural
blockers.

## Glossary

| Term | Definition |
|------|-----------|
| Skeleton implementation | A compilable project containing interface stubs and placeholder types but no business logic. Functions return default/zero values or `unimplemented` errors. |
| Mock CLI app | A command-line application that simulates the behavior of an Android application by exposing the same gRPC/REST interfaces. Used for integration testing without real Android builds. |
| Local infrastructure | Containerized services (MQTT broker, DATA_BROKER) required for local development and integration testing. |
| Proto definition | A Protocol Buffer (`.proto`) file defining gRPC service interfaces, messages, and enums. |
| Workspace | A Cargo workspace (Rust) or Go workspace that groups related packages for unified building and dependency management. |
| DATA_BROKER | Eclipse Kuksa Databroker — a pre-built VSS-compliant gRPC signal broker. Not reimplemented; deployed as-is. |
| UDS | Unix Domain Socket — local IPC transport for same-partition gRPC. |
| OCI | Open Container Initiative — standard for container image format and distribution. |

## Requirements

### Requirement 1: Repository Directory Structure

**User Story:** As a developer, I want a well-organized monorepo structure with
dedicated directories per technology domain, so that I can navigate the
codebase and work on any component without confusion.

#### Acceptance Criteria

1. THE repository SHALL contain a top-level directory for Rust components
   (RHIVOS services). `01-REQ-1.1`
2. THE repository SHALL contain a top-level directory for Go components
   (backend services). `01-REQ-1.2`
3. THE repository SHALL contain a top-level directory for shared protocol
   buffer definitions. `01-REQ-1.3`
4. THE repository SHALL contain a top-level directory for mock CLI
   applications. `01-REQ-1.4`
5. THE repository SHALL contain placeholder directories for the PARKING_APP
   (Kotlin/AAOS) and COMPANION_APP (Flutter/Dart) Android applications.
   `01-REQ-1.5`
6. THE repository SHALL contain a top-level directory for local infrastructure
   configuration (container definitions for development services).
   `01-REQ-1.6`

#### Edge Cases

1. IF a required directory is missing after checkout, THEN the build system
   SHALL report a clear error identifying the missing directory. `01-REQ-1.E1`

---

### Requirement 2: Shared Protocol Buffer Definitions

**User Story:** As a developer, I want shared `.proto` files for all gRPC
interfaces, so that Rust and Go components use identical service contracts and
message types.

#### Acceptance Criteria

1. THE proto directory SHALL contain a `.proto` file defining the
   UPDATE_SERVICE gRPC interface (InstallAdapter, WatchAdapterStates,
   ListAdapters, RemoveAdapter, GetAdapterStatus). `01-REQ-2.1`
2. THE proto directory SHALL contain a `.proto` file defining the
   PARKING_OPERATOR_ADAPTOR gRPC interface (StartSession, StopSession,
   GetStatus, GetRate). `01-REQ-2.2`
3. THE proto directory SHALL contain a `.proto` file defining shared message
   types used across services (adapter lifecycle states, error details, common
   identifiers). `01-REQ-2.3`
4. WHEN `protoc` is invoked on any `.proto` file, THE file SHALL compile
   without errors. `01-REQ-2.4`
5. THE proto files SHALL use `proto3` syntax. `01-REQ-2.5`

#### Edge Cases

1. IF a `.proto` file references a message type from another `.proto` file,
   THEN the import path SHALL be relative to the proto directory root.
   `01-REQ-2.E1`

---

### Requirement 3: Rust Component Scaffolding

**User Story:** As a developer, I want compilable Rust skeleton projects for
all RHIVOS services, so that I can start implementing component logic
immediately in Phase 2.

#### Acceptance Criteria

1. THE repository SHALL contain a Cargo workspace with members for
   LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT, UPDATE_SERVICE, and
   PARKING_OPERATOR_ADAPTOR. `01-REQ-3.1`
2. WHEN `cargo build` is run in the workspace root, THE build system SHALL
   compile all Rust members successfully with zero errors. `01-REQ-3.2`
3. WHEN `cargo test` is run in the workspace root, THE test framework SHALL
   execute without errors (tests may be empty or trivially passing).
   `01-REQ-3.3`
4. THE Rust skeleton for each gRPC service SHALL include generated code from
   the shared `.proto` definitions. `01-REQ-3.4`
5. THE Rust skeletons SHALL include stub implementations that return
   `unimplemented` gRPC status for each service method. `01-REQ-3.5`

#### Edge Cases

1. IF a required `.proto` file is missing or malformed, THEN `cargo build`
   SHALL fail with an error message referencing the proto file. `01-REQ-3.E1`

---

### Requirement 4: Go Component Scaffolding

**User Story:** As a developer, I want compilable Go skeleton projects for
backend services, so that I can start implementing PARKING_FEE_SERVICE and
CLOUD_GATEWAY in Phase 2.

#### Acceptance Criteria

1. THE repository SHALL contain Go modules for PARKING_FEE_SERVICE and
   CLOUD_GATEWAY. `01-REQ-4.1`
2. WHEN `go build ./...` is run in each Go module directory, THE build system
   SHALL compile the module successfully with zero errors. `01-REQ-4.2`
3. WHEN `go test ./...` is run in each Go module directory, THE test framework
   SHALL execute without errors (tests may be empty or trivially passing).
   `01-REQ-4.3`
4. THE Go skeletons SHALL include generated code from the shared `.proto`
   definitions (where the component uses gRPC). `01-REQ-4.4`
5. THE PARKING_FEE_SERVICE skeleton SHALL include a stub HTTP server that
   starts and responds to a health check endpoint. `01-REQ-4.5`
6. THE CLOUD_GATEWAY skeleton SHALL include stub HTTP and MQTT server
   entry points. `01-REQ-4.6`

#### Edge Cases

1. IF a required `.proto` file is missing or malformed, THEN `go generate`
   SHALL fail with an error message referencing the proto file. `01-REQ-4.E1`

---

### Requirement 5: Mock CLI Applications

**User Story:** As a developer, I want mock CLI applications that simulate
PARKING_APP and COMPANION_APP, so that I can integration-test backend and
RHIVOS components without real Android builds.

#### Acceptance Criteria

1. THE repository SHALL contain a mock PARKING_APP CLI application written
   in Go. `01-REQ-5.1`
2. THE repository SHALL contain a mock COMPANION_APP CLI application written
   in Go. `01-REQ-5.2`
3. WHEN built, each mock CLI application SHALL produce a single executable
   binary. `01-REQ-5.3`
4. WHEN run without arguments, each mock CLI application SHALL display a help
   message listing available commands. `01-REQ-5.4`
5. THE mock CLI applications SHALL share the same `.proto` definitions and
   generated code as the real service implementations. `01-REQ-5.5`

#### Edge Cases

1. IF a mock CLI application is invoked with an unknown command, THEN it
   SHALL print an error message and exit with a non-zero exit code.
   `01-REQ-5.E1`

---

### Requirement 6: Build System

**User Story:** As a developer, I want a unified build system that can build
all components with simple commands, so that I don't need to remember
per-component build incantations.

#### Acceptance Criteria

1. THE repository SHALL contain a top-level Makefile. `01-REQ-6.1`
2. WHEN `make build` is run, THE build system SHALL build all Rust and Go
   components (skeletons and mocks). `01-REQ-6.2`
3. WHEN `make test` is run, THE build system SHALL run all tests across all
   components. `01-REQ-6.3`
4. WHEN `make lint` is run, THE build system SHALL run linters for all
   components (clippy for Rust, golangci-lint or go vet for Go). `01-REQ-6.4`
5. WHEN `make proto` is run, THE build system SHALL regenerate all code from
   `.proto` definitions for both Rust and Go. `01-REQ-6.5`
6. WHEN `make clean` is run, THE build system SHALL remove all build
   artifacts. `01-REQ-6.6`

#### Edge Cases

1. IF a required toolchain (Rust, Go, protoc) is not installed, THEN the
   build system SHALL fail with a clear error message naming the missing
   tool. `01-REQ-6.E1`
2. IF `make build` fails for one component, THEN the build system SHALL
   report the failure and continue building remaining components (unless
   dependencies prevent it). `01-REQ-6.E2`

---

### Requirement 7: Local Development Infrastructure

**User Story:** As a developer, I want containerized local infrastructure
(MQTT broker, DATA_BROKER), so that I can run integration tests locally
without cloud dependencies.

#### Acceptance Criteria

1. THE repository SHALL contain a container composition file (Podman-compatible)
   that defines local development services. `01-REQ-7.1`
2. THE local infrastructure SHALL include an Eclipse Mosquitto MQTT broker.
   `01-REQ-7.2`
3. THE local infrastructure SHALL include an Eclipse Kuksa Databroker
   instance. `01-REQ-7.3`
4. WHEN `make infra-up` is run, THE infrastructure services SHALL start and
   become reachable on their configured ports. `01-REQ-7.4`
5. WHEN `make infra-down` is run, THE infrastructure services SHALL stop and
   release all ports. `01-REQ-7.5`

#### Edge Cases

1. IF a required port is already in use, THEN the infrastructure startup
   SHALL fail with a clear error message identifying the conflicting port.
   `01-REQ-7.E1`
2. IF the container runtime (Podman/Docker) is not installed, THEN
   `make infra-up` SHALL fail with a clear error message naming the missing
   tool. `01-REQ-7.E2`

---

### Requirement 8: Testing Framework Setup

**User Story:** As a developer, I want pre-configured testing frameworks for
each technology stack, so that I can write and run tests from day one of
Phase 2 implementation.

#### Acceptance Criteria

1. THE Rust workspace SHALL be configured with the standard Rust test framework
   and include at least one passing placeholder test per component. `01-REQ-8.1`
2. THE Go modules SHALL be configured with Go's built-in testing package and
   include at least one passing placeholder test per component. `01-REQ-8.2`
3. WHEN `make test` is run with all infrastructure stopped, THE unit tests
   SHALL still pass (unit tests SHALL NOT depend on running infrastructure).
   `01-REQ-8.3`
4. THE repository SHALL contain a directory structure for integration tests
   that are kept separate from unit tests. `01-REQ-8.4`

#### Edge Cases

1. IF a test file exists but contains no test functions, THEN the test runner
   SHALL not count it as a failure. `01-REQ-8.E1`
