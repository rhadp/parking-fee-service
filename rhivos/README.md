# RHIVOS Services

Rust services for Red Hat In-Vehicle Operating System (RHIVOS) with safety (ASIL-B) and QM partition support.

## Directory Structure

```
rhivos/
├── Cargo.toml                    # Workspace manifest
├── shared/                       # Shared library for common functionality
│   └── src/
│       ├── lib.rs               # Library entry point
│       ├── config.rs            # Configuration management
│       ├── error.rs             # Error handling utilities
│       └── proto/               # Generated protobuf bindings
├── locking-service/             # ASIL-B door locking service (safety partition)
├── cloud-gateway-client/        # MQTT client for cloud communication (safety partition)
├── parking-operator-adaptor/    # Dynamic parking adapter (QM partition)
├── update-service/              # Container lifecycle management (QM partition)
└── data-broker/                 # Kuksa integration wrapper
```

## Services

| Service | Partition | Description |
|---------|-----------|-------------|
| `locking-service` | Safety (ASIL-B) | Door lock/unlock command execution |
| `cloud-gateway-client` | Safety (ASIL-B) | MQTT client for vehicle-to-cloud communication |
| `parking-operator-adaptor` | QM | Dynamic parking operator integration |
| `update-service` | QM | Container lifecycle and adapter management |

## Building

```bash
# Build all services
make build-rhivos

# Build individual service
cd rhivos && cargo build -p locking-service --release

# Run tests
cd rhivos && cargo test
```

## Dependencies

- Rust 1.75+ with cargo
- Protocol Buffer compiler (protoc)
- tonic for gRPC support
- tokio for async runtime

## Communication

Services communicate via:
- **Unix Domain Sockets (UDS)**: Local inter-process communication within RHIVOS
- **gRPC/TLS**: Cross-domain communication with AAOS

See `infra/config/endpoints.yaml` for socket paths and port assignments.
