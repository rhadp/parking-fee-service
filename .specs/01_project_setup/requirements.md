# Requirements Document

## Introduction

This document specifies the requirements for the Project Setup phase (Phase 1.2) of the SDV Parking Demo System. The scope covers establishing the monorepo directory structure, creating skeleton implementations for all components, defining shared Protocol Buffer interfaces, configuring local development infrastructure (NATS, Kuksa Databroker), and setting up build and test toolchains for Rust and Go.

## Glossary

- **Cargo workspace:** A Rust feature that lets multiple related packages (crates) share a common `Cargo.lock` and output directory, managed by a root `Cargo.toml` with a `[workspace]` section.
- **Go workspace:** A Go feature (`go.work` file) that links multiple Go modules so they can be developed together without publishing.
- **DATA_BROKER:** Eclipse Kuksa Databroker — a VSS-compliant vehicle signal broker providing gRPC pub/sub for vehicle signals. Deployed as a pre-built container image.
- **NATS:** A lightweight, high-performance messaging system used for vehicle-to-cloud communication. Deployed as a containerized `nats-server`.
- **VSS:** Vehicle Signal Specification (COVESA standard) — a taxonomy for vehicle data signals.
- **VSS overlay:** A YAML file that extends the standard VSS tree with custom signals (e.g., `Vehicle.Parking.*`, `Vehicle.Command.*`).
- **Podman Compose:** A tool that runs multi-container applications defined in a `compose.yml` file, using Podman as the container runtime.
- **OCI:** Open Container Initiative — the standard for container image formats.
- **Skeleton:** A minimal implementation that compiles, prints usage/version information, and exits with code 0. Does not include business logic or gRPC handler stubs.
- **Proto file:** A `.proto` file defining Protocol Buffer messages and gRPC service interfaces using proto3 syntax.
- **Binary target:** A Cargo concept where a single crate produces multiple executable binaries, each defined in `src/bin/`.
- **Mock CLI app:** A command-line program that simulates an Android application's behavior for integration testing purposes.

## Requirements

### Requirement 1: Monorepo Directory Structure

**User Story:** As a developer, I want a well-organized monorepo with dedicated sub-folders for each technology domain, so that I can navigate the codebase and find components easily.

#### Acceptance Criteria

1. [01-REQ-1.1] THE repository SHALL contain the following top-level directories: `rhivos/`, `backend/`, `android/`, `mobile/`, `mock/`, `proto/`, `deployments/`, `tests/`.
2. [01-REQ-1.2] THE `rhivos/` directory SHALL contain subdirectories for each Rust component: `locking-service/`, `cloud-gateway-client/`, `update-service/`, `parking-operator-adaptor/`, `mock-sensors/`.
3. [01-REQ-1.3] THE `backend/` directory SHALL contain subdirectories for each Go backend service: `parking-fee-service/`, `cloud-gateway/`.
4. [01-REQ-1.4] THE `mock/` directory SHALL contain subdirectories for each mock CLI app: `parking-app-cli/`, `companion-app-cli/`, `parking-operator/`.
5. [01-REQ-1.5] THE `android/` directory SHALL exist as a placeholder for the future AAOS PARKING_APP (Kotlin).
6. [01-REQ-1.6] THE `mobile/` directory SHALL exist as a placeholder for the future Flutter COMPANION_APP.

#### Edge Cases

1. [01-REQ-1.E1] IF a required directory is missing after setup, THEN THE build system SHALL report an error identifying the missing directory.

### Requirement 2: Rust Workspace Configuration

**User Story:** As a Rust developer, I want a properly configured Cargo workspace, so that all RHIVOS services build together and share dependencies.

#### Acceptance Criteria

1. [01-REQ-2.1] THE `rhivos/` directory SHALL contain a root `Cargo.toml` that defines a Cargo workspace with members: `locking-service`, `cloud-gateway-client`, `update-service`, `parking-operator-adaptor`, `mock-sensors`.
2. [01-REQ-2.2] WHEN `cargo build` is run from the `rhivos/` directory, THE workspace SHALL compile all member crates without errors.
3. [01-REQ-2.3] WHEN `cargo test` is run from the `rhivos/` directory, THE workspace SHALL execute all tests across all member crates and report results.
4. [01-REQ-2.4] THE `mock-sensors` crate SHALL define three binary targets: `location-sensor`, `speed-sensor`, `door-sensor`.

#### Edge Cases

1. [01-REQ-2.E1] IF a workspace member crate is missing its `Cargo.toml`, THEN THE Cargo build SHALL fail with a clear error message identifying the missing member.

### Requirement 3: Go Workspace Configuration

**User Story:** As a Go developer, I want a properly configured Go workspace, so that backend services and mock apps can be developed and built together.

#### Acceptance Criteria

1. [01-REQ-3.1] THE repository root SHALL contain a `go.work` file that links the Go modules: `backend/`, `mock/`, `tests/setup/`.
2. [01-REQ-3.2] WHEN `go build ./...` is run from the repository root, THE Go workspace SHALL compile all Go modules without errors.
3. [01-REQ-3.3] WHEN `go test ./...` is run from the repository root, THE Go workspace SHALL execute all tests across all Go modules and report results.

#### Edge Cases

1. [01-REQ-3.E1] IF a Go module listed in `go.work` is missing its `go.mod`, THEN THE Go workspace SHALL fail with a clear error message identifying the missing module.

### Requirement 4: Skeleton Implementations

**User Story:** As a developer, I want every component to have a compilable skeleton, so that I can verify the build system works end-to-end before implementing business logic.

#### Acceptance Criteria

