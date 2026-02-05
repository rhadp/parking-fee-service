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
│   └── main.go
└── cloud-gateway/           # MQTT broker/router
    └── main.go
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

## Building

```bash
# Build all backend services
make build-backend

# Build individual service
cd backend && go build -o bin/parking-fee-service ./parking-fee-service

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
