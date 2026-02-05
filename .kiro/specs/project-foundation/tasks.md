# Implementation Plan: Project Foundation

## Overview

This implementation plan establishes the foundational infrastructure for the SDV Parking Demo System. Tasks are organized to build incrementally, starting with project structure, then shared definitions, followed by build system, local infrastructure, and finally documentation.

## Tasks

- [x] 1. Create monorepo directory structure
  - [x] 1.1 Create root project structure with all required directories
    - Create `rhivos/` directory with subdirectories for each Rust service
    - Create `android/parking-app/` and `android/companion-app/` directories
    - Create `backend/` directory for Go services
    - Create `proto/` directory with `vss/`, `services/`, and `common/` subdirectories
    - Create `containers/` directory with `rhivos/`, `backend/`, and `mock/` subdirectories
    - Create `infra/` directory with `compose/`, `certs/`, and `config/` subdirectories
    - Create `scripts/` and `docs/` directories
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_

- [x] 2. Implement Protocol Buffer definitions
  - [x] 2.1 Create common error handling proto definitions
    - Create `proto/common/error.proto` with ErrorDetails message
    - Define standard error codes for domain-specific errors
    - _Requirements: 2.6_
  
  - [x] 2.2 Create VSS signal proto definitions
    - Create `proto/vss/signals.proto` with DoorState, Location, VehicleSpeed, ParkingState messages
    - Create VehicleSignal oneof container for subscriptions
    - _Requirements: 2.5_
  
  - [x] 2.3 Create DataBroker service proto definition
    - Create `proto/services/databroker.proto` with GetSignal, SetSignal, Subscribe RPCs
    - Define request/response messages compatible with Eclipse Kuksa
    - _Requirements: 2.1_
  
  - [x] 2.4 Create UpdateService proto definition
    - Create `proto/services/update_service.proto` with InstallAdapter, UninstallAdapter, ListAdapters, WatchAdapterStates RPCs
    - Define AdapterState enum and AdapterInfo message
    - _Requirements: 2.2_
  
  - [x] 2.5 Create ParkingAdaptor service proto definition
    - Create `proto/services/parking_adaptor.proto` with StartSession, StopSession, GetSessionStatus RPCs
    - Define session request/response messages
    - _Requirements: 2.3_
  
  - [x] 2.6 Create LockingService proto definition
    - Create `proto/services/locking_service.proto` with Lock, Unlock, GetLockState RPCs
    - Define Door enum and lock command messages
    - _Requirements: 2.4_

- [x] 3. Checkpoint - Verify proto definitions
  - Ensure all proto files are syntactically valid using `protoc --lint`
  - Ask the user if questions arise

- [x] 4. Set up build system
  - [x] 4.1 Create root Makefile with all build targets
    - Define `all`, `proto`, `build`, `test`, `clean` targets
    - Define per-stack targets: `proto-rust`, `proto-kotlin`, `proto-dart`, `proto-go`
    - Define `build-rhivos`, `build-android`, `build-backend`, `build-containers`
    - Define `infra-up`, `infra-down` for local infrastructure
    - _Requirements: 4.1, 4.9_
  
  - [x] 4.2 Create Rust workspace configuration
    - Create `rhivos/Cargo.toml` workspace manifest
    - Create stub `Cargo.toml` for each service (locking-service, cloud-gateway-client, parking-operator-adaptor, update-service)
    - Create `rhivos/shared/Cargo.toml` for shared library
    - _Requirements: 4.2_
  
  - [x] 4.3 Create Go module configuration
    - Create `backend/go.mod` with module path
    - Create stub directories for parking-fee-service and cloud-gateway
    - _Requirements: 4.5_
  
  - [x] 4.4 Create Android project configuration
    - Create `android/parking-app/build.gradle.kts` with Kotlin and gRPC dependencies
    - Create `android/parking-app/settings.gradle.kts`
    - _Requirements: 4.3_
  
  - [x] 4.5 Create Flutter project configuration
    - Create `android/companion-app/pubspec.yaml` with gRPC and protobuf dependencies
    - _Requirements: 4.4_
  
  - [x] 4.6 Create proto generation scripts
    - Create `scripts/generate-proto.sh` for all language bindings
    - Configure buf.yaml for proto linting and generation
    - _Requirements: 2.7, 4.9_
  
  - [x] 4.7 Write property test for proto regeneration round-trip
    - **Property 1: Proto Regeneration Round-Trip**
    - Test that modifying proto files and regenerating produces compilable code
    - **Validates: Requirements 2.7**

