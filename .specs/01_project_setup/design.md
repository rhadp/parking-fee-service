# Design Document: Project Setup

## Overview

This document defines the architecture and design decisions for the initial project setup of the parking fee service demo system. The setup creates a monorepo with a Rust workspace for RHIVOS services, a Go workspace for backend services and mock apps, shared proto definitions, a root Makefile for build orchestration, and containerized local infrastructure (NATS + Kuksa Databroker).

## Architecture

### Directory Structure

```
parking-fee-service/
├── Makefile                              # Root build orchestration
├── rhivos/                               # Rust workspace
│   ├── Cargo.toml                        # Workspace manifest
│   ├── locking-service/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs                   # Skeleton: print + exit(0)
│   ├── cloud-gateway-client/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── update-service/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── parking-operator-adaptor/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   └── mock-sensors/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs                    # Library crate with sensor modules
├── backend/                              # Go workspace
│   ├── go.work                           # Go workspace file
│   ├── parking-fee-service/
│   │   ├── go.mod
│   │   └── main.go                      # Skeleton: print + exit(0)
│   └── cloud-gateway/
│       ├── go.mod
│       └── main.go
├── android/                              # Placeholder
│   └── README.md
├── mobile/                               # Placeholder
│   └── README.md
├── mock/                                 # Mock CLI apps (Go)
│   ├── go.work                           # Go workspace file
│   ├── parking-app-cli/
│   │   ├── go.mod
│   │   └── main.go
│   ├── companion-app-cli/
│   │   ├── go.mod
│   │   └── main.go
│   └── parking-operator/
│       ├── go.mod
│       └── main.go
├── proto/                                # Shared .proto definitions
│   └── parking/
│       └── v1/
│           └── common.proto              # Common message types
├── deployments/                          # Local infrastructure
│   └── docker-compose.yml                # NATS + Kuksa Databroker
└── .specs/                               # Specification documents
```

### Module Responsibilities

1. **rhivos/** -- Rust workspace containing all RHIVOS partition services. Each service is an independent crate that compiles to a binary (except mock-sensors which is a library).
2. **backend/** -- Go workspace containing cloud-deployed backend services. Each service is an independent Go module.
3. **mock/** -- Go workspace containing mock CLI applications that simulate Android apps and the parking operator for integration testing.
4. **proto/** -- Shared Protocol Buffer definitions used by both Rust and Go components for gRPC interface consistency.
5. **deployments/** -- Infrastructure-as-code for local development: compose files to run NATS and Kuksa Databroker.
6. **Makefile** -- Root build orchestration providing uniform targets across all toolchains.

## Build System Design

### Root Makefile Targets

| Target | Description | Implementation |
|--------|------------|----------------|
| `build` | Compile all components | `cargo build` in rhivos/ + `go build ./...` in backend/ and mock/ modules |
| `test` | Run all unit tests | `cargo test` in rhivos/ + `go test ./...` in backend/ and mock/ modules |
| `lint` | Run all linters | `cargo clippy` in rhivos/ + `go vet ./...` in backend/ and mock/ modules |
| `clean` | Remove all build artifacts | `cargo clean` in rhivos/ + `go clean` in backend/ and mock/ modules |
| `infra-up` | Start local infrastructure | `docker compose -f deployments/docker-compose.yml up -d` |
| `infra-down` | Stop local infrastructure | `docker compose -f deployments/docker-compose.yml down` |
| `proto` | Validate proto files | `protoc --lint_out=. proto/**/*.proto` (or buf lint) |

### Per-Component Builds

Each component is independently buildable:

- **Rust crates:** Standard `cargo build` / `cargo test` within the workspace or per-crate.
- **Go modules:** Standard `go build ./...` / `go test ./...` within each module directory.
- **Proto validation:** `protoc` or `buf` for syntax checking.

### Skeleton Implementation Pattern

All skeletons follow the same pattern:

**Rust binary crates:**
```rust
fn main() {
    println!("{service_name} starting...");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_startup() {
        // Validates the binary can be built and the startup path works
        assert!(true);
    }
}
```

**Rust library crates (mock-sensors):**
```rust
pub mod location;
pub mod speed;
pub mod door;

// Each module is empty but compiles
```

**Go binaries:**
```go
package main

import "fmt"

func main() {
    fmt.Println("{service_name} starting...")
}
```

**Go tests:**
```go
package main

import "testing"

func TestStartup(t *testing.T) {
    // Validates the package compiles and startup path works
}
```

## Local Infrastructure

### Docker Compose Configuration

The `deployments/docker-compose.yml` defines two services:

| Service | Image | Ports | Purpose |
|---------|-------|-------|---------|
| nats | `nats:latest` | 4222 (client), 8222 (monitoring) | NATS message broker for vehicle-cloud communication |
| kuksa-databroker | `ghcr.io/eclipse-kuksa/kuksa-databroker:master` | 55555 (gRPC) | VSS-compliant vehicle signal broker |

### Port Assignments

| Port | Service | Protocol |
|------|---------|----------|
| 4222 | NATS client connections | TCP |
| 8222 | NATS HTTP monitoring | HTTP |
| 55555 | Kuksa Databroker gRPC | gRPC/HTTP2 |

### Health Checks

- NATS: TCP connection to port 4222
- Kuksa Databroker: gRPC health check on port 55555

## Proto Directory Structure

```
proto/
└── parking/
    └── v1/
        └── common.proto      # Shared message types (VIN, Location, etc.)
