# Backend Services

Go services for the SDV Parking Demo System backend infrastructure.

## Directory Structure

```
backend/
├── go.mod                    # Go module definition
├── go.sum                    # Dependency checksums
├── gen/                      # Generated protobuf/gRPC code
│   ├── common/              # Common error types
│   ├── services/            # Service definitions
│   │   ├── databroker/
│   │   ├── locking/
│   │   ├── parking/
│   │   └── update/
│   └── vss/                 # VSS signal types
├── parking-fee-service/     # Parking operations service
│   └── cmd/server/main.go
├── cloud-gateway/           # MQTT broker/router
│   └── cmd/server/main.go
├── companion-cli/           # CLI simulator for COMPANION_APP
│   └── cmd/companion-cli/main.go
└── parking-cli/             # CLI simulator for PARKING_APP
    └── cmd/parking-cli/main.go
```

## Services

### PARKING_FEE_SERVICE

REST API service for parking operations:
- Session management
- Fee calculation
- Payment processing integration

**Endpoints:**
- `POST /api/v1/sessions` - Start parking session
- `DELETE /api/v1/sessions/{id}` - Stop parking session
- `GET /api/v1/sessions/{id}` - Get session status

### CLOUD_GATEWAY

MQTT broker/router for vehicle-to-cloud communication:
- Message routing between vehicles and backend
- Command forwarding to LOCKING_SERVICE
- Telemetry aggregation

## CLI Simulators

Two CLI tools simulate the mobile apps for local development and testing.

### COMPANION_CLI

Simulates the Flutter COMPANION_APP for remote vehicle control.

**Commands:**
- `lock [door]` - Lock specified door (driver, passenger, rear_left, rear_right, all)
- `unlock [door]` - Unlock specified door
- `status <command_id>` - Check command status
- `ping` - Test CLOUD_GATEWAY connectivity
- `help` - Show available commands
- `quit` - Exit the CLI

**Environment Variables:**
| Variable | Description | Default |
|----------|-------------|---------|
| `CLOUD_GATEWAY_URL` | CLOUD_GATEWAY REST API URL | http://localhost:8080 |
| `VIN` | Vehicle identification number | DEMO_VIN_001 |
| `TIMEOUT` | Request timeout | 10s |

### PARKING_CLI

Simulates the Kotlin PARKING_APP for parking session management.

**Commands:**
- `location <lat> <lng>` - Set vehicle GPS location
- `location` - Get current vehicle location
- `zone` - Get parking zone for current location
- `adapters` - List installed parking adapters
- `install <image_ref>` - Install a parking adapter
- `uninstall <adapter_id>` - Remove a parking adapter
- `start <zone_id>` - Start parking session in zone
- `stop` - Stop current parking session
- `session` - Get current session status
- `locks` - Get door lock states
- `ping` - Test connectivity to all services
- `help` - Show available commands
- `quit` - Exit the CLI

**Environment Variables:**
| Variable | Description | Default |
|----------|-------------|---------|
| `DATA_BROKER_ADDR` | DATA_BROKER gRPC address | localhost:55556 |
| `PARKING_FEE_SERVICE_URL` | PARKING_FEE_SERVICE REST URL | http://localhost:8081 |
| `UPDATE_SERVICE_ADDR` | UPDATE_SERVICE gRPC address | localhost:50051 |
| `PARKING_ADAPTOR_ADDR` | PARKING_OPERATOR_ADAPTOR gRPC address | localhost:50052 |
| `LOCKING_SERVICE_ADDR` | LOCKING_SERVICE gRPC address | localhost:50053 |
| `TIMEOUT` | Request timeout | 10s |

### Non-Interactive Mode

Both CLIs support non-interactive mode for scripting:

```bash
# Execute single command
companion-cli -c "lock all"
parking-cli -c "location 52.5200 13.4050"

# Positional arguments
companion-cli lock all
parking-cli adapters

# Pipe commands from stdin
echo "lock all" | companion-cli
echo -e "location 52.5 13.4\nzone" | parking-cli

# JSON output for scripting
companion-cli --json -c "status abc123"
parking-cli --json adapters

# Quiet mode (suppress informational messages)
companion-cli -q -c "lock all"
```

**Exit Codes:**
- `0` - Success
- `1` - Command failed
- `2` - Invalid arguments
- `3` - Connection error

## Building

```bash
# Build all backend services
make build-backend

# Build individual service
cd backend && go build -o bin/parking-fee-service ./parking-fee-service/cmd/server

# Build CLI simulators
make build-cli

# Build individual CLI
make build-companion-cli
make build-parking-cli

# Run tests
cd backend && go test ./...
```

## Dependencies

- Go 1.21+
- Protocol Buffer compiler (protoc)
- protoc-gen-go and protoc-gen-go-grpc plugins

## Proto Generation

```bash
make proto-go
```

Generated code is placed in `backend/gen/`.

## Configuration

Services read configuration from environment variables or `infra/config/development.yaml`.

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | HTTP server port | 8080 |
| `MQTT_BROKER` | MQTT broker URL | mqtt://localhost:1883 |
| `TLS_ENABLED` | Enable TLS | false |
