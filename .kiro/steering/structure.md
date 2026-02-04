# Project Structure

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

## Partition Mapping

| Directory | Runtime Environment |
|-----------|---------------------|
| `rhivos/locking-service/` | RHIVOS Safety Partition (ASIL-B) |
| `rhivos/cloud-gateway-client/` | RHIVOS Safety Partition (ASIL-B) |
| `rhivos/data-broker/` | RHIVOS Safety Partition (ASIL-B) |
| `rhivos/parking-operator-adaptor/` | RHIVOS QM Partition |
| `rhivos/update-service/` | RHIVOS QM Partition |
| `android/parking-app/` | Android Automotive OS (QM) |
| `android/companion-app/` | Android/iOS mobile |
| `backend/*` | Cloud (OpenShift) |

## Specs Location

Feature specifications live in `.kiro/specs/{feature-name}/` with:
- `requirements.md` - User stories and acceptance criteria
- `design.md` - Architecture and interface definitions
- `tasks.md` - Implementation task list