```

The proto files use `parking.v1` as the package namespace. Additional service-specific proto files will be added by subsequent component specs.

### common.proto Contents (Skeleton)

- Package: `parking.v1`
- Syntax: `proto3`
- Placeholder message types for future use (e.g., `Location`, `VehicleIdentifier`)

## Correctness Properties

### Property 1: Directory Completeness

*For any* required directory in the specification (rhivos/, backend/, android/, mobile/, mock/, proto/, deployments/), THE repository SHALL contain that directory and it SHALL not be empty (each contains at least one file or subdirectory).

**Validates: Requirements 01-REQ-1.1, 01-REQ-1.2, 01-REQ-1.3, 01-REQ-1.4**

### Property 2: Build Determinism

*For any* clean checkout of the repository, THE build system (`make build`) SHALL complete successfully with zero errors on two consecutive runs, producing equivalent build artifacts.

**Validates: Requirements 01-REQ-2.2, 01-REQ-3.3, 01-REQ-6.2**

### Property 3: Test Discoverability

*For any* component directory (Rust crate or Go module), THE test runner SHALL discover at least one test and all discovered tests SHALL pass.

**Validates: Requirements 01-REQ-2.3, 01-REQ-3.4, 01-REQ-4.3, 01-REQ-9.1, 01-REQ-9.2**

### Property 4: Skeleton Exit Behavior

*For any* skeleton binary produced by the build, WHEN executed without arguments or configuration, THE binary SHALL exit with code 0 and produce output on stdout.

**Validates: Requirements 01-REQ-4.1, 01-REQ-4.2, 01-REQ-4.E1**

### Property 5: Infrastructure Lifecycle

*For any* execution of the infrastructure lifecycle (`make infra-up` followed by `make infra-down`), THE system SHALL leave no orphaned containers or networks from the compose file.

**Validates: Requirements 01-REQ-7.2, 01-REQ-7.3**

### Property 6: Proto Validity

*For any* .proto file in the `proto/` directory, THE file SHALL be syntactically valid proto3 and pass protoc compilation without errors.

**Validates: Requirements 01-REQ-5.1, 01-REQ-5.2**

### Property 7: Mock CLI Usage Output

*For any* mock CLI app binary (parking-app-cli, companion-app-cli, parking-operator), WHEN executed without arguments, THE binary SHALL print a usage message to stdout and exit with code 0.

**Validates: Requirements 01-REQ-8.1, 01-REQ-8.2, 01-REQ-8.3, 01-REQ-8.4**

## Error Handling

| Error Condition | Behavior | Requirement |
|----------------|----------|-------------|
| Rust workspace member missing Cargo.toml | `cargo build` reports clear error | 01-REQ-2.E1 |
| Go module missing go.mod | `go build` reports clear error | 01-REQ-3.E1 |
| Skeleton binary run without config | Exits cleanly with code 0 | 01-REQ-4.E1 |
| Proto file has syntax error | protoc reports error clearly | 01-REQ-5.E1 |
| Component build failure during `make build` | Non-zero exit, clear error output | 01-REQ-6.E1 |
| Container port already in use | Compose reports port conflict | 01-REQ-7.E1 |
| Docker/Podman not installed | `make infra-up` reports missing dependency | 01-REQ-7.E2 |
| Unknown subcommand to mock CLI | Error message with valid subcommands, non-zero exit | 01-REQ-8.E1 |
| No tests in a package | Test runner reports "no test files" | 01-REQ-9.E1 |

## Technology Stack

| Technology | Version / Source | Purpose |
|-----------|-----------------|---------|
| Rust | Edition 2021 (stable) | RHIVOS services |
| Cargo | Bundled with Rust | Rust build system and package manager |
| Go | 1.22+ | Backend services and mock CLI apps |
| Protocol Buffers | proto3 | Interface definitions |
| protoc | Latest | Proto file compilation and validation |
| Docker / Podman | Latest | Container runtime for local infrastructure |
| Docker Compose / Podman Compose | Latest | Multi-container orchestration |
| NATS Server | Latest (nats:latest image) | Message broker |
| Eclipse Kuksa Databroker | master (ghcr.io/eclipse-kuksa/kuksa-databroker:master) | Vehicle signal broker |
| GNU Make | 3.81+ | Build orchestration |

## Definition of Done

A task group is complete when ALL of the following are true:

1. All subtasks within the group are checked off (`[x]`)
2. All spec tests (`test_spec.md` entries) for the task group pass
3. All property tests for the task group pass
4. All previously passing tests still pass (no regressions)
5. No linter warnings or errors introduced
6. Code is committed on a feature branch and pushed to remote
7. Feature branch is merged back to `develop`
8. `tasks.md` checkboxes are updated to reflect completion

## Testing Strategy

### Unit Tests

- **Rust:** Each crate contains `#[cfg(test)]` modules with at least one test validating compilation and basic behavior. Run via `cargo test`.
- **Go:** Each module contains `_test.go` files with at least one test validating compilation and basic behavior. Run via `go test ./...`.

### Integration Tests

- **Infrastructure tests:** Shell scripts or test harnesses that verify NATS and Kuksa Databroker are reachable after `make infra-up`. These validate port connectivity and basic health.
- **Build system tests:** Shell scripts that verify `make build`, `make test`, `make clean` work correctly end-to-end.

### Property-Based Tests

- **Directory structure:** Script-based tests that enumerate expected directories and verify their existence and non-emptiness.
- **Binary behavior:** Tests that build and execute each skeleton binary, verifying exit code 0 and stdout output.

### Test Execution

All tests are runnable via:
- `make test` -- runs all unit tests across all toolchains
- `cargo test` -- runs Rust tests only
- `go test ./...` -- runs Go tests only (per module)