1. [01-REQ-4.1] WHEN a Rust skeleton binary is executed, THE binary SHALL print a usage message containing its component name and exit with code 0.
2. [01-REQ-4.2] WHEN a Go skeleton binary is executed, THE binary SHALL print a usage message containing its component name and exit with code 0.
3. [01-REQ-4.3] THE repository SHALL produce the following Rust binaries: `locking-service`, `cloud-gateway-client`, `update-service`, `parking-operator-adaptor`, `location-sensor`, `speed-sensor`, `door-sensor`.
4. [01-REQ-4.4] THE repository SHALL produce the following Go binaries: `parking-fee-service`, `cloud-gateway`, `parking-app-cli`, `companion-app-cli`, `parking-operator`.

#### Edge Cases

1. [01-REQ-4.E1] IF a skeleton binary is invoked with an unrecognized flag, THEN THE binary SHALL print a usage message and exit with code 0.

### Requirement 5: Protocol Buffer Definitions

**User Story:** As a developer, I want shared .proto definitions with generated Go code, so that all components share a single source of truth for gRPC interfaces.

#### Acceptance Criteria

1. [01-REQ-5.1] THE `proto/` directory SHALL contain the following proto3 files: `common.proto`, `update_service.proto`, `parking_adaptor.proto`.
2. [01-REQ-5.2] THE `common.proto` file SHALL define shared types including `AdapterState` enum, `AdapterInfo` message, and `ErrorDetails` message.
3. [01-REQ-5.3] THE `update_service.proto` file SHALL define the `UpdateService` gRPC service with RPCs: `InstallAdapter`, `WatchAdapterStates`, `ListAdapters`, `RemoveAdapter`, `GetAdapterStatus`.
4. [01-REQ-5.4] THE `parking_adaptor.proto` file SHALL define the `ParkingAdaptor` gRPC service with RPCs: `StartSession`, `StopSession`, `GetStatus`, `GetRate`.
5. [01-REQ-5.5] WHEN `make proto` is run, THE build system SHALL generate Go code from all proto files into the `gen/go/` directory with packages: `commonpb`, `updateservicepb`, `parkingadaptorpb`.
6. [01-REQ-5.6] WHEN `make proto` is run, THE generated Go code SHALL compile without errors.

#### Edge Cases

1. [01-REQ-5.E1] IF the `protoc` compiler is not installed, THEN THE `make proto` target SHALL fail with an error message stating that `protoc` is required.

### Requirement 6: Local Infrastructure

**User Story:** As a developer, I want to start and stop local infrastructure (NATS, Kuksa Databroker) with a single command, so that I can run integration tests locally.

#### Acceptance Criteria

1. [01-REQ-6.1] THE `deployments/` directory SHALL contain a `compose.yml` file defining two services: NATS server (port 4222) and Eclipse Kuksa Databroker (port 55556).
2. [01-REQ-6.2] WHEN `make infra-up` is run, THE build system SHALL start both NATS and Kuksa Databroker containers using Podman Compose.
3. [01-REQ-6.3] WHEN `make infra-down` is run, THE build system SHALL stop and remove all infrastructure containers started by `make infra-up`.
4. [01-REQ-6.4] THE `deployments/` directory SHALL contain a NATS server configuration file at `nats/nats-server.conf`.
5. [01-REQ-6.5] THE `deployments/` directory SHALL contain a VSS overlay file that defines custom signals: `Vehicle.Parking.SessionActive`, `Vehicle.Command.Door.Lock`, `Vehicle.Command.Door.Response`.

#### Edge Cases

1. [01-REQ-6.E1] IF Podman is not running, THEN THE `make infra-up` target SHALL fail with an error message indicating that Podman is required.
2. [01-REQ-6.E2] IF infrastructure containers are already running, THEN THE `make infra-up` target SHALL be idempotent and not create duplicate containers.

### Requirement 7: Build Orchestration

**User Story:** As a developer, I want a root Makefile that orchestrates builds across all toolchains, so that I can build, test, and lint the entire project with simple commands.

#### Acceptance Criteria

1. [01-REQ-7.1] THE root `Makefile` SHALL provide a `build` target that compiles all Rust and Go components.
2. [01-REQ-7.2] THE root `Makefile` SHALL provide a `test` target that runs all unit tests across Rust and Go.
3. [01-REQ-7.3] THE root `Makefile` SHALL provide a `lint` target that runs `cargo clippy` for Rust and `go vet` for Go.
4. [01-REQ-7.4] THE root `Makefile` SHALL provide a `check` target that runs `build`, `test`, and `lint` in sequence.
5. [01-REQ-7.5] THE root `Makefile` SHALL provide a `clean` target that removes all build artifacts.
6. [01-REQ-7.6] THE root `Makefile` SHALL provide `proto`, `infra-up`, and `infra-down` targets.

#### Edge Cases

1. [01-REQ-7.E1] IF a required toolchain (Rust or Go) is not installed, THEN THE corresponding Makefile target SHALL fail with an error message identifying the missing toolchain.

### Requirement 8: Test Infrastructure

**User Story:** As a developer, I want test runners configured for all components with placeholder tests, so that I can verify the test infrastructure works before writing real tests.

#### Acceptance Criteria

1. [01-REQ-8.1] WHEN `cargo test` is run in the `rhivos/` workspace, THE test runner SHALL find and execute at least one test per Rust crate.
2. [01-REQ-8.2] WHEN `go test ./...` is run from the repository root, THE test runner SHALL find and execute at least one test per Go module.
3. [01-REQ-8.3] THE `tests/setup/` directory SHALL contain a standalone Go module with tests that verify the project structure is correct.
4. [01-REQ-8.4] WHEN `make test` is run, THE build system SHALL execute all Rust and Go tests and report a summary of pass/fail results.

#### Edge Cases

1. [01-REQ-8.E1] IF no test files exist in a component directory, THEN THE test runner SHALL report a warning (not fail silently).
