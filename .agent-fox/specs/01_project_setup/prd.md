# PRD: Project Setup

## Overview

This specification covers **Phase 1.2: Setup** from the main PRD. It establishes the monorepo structure, skeleton implementations, local build system, local infrastructure, and local testing capabilities for the parking fee service demo system.

The goal is to create a fully functional development foundation where every component has a compilable/buildable skeleton, shared proto definitions are in place, local infrastructure (NATS, Kuksa Databroker) can be started with a single command, and test runners are configured for all toolchains.

## Scope

### In Scope

- Setup the code repo with dedicated sub-folders for each type of code:
  - `rhivos/` -- Rust services (LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT, UPDATE_SERVICE, PARKING_OPERATOR_ADAPTOR, mock sensors)
  - `backend/` -- Go services (PARKING_FEE_SERVICE, CLOUD_GATEWAY)
  - `android/` -- Kotlin (PARKING_APP placeholder directory only)
  - `mobile/` -- Flutter/Dart (COMPANION_APP placeholder directory only)
  - `mock/` -- Go/Rust mock CLI apps (parking-app-cli, companion-app-cli, parking-operator)
  - `proto/` -- Shared .proto definitions
  - `deployments/` -- Podman Compose files for local infra
- Create skeleton implementations for each component (except Android apps -- just placeholder dirs)
- Create mock CLI apps for PARKING_APP and COMPANION_APP
- Create local build capabilities (root Makefile + per-component build files)
- Setup local infrastructure (containerized NATS server, Kuksa Databroker)
- Setup local unit and integration testing capabilities

### Out of Scope

- Actual component implementation (covered by specs 02-09)
- Android app development (PARKING_APP on AAOS, COMPANION_APP on mobile)
- CI/CD pipelines (Phase 3)
- Cloud deployment
- Production infrastructure

## Components

### Directory Structure

```
parking-fee-service/
  rhivos/                          # Rust workspace
    locking-service/               # ASIL-B door lock service
    cloud-gateway-client/          # NATS-based cloud connectivity
    update-service/                # Adapter lifecycle management
    parking-operator-adaptor/      # Parking session adapter
    mock-sensors/                  # CLI tools: location, speed, door sensors
  backend/                         # Go module workspace
    parking-fee-service/           # Operator discovery REST API
    cloud-gateway/                 # Dual-interface gateway (REST + NATS)
  android/                         # Placeholder for AAOS PARKING_APP
  mobile/                          # Placeholder for Flutter COMPANION_APP
  mock/                            # Mock CLI apps
    parking-app-cli/               # Go - simulates PARKING_APP
    companion-app-cli/             # Go - simulates COMPANION_APP
    parking-operator/              # Go - simulates parking operator REST API
  proto/                           # Shared .proto definitions
  deployments/                     # Podman Compose files
  Makefile                         # Root build orchestration
```

### Technology Stack

- **Rust** (edition 2021): RHIVOS services, using Cargo workspace
- **Go** (1.22+): Backend services and mock CLI apps, using Go workspace
- **Protocol Buffers** (proto3): Shared interface definitions
- **Podman Compose**: Local infrastructure
- **NATS** (nats-server container): Message broker for vehicle-cloud communication
- **Eclipse Kuksa Databroker** (container): VSS-compliant vehicle signal broker
- **Make**: Build orchestration

## Dependencies

This spec has no cross-spec dependencies. It is the foundation upon which all other specs build.

## Clarifications

The following clarifications were obtained during requirements analysis.

- **C1 (Skeleton scope):** Skeleton implementations print usage/version info and exit 0. No gRPC handler stubs — those are introduced by component specs 02-09.
- **C2 (Testing capabilities):** Test runners are configured so `cargo test` and `go test ./...` succeed. Each component includes at least one trivial placeholder test (e.g., `#[test] fn it_compiles()` or `TestMain`).
- **C3 (Infrastructure directory):** The `deployments/` directory contains `compose.yml` (NATS on :4222, Kuksa Databroker on :55556) and `nats/nats-server.conf`.
- **C4 (Android directories):** `android/` is the placeholder for the AAOS PARKING_APP (Kotlin). `mobile/` is the placeholder for the Flutter COMPANION_APP. These match the PRD (the README diverges here and is outdated).
- **C5 (Mock sensors):** `rhivos/mock-sensors/` is a single Cargo crate with three binary targets (`location-sensor`, `speed-sensor`, `door-sensor`) that share common DATA_BROKER gRPC client code.
- **C6 (Mock parking-operator):** `mock/parking-operator/` is a Go binary under `mock/`, simulating the PARKING_OPERATOR REST API.
- **C7 (Proto contents):** Proto files contain full message and service definitions (types, enums, RPC signatures), not just placeholders. They define the contract for all downstream component specs.
- **C8 (Infrastructure configuration):** Default NATS config (port 4222). Kuksa Databroker on port 55556 with a custom VSS overlay file for `Vehicle.Parking.*` and `Vehicle.Command.*` signals. Configuration files live in `deployments/`.
- **C9 (Tests directory):** `tests/setup/` is a standalone Go module with shell-script-driven tests that verify: all binaries compile, infrastructure starts and stops cleanly, and proto code generation works.
- **C10 (PRD authority):** The PRD is authoritative. Where the README diverges from the PRD, the README is outdated and should be updated to match.
