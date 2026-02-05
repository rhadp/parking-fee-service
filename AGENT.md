# Product Overview

Parking Fee Service - a Software-Defined Vehicle (SDV) demo system showcasing mixed-criticality architecture.

## Overview

Demonstrates automatic parking fee payment through vehicle-to-cloud integration:

- Android IVI parking app communicates with ASIL-B door locking service on RHIVOS
- Dynamic parking operator adapters download on-demand based on vehicle location
- Cross-domain communication between QM-level Android app and safety-relevant locking system

### Core User Flow

1. Vehicle parks → location detected → appropriate parking adapter downloads
2. User locks vehicle → parking session starts automatically
3. User unlocks vehicle → session ends, payment processed
4. Unused adapters offload after 24 hours

### Key Components

- **PARKING_APP**: Android Automotive OS IVI application
- **LOCKING_SERVICE**: ASIL-B door lock service (RHIVOS safety partition)
- **DATA_BROKER**: Eclipse Kuksa VSS-compliant signal broker
- **PARKING_OPERATOR_ADAPTOR**: Dynamic containerized adapters (RHIVOS QM partition)
- **UPDATE_SERVICE**: Container lifecycle management
- **CLOUD_GATEWAY**: Vehicle-to-cloud MQTT connectivity
- **COMPANION_APP**: Mobile app for remote lock/unlock

### Demo Scope

- Mock payment processing (no real transactions)
- Simulated location/sensors
- Pre-signed adapters (simplified trust chain)
- Minimal UI for Android apps

## Spec-Driven Development

This project uses "Spec-Driven Development" as its primary means to create a working implementation.

The guiding documents that provide the requirements, design and list of tasks to be done live in `.kiro/specs/{feature-name}/` with:

- `requirements.md` - User stories and acceptance criteria
- `design.md` - Architecture and interface definitions
- `tasks.md` - Implementation task list

A guideline on how to write specifications is in @.kiro/steering/requirements-engineering.md

### Architecture Decision Records

An architecture decision record (ADR) is a document that captures an important architecture decision made
along with its context and consequences. ADRs live in `docs/adr/{decision.md}`.

### Other Documentation

Other misc. documentation markdown files live in `docs/{topic.md}`

## Development Workflow

This project uses a typical "Git-flow" during the development. This is defined in @.kiro/steering/git-flow.md.

When implementing a task, always update the `.kiro/specs/{feature-name}/task.md` document, accoring to the conventions defined in @.kiro/steering/requirements-engineering.md

### Workflow Per Task

1. Create feature branch from `develop`: `git checkout -b feature/<task-name> develop`
2. Implement changes
3. Stage and commit with descriptive message: `git add . && git commit -m "<type>: <description>"`
4. Push and create PR via GitHub MCP server
5. Always merge the changes back to `develop` before starting next task

## Project Structure

```
parking-fee-service/
├── rhivos/                         # Rust services (RHIVOS)
│   ├── locking-service/            # ASIL-B door locking (safety partition)
│   ├── data-broker/                # Kuksa integration wrapper
│   ├── cloud-gateway-client/       # MQTT client (safety partition)
│   ├── parking-operator-adaptor/   # Dynamic adapter (QM partition)
│   ├── update-service/             # Container lifecycle (QM partition)
│   └── shared/                     # Shared Rust libraries
├── android/
│   ├── parking-app/                # Kotlin AAOS application
│   └── companion-app/              # Flutter/Dart mobile app
├── backend/
│   ├── parking-fee-service/        # Go parking operations service
│   └── cloud-gateway/              # Go MQTT broker/router
├── proto/                          # Shared Protocol Buffer definitions
│   ├── vss/                        # VSS signal definitions
│   ├── services/                   # Service interface definitions
│   └── common/                     # Shared message types
├── containers/                     # Containerfiles
│   ├── rhivos/                     # RHIVOS service containers
│   ├── backend/                    # Backend service containers
│   └── mock/                       # Mock service containers
├── infra/                          # Local development infrastructure
│   ├── compose/                    # Podman compose files
│   ├── certs/                      # Development TLS certificates
│   └── config/                     # Service configurations
├── scripts/                        # Build and utility scripts
├── docs/                           # Documentation
└── Makefile                        # Root build orchestration
```

## Technology Stack

### Languages by Component

| Component | Language | Location |
|-----------|----------|----------|
| RHIVOS services | Rust | `rhivos/` |
| Android IVI app | Kotlin | `android/parking-app/` |
| Companion app | Flutter/Dart | `android/companion-app/` |
| Backend services | Go | `backend/` |

### Key Dependencies

- **Eclipse Kuksa Databroker**: VSS-compliant vehicle signal broker
- **Eclipse Mosquitto**: MQTT broker
- **Protocol Buffers**: Interface definitions (`proto/`)
- **Podman**: Container builds and local orchestration
- **Container Images**: Red Hat UBI10 based container images, or Centos AutoSD based

### External Dependencies

- **Google Artifact Registry**: OCI-compliant registry stores validated PARKING_OPERATOR_ADAPTOR images
