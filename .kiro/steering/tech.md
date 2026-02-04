# Technology Stack

## Languages by Component

| Component | Language | Location |
|-----------|----------|----------|
| RHIVOS services | Rust | `rhivos/` |
| Android IVI app | Kotlin | `android/parking-app/` |
| Companion app | Flutter/Dart | `android/companion-app/` |
| Backend services | Go | `backend/` |

## Communication Protocols

- **gRPC over UDS**: Local IPC within RHIVOS partitions
- **gRPC over TLS**: Cross-domain (AAOS ↔ RHIVOS)
- **MQTT over TLS**: Vehicle-to-cloud communication
- **HTTPS/REST**: PARKING_APP ↔ PARKING_FEE_SERVICE
- **HTTPS/OCI**: Container registry pulls

## Key Dependencies

- **Eclipse Kuksa Databroker**: VSS-compliant vehicle signal broker
- **Eclipse Mosquitto**: MQTT broker
- **Protocol Buffers**: Interface definitions (`proto/`)
- **Podman**: Container builds and local orchestration

## Build Commands

```bash
# Generate all proto bindings
make proto

# Build all components
make build

# Build specific stacks
make build-rhivos      # Rust services
make build-android     # Android apps
make build-backend     # Go services
make build-containers  # OCI images

# Run tests
make test

# Local infrastructure
make infra-up          # Start local services
make infra-down        # Stop local services

# Clean build artifacts
make clean
```

## Container Registry

OCI-compliant registry (Google Artifact Registry) stores validated PARKING_OPERATOR_ADAPTOR images.

## VSS Signals (COVESA v5.1)

- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen`
- `Vehicle.CurrentLocation.Latitude`
- `Vehicle.CurrentLocation.Longitude`
- `Vehicle.Speed`
- `Vehicle.Parking.SessionActive` (custom)
