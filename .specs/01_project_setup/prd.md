# PRD: Project Setup (Phase 1.2)

> Extracted from the main PRD (`.specs/prd.md`) — Phase 1.2: Setup.

## Context

This specification covers the initial project setup for the SDV Parking Demo
System. It establishes the repository structure, build system, shared protocol
definitions, skeleton implementations, mock CLI applications, local development
infrastructure, and testing framework — everything needed so that Phase 2
component implementation can proceed immediately.

## Scope

From the main PRD, Phase 1.2:

- Setup the code repo, with dedicated sub-folders for each type of code:
  - RHIVOS services: Rust
  - Android Automotive OS app: Kotlin
  - Android app: Flutter, Dart
  - Backend-services: Golang
- Create skeleton implementations for each component, except for the Android
  Automotive OS and Android app. They will come in a later stage. Just keep the
  placeholder directories.
- Create mock CLI apps for PARKING_APP and COMPANION_APP (see *Android
  Development Separation* in the main PRD) so that backend-services and RHIVOS
  components can be integration-tested without real Android builds.
- Create local build capabilities for each toolchain using make/cmake etc.
- Setup local infrastructure, used for local unit and integration testing.
- Setup local unit and integration testing capabilities.

## Components Requiring Skeletons

Skeleton implementations must compile successfully but contain only interface
stubs (no business logic). The following components require skeletons:

| Component                | Language | Partition / Domain       |
|--------------------------|----------|--------------------------|
| LOCKING_SERVICE          | Rust     | RHIVOS Safety Partition  |
| CLOUD_GATEWAY_CLIENT     | Rust     | RHIVOS Safety Partition  |
| UPDATE_SERVICE           | Rust     | RHIVOS QM Partition      |
| PARKING_OPERATOR_ADAPTOR | Rust     | RHIVOS QM Partition      |
| PARKING_FEE_SERVICE      | Go       | Backend (Cloud)          |
| CLOUD_GATEWAY            | Go       | Backend (Cloud)          |

## Components Requiring Mock CLI Apps

| Mock App               | Language | Simulates        | Primary Interface |
|------------------------|----------|-------------------|-------------------|
| Mock PARKING_APP CLI   | Go       | PARKING_APP       | gRPC + REST       |
| Mock COMPANION_APP CLI | Go       | COMPANION_APP     | REST              |

## Components Requiring Placeholder Directories Only

| Component      | Language      | Notes                          |
|----------------|---------------|--------------------------------|
| PARKING_APP    | Kotlin (AAOS) | Developed in a later phase     |
| COMPANION_APP  | Flutter/Dart  | Developed in a later phase     |

## Shared Protocol Buffer Definitions

All gRPC interfaces defined in the main PRD must have `.proto` files:

- DATA_BROKER: Uses Eclipse Kuksa Databroker's existing proto (VSS gRPC API)
- UPDATE_SERVICE: InstallAdapter, WatchAdapterStates, ListAdapters,
  RemoveAdapter, GetAdapterStatus
- PARKING_OPERATOR_ADAPTOR: StartSession, StopSession, GetStatus, GetRate

## Local Infrastructure

- Eclipse Mosquitto (MQTT broker) — containerized via Podman
- Eclipse Kuksa Databroker — containerized or binary for local testing

## Technology Stack

- Rust: Cargo workspace, edition 2021+
- Go: Go modules, version 1.22+
- Protocol Buffers: protoc with language-specific plugins (prost for Rust,
  protoc-gen-go for Go)
- Containers: Podman / docker-compose compatible
- Build: Top-level Makefile orchestrating per-component builds

## Out-of-Scope for This Spec

- Any business logic implementation (that's Phase 2+)
- Android or Flutter project setup beyond placeholder directories
- Cloud deployment configuration
- CI/CD pipeline setup (Phase 3)
- Mock PARKING_OPERATOR (Phase 2.3)
- Mock sensor services: LOCATION_SENSOR, SPEED_SENSOR, DOOR_SENSOR (Phase 2.1)

## Dependencies

| Spec | Relationship | Notes |
|------|-------------|-------|
| (none) | — | This is the first spec; no dependencies |
