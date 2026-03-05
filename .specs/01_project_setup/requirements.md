# Requirements: Project Setup (Spec 01)

> EARS-syntax requirements for the SDV Parking Demo monorepo scaffolding.
> Derived from the PRD at `.specs/01_project_setup/prd.md` and the master PRD at `.specs/prd.md`.

## Notation

Requirements use EARS (Easy Approach to Requirements Syntax) patterns:

| Pattern | Template |
|---------|----------|
| Ubiquitous | The system SHALL ... |
| Event-driven | WHEN [event], the system SHALL ... |
| State-driven | WHILE [state], the system SHALL ... |
| Conditional | IF [condition], THEN the system SHALL ... |
| Complex | WHILE [state], WHEN [event], the system SHALL ... |

## Requirements

### 01-REQ-1: Repository Directory Structure

The repository SHALL contain the following top-level directories: `proto/`, `gen/go/`, `rhivos/`, `backend/`, `mock/`, `aaos/`, `android/`, `infra/`, and `tests/`.

**Edge cases:**

- IF any required directory is missing, THEN the `make check` target SHALL fail with a non-zero exit code.
- The `aaos/` and `android/` directories SHALL exist as placeholder directories containing only a `.gitkeep` file.

### 01-REQ-2: Protocol Buffer Definitions

The `proto/` directory SHALL contain three `.proto` files: `common.proto`, `update_service.proto`, and `parking_adaptor.proto`, each specifying `syntax = "proto3"` and a `go_package` option targeting the corresponding `gen/go/` sub-package.

**Edge cases:**

- IF a `.proto` file contains a syntax error, THEN `make proto` SHALL fail with a non-zero exit code and print the protoc error message to stderr.
- Each `.proto` file SHALL use the package prefix `sdv.parking.v1` to avoid protobuf namespace collisions.

### 01-REQ-3: Go Code Generation from Protos

WHEN `make proto` is executed, the build system SHALL invoke `protoc` with `protoc-gen-go` and `protoc-gen-go-grpc` plugins to generate Go source files into `gen/go/commonpb/`, `gen/go/updateservicepb/`, and `gen/go/parkingadaptorpb/`.

**Edge cases:**

- IF `protoc`, `protoc-gen-go`, or `protoc-gen-go-grpc` is not installed, THEN `make proto` SHALL fail with a descriptive error message indicating which tool is missing.
- WHEN `make proto` is executed and generated files already exist, the build system SHALL overwrite them with freshly generated output.
- The generated Go package under `gen/go/` SHALL contain a valid `go.mod` file so it can be referenced by other Go modules in the workspace.

### 01-REQ-4: Rust Workspace with Skeleton Crates

The `rhivos/` directory SHALL contain a `Cargo.toml` workspace manifest that declares the following member crates: `locking-service`, `cloud-gateway-client`, `update-service`, and `parking-operator-adaptor`. Each crate SHALL compile successfully with `cargo build` even though it contains only skeleton code (a `main.rs` with an empty or minimal `main` function).

**Edge cases:**

- IF a crate listed in the workspace manifest does not exist on disk, THEN `cargo build` SHALL fail with an error identifying the missing crate.
- Each skeleton crate SHALL declare `edition = "2021"` in its `Cargo.toml`.

### 01-REQ-5: Build System (Makefile)

The repository SHALL contain a top-level `Makefile` providing the following targets: `build`, `test`, `lint`, `check`, `proto`, `infra-up`, `infra-down`, and `clean`.

- `build` SHALL compile all Rust crates (`cargo build --workspace` in `rhivos/`) and all Go modules (`go build ./...` for each module in `backend/` and `mock/`).
- `test` SHALL run all unit tests across Rust (`cargo test --workspace`) and Go (`go test ./...`).
- `lint` SHALL run `cargo clippy --workspace -- -D warnings` and `go vet ./...` for each Go module.
- `check` SHALL execute `build`, `test`, and `lint` in sequence, failing on the first error.
- `proto` SHALL regenerate Go code from `.proto` definitions.
- `infra-up` SHALL start local infrastructure via `docker-compose up -d` (or `podman-compose up -d`).
- `infra-down` SHALL stop local infrastructure via `docker-compose down` (or `podman-compose down`).
- `clean` SHALL remove Rust build artifacts (`cargo clean` in `rhivos/`) and Go build caches.

**Edge cases:**

- IF any sub-target of `check` fails, THEN `make check` SHALL exit immediately with the failing target's exit code.
- WHEN `make clean` is executed in a repository with no build artifacts, the target SHALL succeed without error.

### 01-REQ-6: Local Infrastructure (Docker-Compose)

The `infra/` directory SHALL contain a `docker-compose.yml` file that defines two services:

1. A NATS server (`nats-server`) exposing port 4222 on the host.
2. An Eclipse Kuksa Databroker exposing gRPC on port 55556 on the host.

WHEN `make infra-up` is executed, both services SHALL start and become reachable on their respective ports within 30 seconds.

**Edge cases:**

- IF port 4222 or port 55556 is already in use on the host, THEN `docker-compose up` SHALL fail with an error indicating the port conflict.
- WHEN `make infra-down` is executed and no infrastructure is running, the command SHALL succeed without error.
- Each service SHALL define a health check so that `docker-compose ps` reports health status.

### 01-REQ-7: Go Workspace

The repository root SHALL contain a `go.work` file (Go 1.22+) that references all Go modules: `backend/parking-fee-service`, `backend/cloud-gateway`, `mock/parking-app-cli`, `mock/companion-app-cli`, `gen/go`, and `tests/setup`. WHEN any referenced module is built, the Go toolchain SHALL resolve cross-module imports within the workspace without requiring `go mod download` of workspace-local dependencies.

**Edge cases:**

- IF a module listed in `go.work` does not have a valid `go.mod`, THEN `go work sync` SHALL fail with a diagnostic error.
- IF a new Go module is added to the repository but not listed in `go.work`, THEN cross-module imports from that module SHALL fail to resolve.

## Traceability

| Requirement | PRD Deliverable |
|-------------|-----------------|
| 01-REQ-1 | Deliverable 1: Repository structure |
| 01-REQ-2 | Deliverable 2: Protocol Buffer definitions |
| 01-REQ-3 | Deliverable 2: Protocol Buffer definitions (code gen) |
| 01-REQ-4 | Deliverable 6: Rust workspace |
| 01-REQ-5 | Deliverable 3: Build system |
| 01-REQ-6 | Deliverable 4: Local infrastructure |
| 01-REQ-7 | Deliverable 5: Go workspace |
