# SDV Parking Demo System

A Software-Defined Vehicle (SDV) demonstration showcasing mixed-criticality communication between Android Automotive OS applications and ASIL-B safety services running on Red Hat In-Vehicle OS (RHIVOS).

## Overview

This project demonstrates automatic parking fee payment where the parking session starts when the vehicle locks and stops when it unlocks. The architecture spans multiple domains:

- **Android IVI (QM)**: In-car user interface for the parking app
- **RHIVOS Safety Partition (ASIL-B)**: Door locking service and vehicle signals
- **RHIVOS QM Partition**: Dynamic parking operator adapters (containers)
- **Cloud Backend**: Parking fee service and adapter registry

The system uses containerized adapters that download on-demand based on vehicle location, demonstrating a "feature-on-demand" pattern for post-production service enablement.

## Architecture

```mermaid
flowchart TB
    subgraph AAOS["Android Automotive OS"]
        PA[PARKING_APP]
    end

    subgraph RHIVOS_Safety["RHIVOS Safety Partition"]
        LS[LOCKING_SERVICE]
        DB[DATA_BROKER<br/>Eclipse Kuksa]
        CGC[CLOUD_GATEWAY_CLIENT]
    end

    subgraph RHIVOS_QM["RHIVOS QM Partition"]
        POA[PARKING_OPERATOR_ADAPTOR]
        US[UPDATE_SERVICE]
    end

    subgraph Cloud["Backend Services"]
        PFS[PARKING_FEE_SERVICE]
        CG[CLOUD_GATEWAY]
        REG[REGISTRY]
    end

    PA -->|"gRPC/TLS"| DB
    PA -->|"gRPC/TLS"| US
    PA -->|"gRPC/TLS"| POA
    PA -->|"HTTPS/REST"| PFS

    LS -->|"gRPC/UDS"| DB
    CGC -->|"gRPC/UDS"| LS
    CGC -->|"gRPC/UDS"| DB
    CGC -->|"MQTT/TLS"| CG

    POA -->|"gRPC/UDS"| DB
    US -->|"HTTPS/OCI"| REG
```

## Project Structure

```
parking-fee-service/
├── Makefile                        # Root build orchestrator
├── rhivos/                         # Rust services for RHIVOS
│   ├── Cargo.toml                  # Workspace manifest
│   ├── parking-proto/              # Shared proto bindings crate
│   ├── locking-service/            # ASIL-B door locking (safety partition)
│   ├── cloud-gateway-client/       # MQTT client (safety partition)
│   ├── parking-operator-adaptor/   # Dynamic adapter (QM partition)
│   └── update-service/             # Container lifecycle (QM partition)
├── backend/
│   ├── parking-fee-service/        # Go REST service for parking operations
│   └── cloud-gateway/              # Go REST gateway for vehicle-to-cloud
├── mock/
│   ├── parking-app-cli/            # Go CLI simulating Android parking app
│   ├── companion-app-cli/          # Go CLI simulating companion mobile app
│   └── sensors/                    # Rust CLI for mock VSS sensor data
├── android/
│   ├── parking-app/                # AAOS application (placeholder)
│   └── companion-app/              # Mobile companion app (placeholder)
├── proto/                          # Shared Protocol Buffer definitions
│   ├── common/                     # Shared message types
│   ├── services/                   # Service interface definitions
│   └── gen/go/                     # Generated Go proto packages
├── containers/                     # Containerfiles for OCI images
│   ├── rhivos/                     # Rust service containers
│   ├── backend/                    # Go service containers
│   └── mock/                       # Mock tool containers
├── infra/                          # Local development infrastructure
│   ├── compose.yaml                # Podman/Docker Compose for Kuksa + Mosquitto
│   └── config/                     # Service configuration files
├── scripts/                        # Build and utility scripts
├── docs/                           # Documentation
└── tests/                          # Verification test scripts
```

## Quick Start

### Prerequisites

- **Rust** 1.75+ with cargo
- **Go** 1.22+
- **Protocol Buffers** compiler (`protoc` 3.x+)
- **protoc-gen-go** and **protoc-gen-go-grpc** (Go proto plugins)
- **Podman** (preferred) or Docker for containers and local infrastructure

Verify all tools are installed:

```bash
./scripts/check-tools.sh
```

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/rhadp/parking-fee-service.git
cd parking-fee-service

# Generate Protocol Buffer Go bindings
make proto

# Build all components (Rust + Go)
make build

# Run all tests
make test

# Run linters (clippy for Rust, go vet for Go)
make lint
```

### Start Local Infrastructure

Local development uses Eclipse Kuksa Databroker (VSS signal broker) and Eclipse Mosquitto (MQTT broker):

```bash
# Start local development services
make infra-up

# Verify services are running
make infra-status

