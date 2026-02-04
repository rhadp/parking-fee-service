# Requirements Document

## Introduction

This document defines the requirements for the foundational infrastructure of the Software-Defined Vehicle (SDV) Parking Demo System. The foundation establishes the project structure, shared interface definitions, local development infrastructure, build system, and communication protocols that all subsequent components will build upon.

The system demonstrates mixed-criticality communication between an Android parking app and ASIL-B door locking services on RHIVOS, with dynamic parking operator adapters that download on-demand based on vehicle location.

## Glossary

- **Monorepo**: A single repository containing multiple related projects with shared dependencies and tooling
- **Protocol_Buffers**: Google's language-neutral, platform-neutral serialization format for defining service interfaces
- **gRPC**: A high-performance RPC framework using Protocol Buffers for service definitions
- **VSS**: Vehicle Signal Specification - a standardized taxonomy for vehicle data from COVESA
- **DATA_BROKER**: Eclipse Kuksa Databroker providing VSS-compliant gRPC pub/sub interface for vehicle signals
- **RHIVOS**: Red Hat In-Vehicle Operating System with safety (ASIL-B) and QM partitions
- **AAOS**: Android Automotive Operating System
- **UDS**: Unix Domain Sockets for local inter-process communication
- **TLS**: Transport Layer Security for encrypted network communication
- **MQTT**: Message Queuing Telemetry Transport protocol for vehicle-to-cloud communication
- **OCI**: Open Container Initiative standard for container images
- **Container_Manifest**: Metadata file describing container image contents and configuration for validation

## Requirements

### Requirement 1: Monorepo Project Structure

**User Story:** As a developer in an engineering team, I want a well-organized monorepo structure, so that I can navigate between different technology stacks and understand component relationships easily.

#### Acceptance Criteria

1. THE Project_Structure SHALL contain a `rhivos/` directory for all Rust services (safety and QM partition components)
2. THE Project_Structure SHALL contain an `android/parking-app/` directory for the Kotlin AAOS application
3. THE Project_Structure SHALL contain an `android/companion-app/` directory for the Flutter/Dart mobile application
4. THE Project_Structure SHALL contain a `backend/` directory for Golang services
5. THE Project_Structure SHALL contain a `proto/` directory for shared Protocol Buffer definitions
6. THE Project_Structure SHALL contain a `containers/` directory for container build files (Containerfiles)
7. THE Project_Structure SHALL contain an `infra/` directory for local development infrastructure configuration
8. WHEN a developer clones the repository THEN the Project_Structure SHALL include a README with setup instructions for each technology stack

### Requirement 2: Protocol Buffer Service Definitions

**User Story:** As a developer, I want shared Protocol Buffer definitions for all inter-component communication, so that I can ensure type-safe communication across different programming languages.

#### Acceptance Criteria

1. THE Proto_Definitions SHALL define a DATA_BROKER service interface compatible with Eclipse Kuksa VSS signals
2. THE Proto_Definitions SHALL define an UPDATE_SERVICE interface for adapter lifecycle management including install, uninstall, and state watching operations
3. THE Proto_Definitions SHALL define a PARKING_OPERATOR_ADAPTOR interface for parking session management including start and stop operations
4. THE Proto_Definitions SHALL define a CLOUD_GATEWAY_CLIENT to LOCKING_SERVICE interface for lock/unlock command forwarding
5. THE Proto_Definitions SHALL define VSS signal message types for:
   - Vehicle.Cabin.Door.Row1.DriverSide.IsLocked (boolean)
   - Vehicle.Cabin.Door.Row1.DriverSide.IsOpen (boolean)
   - Vehicle.CurrentLocation.Latitude (double)
   - Vehicle.CurrentLocation.Longitude (double)
   - Vehicle.Speed (float)
   - Vehicle.Parking.SessionActive (boolean, custom signal)
6. THE Proto_Definitions SHALL use standard gRPC error codes with custom ErrorDetails messages for domain-specific errors
7. WHEN Proto_Definitions are modified THEN the Build_System SHALL regenerate language-specific bindings for Rust, Kotlin, Dart, and Golang

