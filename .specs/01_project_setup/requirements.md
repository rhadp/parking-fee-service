# Requirements Document

## Introduction

This document specifies the requirements for the initial project setup of the parking fee service demo system. It covers repository structure, skeleton implementations, build system, local infrastructure, and test runner configuration. This is the foundation spec -- all subsequent component specs depend on it.

## Glossary

| Term | Definition |
|------|-----------|
| Skeleton | A minimal, compilable/buildable project that produces a binary or library but implements no business logic |
| Cargo workspace | Rust's mechanism for managing multiple related packages in a single repository |
| Go workspace | Go's mechanism (go.work) for managing multiple modules in a single repository |
| NATS | A lightweight, high-performance messaging system used for vehicle-cloud communication |
| Kuksa Databroker | Eclipse Kuksa's VSS-compliant vehicle signal broker providing gRPC pub/sub for vehicle signals |
| Proto definitions | Protocol Buffer (.proto) files defining gRPC service interfaces and message types |
| Mock CLI app | A command-line program that simulates the interface of a real component for integration testing |
| Compose file | A YAML file (docker-compose.yml or podman-compose.yml) defining multi-container local infrastructure |
| VIN | Vehicle Identification Number, used to uniquely identify a vehicle in the system |

## Requirements

### Requirement 1: Repository Directory Structure

**User Story:** As a developer, I want a well-organized monorepo with dedicated sub-folders for each technology domain, so that I can navigate the codebase and work on components independently.

#### Acceptance Criteria

1. `01-REQ-1.1` THE repository SHALL contain the following top-level directories: `rhivos/`, `backend/`, `android/`, `mobile/`, `mock/`, `proto/`, `deployments/`.
2. `01-REQ-1.2` THE `rhivos/` directory SHALL contain sub-directories for each Rust service: `locking-service/`, `cloud-gateway-client/`, `update-service/`, `parking-operator-adaptor/`, `mock-sensors/`.
3. `01-REQ-1.3` THE `backend/` directory SHALL contain sub-directories for each Go service: `parking-fee-service/`, `cloud-gateway/`.
4. `01-REQ-1.4` THE `mock/` directory SHALL contain sub-directories for each mock CLI app: `parking-app-cli/`, `companion-app-cli/`, `parking-operator/`.

#### Edge Cases

1. `01-REQ-1.E1` THE `android/` directory SHALL exist as a placeholder containing only a README.md file, WHEN no Android development has been started.
2. `01-REQ-1.E2` THE `mobile/` directory SHALL exist as a placeholder containing only a README.md file, WHEN no Flutter development has been started.

### Requirement 2: Rust Workspace Configuration

**User Story:** As a Rust developer, I want a properly configured Cargo workspace, so that all RHIVOS services share dependencies and build together.

#### Acceptance Criteria

1. `01-REQ-2.1` THE `rhivos/` directory SHALL contain a `Cargo.toml` workspace file that lists all Rust service crates as workspace members.
2. `01-REQ-2.2` WHEN `cargo build` is executed in the `rhivos/` directory, THE build system SHALL compile all workspace members successfully with zero errors.
3. `01-REQ-2.3` WHEN `cargo test` is executed in the `rhivos/` directory, THE test runner SHALL execute and pass all unit tests across all workspace members.

#### Edge Cases

1. `01-REQ-2.E1` IF a workspace member has a missing or malformed `Cargo.toml`, THEN THE build system SHALL report a clear error identifying the problematic crate.

### Requirement 3: Go Workspace Configuration

**User Story:** As a Go developer, I want a properly configured Go workspace, so that backend services and mock apps share module references and build together.

#### Acceptance Criteria

1. `01-REQ-3.1` THE `backend/` directory SHALL contain a `go.work` file that lists all Go service modules as workspace members.
2. `01-REQ-3.2` THE `mock/` directory SHALL contain a `go.work` file that lists all mock CLI app modules as workspace members.
3. `01-REQ-3.3` WHEN `go build ./...` is executed in each Go module directory, THE build system SHALL compile the module successfully with zero errors.
4. `01-REQ-3.4` WHEN `go test ./...` is executed in each Go module directory, THE test runner SHALL execute and pass all unit tests.

#### Edge Cases

1. `01-REQ-3.E1` IF a Go module has a missing or malformed `go.mod`, THEN THE build system SHALL report a clear error identifying the problematic module.

### Requirement 4: Skeleton Implementations

**User Story:** As a developer, I want each component to have a minimal compilable skeleton, so that the build pipeline works end-to-end from day one.

#### Acceptance Criteria

1. `01-REQ-4.1` WHEN built, each Rust service skeleton SHALL produce a binary that prints a startup message and exits with code 0.
2. `01-REQ-4.2` WHEN built, each Go service skeleton SHALL produce a binary that prints a startup message and exits with code 0.
3. `01-REQ-4.3` Each skeleton SHALL include at least one passing unit test that validates the startup behavior.

#### Edge Cases

1. `01-REQ-4.E1` IF a skeleton binary is executed without any configuration, THEN THE binary SHALL exit cleanly with code 0 and a human-readable message, not panic or crash.