- [x] 5. Set up container build configuration
  - [x] 5.1 Create Containerfiles for RHIVOS services using UBI10 base images
    - Create `containers/rhivos/Containerfile.locking-service` with UBI10-minimal final stage
    - Create `containers/rhivos/Containerfile.update-service` with UBI10-minimal final stage
    - Create `containers/rhivos/Containerfile.parking-operator-adaptor` with UBI10-minimal final stage
    - Create `containers/rhivos/Containerfile.cloud-gateway-client` with UBI10-minimal final stage
    - Include base image rationale comment in each Containerfile
    - Use multi-stage builds with Rust builder and UBI10 final stage
    - _Requirements: 4.6, 8.1, 8.3, 8.4, 8.5_
  
  - [x] 5.2 Create Containerfiles for backend services using UBI10 base images
    - Create `containers/backend/Containerfile.parking-fee-service` with UBI10-micro final stage
    - Create `containers/backend/Containerfile.cloud-gateway` with UBI10-micro final stage
    - Include base image rationale comment in each Containerfile
    - Use multi-stage builds with Go builder and UBI10 final stage
    - _Requirements: 4.6, 8.1, 8.3, 8.4, 8.5_
  
  - [x] 5.3 Create mock service Containerfile using UBI10 base image
    - Create `containers/mock/Containerfile.parking-operator` with UBI10-minimal final stage
    - Include base image rationale comment
    - _Requirements: 3.4, 8.1, 8.3, 8.4_
  
  - [x] 5.4 Create container manifest generation script
    - Create `scripts/generate-manifest.sh` that extracts image metadata
    - Include git commit hash, build timestamp, and labels in manifest
    - _Requirements: 4.7, 4.8_
  
  - [x] 5.5 Write property test for container image git tagging
    - **Property 3: Container Image Git Tagging**
    - Test that built images contain valid git metadata in tags
    - **Validates: Requirements 4.8**
  
  - [x] 5.6 Write property test for UBI10 base image compliance
    - **Property 5: UBI10 Base Image Compliance**
    - Parse all Containerfiles and verify final stage uses `registry.access.redhat.com/ubi10/*`
    - Reject any final stages using alpine, ubuntu, debian, or other non-UBI images
    - **Validates: Requirements 8.1, 8.2, 8.5**
  
  - [x] 5.7 Write property test for Containerfile documentation compliance
    - **Property 6: Containerfile Documentation Compliance**
    - Verify each Containerfile contains a comment block with base image rationale
    - **Validates: Requirements 8.4**

- [ ] 6. Checkpoint - Verify build system
  - Ensure `make proto` generates valid code stubs
  - Ensure container builds complete successfully
  - Ask the user if questions arise

- [ ] 7. Set up local development infrastructure
  - [ ] 7.1 Create Podman Compose configuration
    - Create `infra/compose/podman-compose.yml` with all services
    - Configure Eclipse Mosquitto MQTT broker service
    - Configure Eclipse Kuksa Databroker service from CentOS Automotive images
    - Add health checks for all services
    - _Requirements: 3.1, 3.2, 3.3, 3.6_
  
  - [ ] 7.2 Create service configuration files
    - Create `infra/config/mosquitto/mosquitto.conf` with listener and TLS settings
    - Create `infra/config/kuksa/config.json` with VSS signal definitions
    - _Requirements: 3.5_
  
  - [ ] 7.3 Write property test for health check configuration completeness
    - **Property 2: Health Check Configuration Completeness**
    - Test that all services in compose file have health check configurations
    - **Validates: Requirements 3.6**

- [ ] 8. Set up communication protocol configuration
  - [ ] 8.1 Create development TLS certificates
    - Create `scripts/generate-certs.sh` to generate self-signed CA and certificates
    - Generate CA certificate, server certificate, and client certificate
    - Place certificates in `infra/certs/` directory structure
    - _Requirements: 5.5_
  
  - [ ] 8.2 Create service endpoint configuration
    - Create `infra/config/endpoints.yaml` documenting all service ports and socket paths
    - Configure UDS paths for local RHIVOS communication
    - Configure TCP ports for cross-domain communication
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.7_
  
  - [ ] 8.3 Create development mode configuration
    - Add environment variable support for disabling TLS verification
    - Document development mode settings in configuration
    - _Requirements: 5.6_

- [ ] 9. Checkpoint - Verify local infrastructure
  - Ensure `make infra-up` starts all services
  - Ensure services respond on documented ports
  - Ask the user if questions arise

- [ ] 10. Create documentation
  - [ ] 10.1 Create root README with project overview
    - Write project description and architecture overview
    - Include quick-start instructions for cloning and building
    - Include communication diagram (Mermaid)
    - _Requirements: 6.1, 6.5, 1.8_
  
  - [ ] 10.2 Create per-directory README files
    - Create `rhivos/README.md` explaining Rust services structure
    - Create `android/README.md` explaining Android apps structure
    - Create `backend/README.md` explaining Go services structure
    - Create `proto/README.md` explaining proto organization
    - Create `infra/README.md` explaining local infrastructure
    - Create `containers/README.md` explaining Containerfile organization
    - _Requirements: 6.2_
  
  - [ ] 10.3 Create development environment setup guides
    - Create `docs/setup-rust.md` for Rust development environment
    - Create `docs/setup-android.md` for Android/Kotlin development
    - Create `docs/setup-flutter.md` for Flutter/Dart development
    - Create `docs/setup-go.md` for Go development
    - _Requirements: 6.3_
  
  - [ ] 10.4 Create infrastructure documentation
    - Create `docs/local-infrastructure.md` with detailed setup instructions
    - Document port assignments and service dependencies
    - _Requirements: 6.4_
  
  - [ ] 10.5 Write property test for documentation directory coverage
    - **Property 4: Documentation Directory Coverage**
    - Test that all major directories have README.md files
    - **Validates: Requirements 6.2**

- [ ] 11. Set up demo scenario support
  - [ ] 11.1 Create mock data generators
    - Create `scripts/mock-location.sh` for generating location signals
    - Create `scripts/mock-speed.sh` for generating speed signals
    - Create `scripts/mock-door.sh` for generating door sensor signals
    - _Requirements: 7.4_
  
  - [ ] 11.2 Create failure simulation configuration
    - Add configuration options to simulate registry unavailability
    - Add configuration options to simulate network failures
    - Document failure simulation in `docs/demo-scenarios.md`
    - _Requirements: 7.5_

- [ ] 12. Final checkpoint - Verify complete foundation
  - Ensure all tests pass
  - Verify all three demo scenarios can be configured
  - Ask the user if questions arise

## Notes

- All tasks are required for comprehensive testing from the start
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties
- All Containerfiles must use UBI10 base images per Requirement 8
- The foundation does not include actual service implementations - those are covered in separate specs