# Stop infrastructure when done
make infra-down
```

| Service | Port | Protocol |
|---------|------|----------|
| Kuksa Databroker | 55555 | gRPC |
| Mosquitto | 1883 | MQTT |

### Build Container Images

```bash
# Build all container images
make build-containers
```

### Clean Build Artifacts

```bash
# Remove all build artifacts (Rust, Go, generated proto files)
make clean
```

## Makefile Targets

| Target | Description |
|--------|-------------|
| `make build` | Build all Rust and Go components |
| `make test` | Run all unit tests across Rust and Go |
| `make lint` | Run linters (clippy for Rust, go vet for Go) |
| `make proto` | Generate Go proto bindings from `.proto` files |
| `make clean` | Remove all build artifacts |
| `make infra-up` | Start local Kuksa Databroker and Mosquitto |
| `make infra-down` | Stop and remove infrastructure containers |
| `make infra-status` | Show infrastructure container status |
| `make build-containers` | Build OCI container images for all services |
| `make check-tools` | Verify all required development tools are installed |

## Service Ports (Local Development)

| Service | Port | Protocol |
|---------|------|----------|
| locking-service | — | Kuksa client (no listening port) |
| cloud-gateway-client | — | Kuksa/MQTT client (no listening port) |
| update-service | 50053 | gRPC |
| parking-operator-adaptor | 50054 | gRPC |
| parking-fee-service | 8080 | HTTP/REST |
| cloud-gateway | 8081 | HTTP/REST |
| Kuksa Databroker (infra) | 55555 | gRPC |
| Mosquitto (infra) | 1883 | MQTT |

## Mock CLI Tools

Mock CLI applications are provided for integration testing without real Android builds.

### parking-app-cli

Simulates the Android parking app, calling gRPC services:

```bash
parking-app-cli [flags] <command>

Commands:
  install-adapter   Call UpdateService.InstallAdapter
  list-adapters     Call UpdateService.ListAdapters
  remove-adapter    Call UpdateService.RemoveAdapter
  adapter-status    Call UpdateService.GetAdapterStatus
  watch-adapters    Call UpdateService.WatchAdapterStates (streaming)
  start-session     Call ParkingAdapter.StartSession
  stop-session      Call ParkingAdapter.StopSession
  get-status        Call ParkingAdapter.GetStatus
  get-rate          Call ParkingAdapter.GetRate

Flags:
  --update-service-addr   Address of UpdateService (default: localhost:50053)
  --adapter-addr          Address of ParkingAdapter (default: localhost:50054)
```

### companion-app-cli

Simulates the companion mobile app, calling REST endpoints:

```bash
companion-app-cli [flags] <command>

Commands:
  lock     POST /api/v1/vehicles/{vin}/lock
  unlock   POST /api/v1/vehicles/{vin}/unlock
  status   GET  /api/v1/vehicles/{vin}/status

Flags:
  --gateway-addr   Address of CloudGateway (default: http://localhost:8081)
  --vin            Vehicle VIN (required)
  --token          Bearer token (default: demo-token)
```

### mock-sensors

Publishes mock VSS sensor data to Kuksa Databroker:

```bash
mock-sensors [flags] <command>

Commands:
  set-location <lat> <lon>     Set Vehicle.CurrentLocation.{Latitude,Longitude}
  set-speed <km/h>             Set Vehicle.Speed
  set-door <open|closed>       Set Vehicle.Cabin.Door.Row1.DriverSide.IsOpen
  lock-command <lock|unlock>   Set Vehicle.Command.Door.Lock

Flags:
  --databroker-addr   Address of Kuksa Databroker (default: http://localhost:55555)
```

## Communication Protocols

| Source | Target | Protocol | Description |
|--------|--------|----------|-------------|
| PARKING_APP | DATA_BROKER | gRPC/TLS | Read vehicle signals |
| PARKING_APP | UPDATE_SERVICE | gRPC/TLS | Adapter lifecycle |
| PARKING_APP | PARKING_FEE_SERVICE | HTTPS/REST | Parking operations |
| LOCKING_SERVICE | DATA_BROKER | gRPC/UDS | Write lock events |
| CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY | MQTT/TLS | Vehicle-to-cloud |
| UPDATE_SERVICE | REGISTRY | HTTPS/OCI | Pull adapters |

## Proto Definitions

The `proto/` directory contains shared Protocol Buffer definitions:

- **`common/common.proto`** — Shared message types: `Location`, `VehicleId`, `AdapterInfo`, `AdapterState`, `ErrorDetails`
- **`services/update_service.proto`** — UpdateService gRPC interface (adapter lifecycle)
- **`services/parking_adapter.proto`** — ParkingAdapter gRPC interface (parking sessions)

Generated Go bindings are committed under `proto/gen/go/`. Rust bindings are generated at build time via `tonic-build` in `rhivos/parking-proto/`.

## Current Status

| Service | Status | Description |
|---------|--------|-------------|
| locking-service | **Implemented** | Safety-validated lock/unlock via Kuksa Databroker |
| mock-sensors | **Implemented** | CLI tool for publishing mock VSS signals |
| cloud-gateway-client | Skeleton | Logs startup, waits for shutdown |
| update-service | Skeleton | Returns `UNIMPLEMENTED` for all RPCs |
| parking-operator-adaptor | Skeleton | Returns `UNIMPLEMENTED` for all RPCs |
| parking-fee-service | Skeleton | Returns HTTP 501 |
| cloud-gateway | Skeleton | Returns HTTP 501 |

The **locking-service** subscribes to `Vehicle.Command.Door.Lock` via Kuksa Databroker, validates commands against vehicle speed and door state, and writes `IsLocked` and `LockResult` signals. See [docs/vss-signals.md](docs/vss-signals.md) for custom VSS signal definitions.

## License

See [LICENSE](LICENSE) for details.