### Requirement 5: Shared Proto Definitions

**User Story:** As a developer, I want shared .proto files in a single directory, so that all components use consistent interface definitions.

#### Acceptance Criteria

1. `01-REQ-5.1` THE `proto/` directory SHALL contain at least one .proto file with a package declaration and proto3 syntax.
2. `01-REQ-5.2` THE .proto files in `proto/` SHALL be syntactically valid and pass `protoc` validation without errors.

#### Edge Cases

1. `01-REQ-5.E1` IF a .proto file contains syntax errors, THEN `protoc --lint_out=. proto/*.proto` or equivalent validation SHALL report the error clearly.

### Requirement 6: Root Build System

**User Story:** As a developer, I want a single Makefile at the repo root, so that I can build, test, and manage all components with uniform commands.

#### Acceptance Criteria

1. `01-REQ-6.1` THE root `Makefile` SHALL provide the following targets: `build`, `test`, `lint`, `clean`, `infra-up`, `infra-down`.
2. `01-REQ-6.2` WHEN `make build` is executed, THE build system SHALL compile all Rust and Go components successfully.
3. `01-REQ-6.3` WHEN `make test` is executed, THE build system SHALL run all unit tests across all Rust and Go components.
4. `01-REQ-6.4` WHEN `make clean` is executed, THE build system SHALL remove all build artifacts from all components.

#### Edge Cases

1. `01-REQ-6.E1` IF a component build fails, THEN `make build` SHALL report the failure clearly and exit with a non-zero status code.

### Requirement 7: Local Infrastructure

**User Story:** As a developer, I want to start local infrastructure (NATS, Kuksa Databroker) with a single command, so that I can run integration tests locally.

#### Acceptance Criteria

1. `01-REQ-7.1` THE `deployments/` directory SHALL contain a compose file (docker-compose.yml or podman-compose.yml) that defines NATS server and Kuksa Databroker services.
2. `01-REQ-7.2` WHEN `make infra-up` is executed, THE infrastructure SHALL start NATS server and Kuksa Databroker containers and they SHALL be reachable on their configured ports within 30 seconds.
3. `01-REQ-7.3` WHEN `make infra-down` is executed, THE infrastructure SHALL stop and remove all containers started by `make infra-up`.

#### Edge Cases

1. `01-REQ-7.E1` IF a container port is already in use, THEN the compose file SHALL fail with a clear error message indicating the port conflict.
2. `01-REQ-7.E2` IF Docker/Podman is not installed or not running, THEN `make infra-up` SHALL report a clear error indicating the missing dependency.

### Requirement 8: Mock CLI App Skeletons

**User Story:** As a developer, I want mock CLI apps for PARKING_APP and COMPANION_APP, so that I can integration-test backend and RHIVOS components without real Android builds.

#### Acceptance Criteria

1. `01-REQ-8.1` THE `mock/parking-app-cli/` skeleton SHALL build into an executable Go binary named `parking-app-cli`.
2. `01-REQ-8.2` THE `mock/companion-app-cli/` skeleton SHALL build into an executable Go binary named `companion-app-cli`.
3. `01-REQ-8.3` THE `mock/parking-operator/` skeleton SHALL build into an executable Go binary named `parking-operator`.
4. `01-REQ-8.4` WHEN executed without arguments, each mock CLI app SHALL print a usage message and exit with code 0.

#### Edge Cases

1. `01-REQ-8.E1` IF an unknown subcommand is provided, THEN each mock CLI app SHALL print an error message listing valid subcommands and exit with a non-zero status code.

### Requirement 9: Test Runner Configuration

**User Story:** As a developer, I want pre-configured test runners for all toolchains, so that I can run tests immediately without manual setup.

#### Acceptance Criteria

1. `01-REQ-9.1` WHEN `cargo test` is executed in `rhivos/`, THE test runner SHALL discover and execute all tests across the Rust workspace.
2. `01-REQ-9.2` WHEN `go test ./...` is executed in each Go module directory, THE test runner SHALL discover and execute all tests in that module.
3. `01-REQ-9.3` WHEN `make test` is executed at the repo root, THE build system SHALL run tests for all Rust and Go components and report a combined pass/fail result.

#### Edge Cases

1. `01-REQ-9.E1` IF no tests exist in a package, THEN THE test runner SHALL report "no test files" rather than failing.

### Requirement 10: Mock Sensor Tools

**User Story:** As a developer, I want mock sensor CLI tools (location, speed, door), so that I can simulate vehicle signal inputs during integration testing.

#### Acceptance Criteria

1. `01-REQ-10.1` THE `rhivos/mock-sensors/` crate SHALL build into a library or set of binaries providing mock sensor functionality for location, speed, and door signals.
2. `01-REQ-10.2` WHEN built, THE mock-sensors crate SHALL compile successfully as part of the Rust workspace.

#### Edge Cases

1. `01-REQ-10.E1` IF mock-sensors is built standalone outside the workspace, THEN THE build SHALL still succeed using its own Cargo.toml.