### Requirement 3: Local Development Infrastructure

**User Story:** As a developer, I want a containerized local development environment, so that I can develop and test components without cloud dependencies.

#### Acceptance Criteria

1. THE Local_Infrastructure SHALL provide a Podman Compose configuration for orchestrating local services
2. THE Local_Infrastructure SHALL include a containerized Eclipse Mosquitto MQTT broker for simulating vehicle-to-cloud communication
3. THE Local_Infrastructure SHALL integrate with the Eclipse Kuksa Databroker container from the official CentOS Automotive container images
4. THE Local_Infrastructure SHALL provide mock service containers for testing component interactions
5. WHEN a developer runs the local infrastructure THEN all services SHALL be accessible via localhost with documented ports
6. THE Local_Infrastructure SHALL include health check configurations for all containerized services
7. IF a required container fails to start THEN the Local_Infrastructure SHALL provide clear error messages indicating the failure reason

### Requirement 4: Build System

**User Story:** As a developer, I want unified build scripts for each technology stack, so that I can build components consistently across different environments.

#### Acceptance Criteria

1. THE Build_System SHALL provide a root Makefile with targets for building all components
2. THE Build_System SHALL provide Rust build configuration using Cargo for RHIVOS services
3. THE Build_System SHALL provide Gradle build configuration for the Kotlin AAOS application
4. THE Build_System SHALL provide Flutter build configuration for the Dart companion application
5. THE Build_System SHALL provide Go modules configuration for backend services
6. THE Build_System SHALL provide container build targets that generate OCI-compliant images
7. THE Build_System SHALL generate container manifests during the build process for validation purposes
8. WHEN building containers THEN the Build_System SHALL tag images with version information derived from git metadata
9. THE Build_System SHALL provide a `make proto` target that regenerates all language bindings from Protocol Buffer definitions

### Requirement 5: Communication Protocol Configuration

**User Story:** As a developer, I want pre-configured communication protocols for each component interaction, so that I can focus on business logic rather than connection setup.

#### Acceptance Criteria

1. THE Communication_Config SHALL configure gRPC over Unix Domain Sockets for local inter-process communication within RHIVOS
2. THE Communication_Config SHALL configure gRPC over TLS for cross-domain communication between AAOS and RHIVOS
3. THE Communication_Config SHALL configure MQTT over TLS for vehicle-to-cloud communication via CLOUD_GATEWAY
4. THE Communication_Config SHALL configure HTTPS/REST for PARKING_APP to PARKING_FEE_SERVICE communication
5. THE Communication_Config SHALL provide example TLS certificates for local development (self-signed)
6. WHEN running in local development mode THEN the Communication_Config SHALL allow disabling TLS verification for testing
7. THE Communication_Config SHALL document the port assignments for each service endpoint

### Requirement 6: Development Documentation

**User Story:** As a developer, I want comprehensive setup documentation, so that I can quickly onboard and start contributing to the project.

#### Acceptance Criteria

1. THE Documentation SHALL include a root README with project overview and quick-start instructions
2. THE Documentation SHALL include per-directory README files explaining the purpose and structure of each component
3. THE Documentation SHALL include instructions for setting up each development environment (Rust, Kotlin, Flutter, Go)
4. THE Documentation SHALL include instructions for running the local infrastructure
5. THE Documentation SHALL include a communication diagram showing all component interactions and protocols
6. WHEN a new developer follows the setup instructions THEN they SHALL be able to build all components within 30 minutes (excluding dependency downloads)

### Requirement 7: Demo Scenario Support

**User Story:** As a demo presenter, I want the foundation to support all three demo scenarios, so that I can demonstrate the complete system capabilities.

#### Acceptance Criteria

1. THE Foundation SHALL support the Happy Path scenario where an adapter downloads and manages a parking session
2. THE Foundation SHALL support the Adapter Already Installed scenario where no download is needed
3. THE Foundation SHALL support the Error Handling scenario with simulated registry unavailability
4. THE Foundation SHALL provide mock data generators for location, speed, and door sensor signals
5. WHEN demonstrating error scenarios THEN the Foundation SHALL provide configuration options to simulate failures
