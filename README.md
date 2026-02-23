# SDV Parking Demo System

A mixed-criticality Software-Defined Vehicle (SDV) demo showcasing automatic
parking fee payment across Android Automotive OS (AAOS), Red Hat In-Vehicle OS
(RHIVOS), and cloud backend services.

The system demonstrates dynamic, on-demand provisioning of parking operator
adapters that download to the vehicle based on location, start/stop parking
sessions autonomously via door lock/unlock events, and offload when no longer
needed.

## Prerequisites

Install the following tools before building or running the project:

| Tool | Version | Purpose |
|------|---------|---------|
| [Rust](https://rustup.rs) | 1.75+ | RHIVOS service components |
| [Go](https://go.dev/dl/) | 1.22+ | Backend services and mock CLI apps |
| [protoc](https://grpc.io/docs/protoc-installation/) | 3.x | Protocol Buffer compiler |
| [protoc-gen-go](https://pkg.go.dev/google.golang.org/protobuf/cmd/protoc-gen-go) | 1.34+ | Go protobuf code generation |
| [protoc-gen-go-grpc](https://pkg.go.dev/google.golang.org/grpc/cmd/protoc-gen-go-grpc) | 1.4+ | Go gRPC code generation |
| [Podman](https://podman.io) or [Docker](https://docs.docker.com/get-docker/) | 4.x+ | Container runtime for local infrastructure |
| GNU Make | 3.81+ | Build orchestration |

## Quick Start

```bash
# Build all components (Rust + Go)
make build

# Run all unit tests
make test

# Run linters (cargo clippy + go vet)
make lint

# Run build + test + lint in sequence
make check

# Regenerate Go code from .proto definitions
make proto

# Start local infrastructure (Mosquitto MQTT broker + Kuksa Databroker)
make infra-up

# Stop local infrastructure
make infra-down

# Remove all build artifacts
make clean
```

## Repository Structure

```
parking-fee-service/
├── proto/                          # Shared .proto definitions (single source of truth)
│   ├── common.proto                #   Shared types: AdapterState, AdapterInfo, ErrorDetails
│   ├── update_service.proto        #   UPDATE_SERVICE gRPC interface
│   └── parking_adaptor.proto       #   PARKING_OPERATOR_ADAPTOR gRPC interface
│
├── gen/go/                         # Generated Go protobuf code
│   ├── commonpb/                   #   Common types package
│   ├── updateservicepb/            #   Update service package
│   └── parkingadaptorpb/           #   Parking adaptor package
│
├── rhivos/                         # Rust workspace — RHIVOS service skeletons
│   ├── locking-service/            #   ASIL-B door locking service
│   ├── cloud-gateway-client/       #   Cloud connectivity client
│   ├── update-service/             #   Adapter lifecycle management (gRPC)
│   └── parking-operator-adaptor/   #   Dynamic parking operator adapter (gRPC)
│
├── backend/                        # Go backend services
│   ├── parking-fee-service/        #   Operator discovery + fee calculation (HTTP :8080)
│   └── cloud-gateway/              #   Vehicle command relay (HTTP :8081, MQTT)
│
├── mock/                           # Mock CLI apps (simulate Android apps)
│   ├── parking-app-cli/            #   Mock PARKING_APP (9 subcommands)
│   └── companion-app-cli/          #   Mock COMPANION_APP (3 subcommands)
│
├── aaos/                           # AAOS PARKING_APP placeholder (Kotlin, future)
├── android/                        # COMPANION_APP placeholder (Flutter, future)
│
├── infra/                          # Local development infrastructure
│   ├── docker-compose.yml          #   Mosquitto (:1883) + Kuksa Databroker (:55556)
│   └── mosquitto/mosquitto.conf    #   MQTT broker configuration
│
├── tests/
│   ├── setup/                      # Spec verification tests (standalone Go module)
│   └── integration/                # Cross-component integration tests (future)
│
├── Makefile                        # Top-level build orchestration
├── go.work                         # Go workspace linking all Go modules
└── .specs/                         # Specification documents
    └── prd.md                      # Product Requirements Document
```

## Local Development Ports

| Service | Protocol | Port |
|---------|----------|------|
| Eclipse Mosquitto | MQTT | 1883 |
| Eclipse Kuksa Databroker | gRPC | 55556 |
| PARKING_FEE_SERVICE | HTTP | 8080 |
| CLOUD_GATEWAY | HTTP | 8081 |
| UPDATE_SERVICE | gRPC | 50051 |
| PARKING_OPERATOR_ADAPTOR | gRPC | 50052 |

## Architecture

The system spans multiple domains:

- **RHIVOS Safety Partition (ASIL-B):** Door locking service, DATA_BROKER
  (vehicle signals), cloud gateway client
- **RHIVOS QM Partition:** Dynamic parking operator adapters (OCI containers),
  update service for adapter lifecycle management
- **Android IVI:** PARKING_APP user interface for operator discovery and session
  management
- **Cloud Backend:** Parking fee service (operator discovery), cloud gateway
  (vehicle command relay via MQTT)

Cross-partition communication uses gRPC over TCP. Same-partition services
communicate via Unix Domain Sockets. Vehicle signal state is brokered through
the Eclipse Kuksa Databroker using the Vehicle Signal Specification (VSS).

For full product requirements, see [`.specs/prd.md`](.specs/prd.md).

## License

See [LICENSE](LICENSE) for details.
