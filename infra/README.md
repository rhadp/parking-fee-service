# Infrastructure

Local development infrastructure configuration for the SDV Parking Demo System.

## Directory Structure

```
infra/
├── compose/                  # Container orchestration
│   └── podman-compose.yml   # Podman Compose configuration
├── certs/                   # TLS certificates
│   ├── ca/                  # Certificate Authority
│   │   ├── ca.crt          # CA certificate
│   │   └── ca.key          # CA private key (dev only)
│   ├── server/             # Server certificates
│   │   ├── server.crt
│   │   └── server.key
│   └── client/             # Client certificates
│       ├── client.crt
│       └── client.key
└── config/                  # Service configurations
    ├── development.yaml     # Development environment settings
    ├── endpoints.yaml       # Service endpoint definitions
    ├── kuksa/              # Kuksa Databroker config
    │   └── config.json
    └── mosquitto/          # MQTT broker config
        └── mosquitto.conf
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| Mosquitto MQTT | 1883, 8883 | MQTT broker (plain/TLS) |
| Kuksa Databroker | 55556 | VSS signal pub/sub |
| Mock Parking Operator | 8080 | Test parking API |

## Quick Start

```bash
# Start all services
make infra-up

# Stop all services
make infra-down

# View logs
cd infra/compose && podman-compose logs -f
```

## TLS Certificates

Development certificates are self-signed. Generate new ones:

```bash
./scripts/generate-certs.sh
```

**Warning:** Never use development certificates in production.

## Configuration Files

### endpoints.yaml

Defines all service endpoints:
- Unix Domain Socket paths for RHIVOS local communication
- TCP ports for cross-domain communication
- TLS settings per endpoint

### development.yaml

Development-specific settings:
- TLS verification disabled
- Debug logging enabled
- Mock service endpoints

### mosquitto.conf

MQTT broker configuration:
- Listener ports
- TLS settings
- Authentication (disabled for dev)

### kuksa/config.json

Kuksa Databroker configuration:
- VSS signal definitions
- Access control rules

## Health Checks

All services include health checks. Verify status:

```bash
# Check all services
cd infra/compose && podman-compose ps

# Check specific service
curl -sf http://localhost:8080/health  # Mock parking operator
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Port already in use | Check for conflicting services: `lsof -i :PORT` |
| Container won't start | Check logs: `podman-compose logs SERVICE` |
| TLS errors | Regenerate certificates: `./scripts/generate-certs.sh` |
