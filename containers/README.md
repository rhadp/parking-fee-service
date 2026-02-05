# Container Build Files

Containerfiles for building OCI-compliant container images for the SDV Parking Demo System.

## Directory Structure

```
containers/
├── rhivos/                              # RHIVOS service containers
│   ├── Containerfile.locking-service
│   ├── Containerfile.update-service
│   ├── Containerfile.parking-operator-adaptor
│   └── Containerfile.cloud-gateway-client
├── backend/                             # Backend service containers
│   ├── Containerfile.parking-fee-service
│   └── Containerfile.cloud-gateway
└── mock/                                # Mock service containers
    ├── Containerfile.parking-operator
    └── parking-operator/               # Mock service source
        └── app.py
```

## Base Image Standards

All containers use Red Hat Universal Base Image 10 (UBI10) for enterprise support and security compliance.

### UBI10 Variants

| Variant | Use Case | Size |
|---------|----------|------|
| `ubi10/ubi` | General-purpose with full tooling | ~200MB |
| `ubi10/ubi-minimal` | Size-optimized with microdnf | ~100MB |
| `ubi10/ubi-micro` | Minimal footprint, no package manager | ~30MB |

### Build Stage Requirements

- **Rust/Go builds**: Must use `ghcr.io/rhadp/builder`
- **Final stage**: Must use `registry.access.redhat.com/ubi10/*`

## Building Containers

```bash
# Build all containers
make build-containers

# Build specific container
podman build -f containers/rhivos/Containerfile.locking-service -t locking-service:latest .

# Build with git metadata
./scripts/generate-manifest.sh
```

## Container Images

### RHIVOS Services

| Image | Base | Description |
|-------|------|-------------|
| locking-service | ubi10-minimal | ASIL-B door locking |
| update-service | ubi10-minimal | Adapter lifecycle |
| parking-operator-adaptor | ubi10-minimal | Parking integration |
| cloud-gateway-client | ubi10-minimal | MQTT client |

### Backend Services

| Image | Base | Description |
|-------|------|-------------|
| parking-fee-service | ubi10-micro | Parking operations API |
| cloud-gateway | ubi10-micro | MQTT broker/router |

### Mock Services

| Image | Base | Description |
|-------|------|-------------|
| mock-parking-operator | ubi10-minimal | Test parking API |

## Image Tagging

Images are tagged with git metadata:
- `latest` - Most recent build
- `v1.0.0` - Release version
- `sha-abc1234` - Git commit hash

## Containerfile Requirements

Each Containerfile must:
1. Use UBI10 variant as final stage base
2. Include comment documenting base image rationale
3. Use multi-stage builds for compiled languages
4. Run as non-root user
5. Include appropriate labels (maintainer, version, description)

## Prohibited Base Images

The following are NOT permitted in final stages:
- alpine, ubuntu, debian, centos, fedora
- Any `*-slim` variants

These may be used in build stages only.
