# Go Development Environment Setup

Guide for setting up a Go development environment for backend services.

## Prerequisites

- macOS, Linux, or Windows
- Git
- Protocol Buffer compiler (protoc)

## Install Go

### macOS

```bash
brew install go
```

### Linux

```bash
# Download and install
wget https://go.dev/dl/go1.21.linux-amd64.tar.gz
sudo tar -C /usr/local -xzf go1.21.linux-amd64.tar.gz

# Add to PATH
echo 'export PATH=$PATH:/usr/local/go/bin' >> ~/.bashrc
echo 'export PATH=$PATH:$(go env GOPATH)/bin' >> ~/.bashrc
source ~/.bashrc
```

### Verify Installation

```bash
go version    # Should be 1.21+
```

## Install Protocol Buffer Tools

### Install protoc

#### macOS

```bash
brew install protobuf
```

#### Linux (Debian/Ubuntu)

```bash
sudo apt-get install -y protobuf-compiler
```

### Install Go Plugins

```bash
go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
```

### Verify

```bash
protoc --version
protoc-gen-go --version
protoc-gen-go-grpc --version
```

## Project Setup

### Clone Repository

```bash
git clone <repository-url>
cd parking-fee-service
```

### Download Dependencies

```bash
cd backend && go mod download
```

### Build Services

```bash
# Build all backend services
make build-backend

# Or build directly
cd backend && go build -o bin/parking-fee-service ./parking-fee-service
```

## Generate Proto Bindings

```bash
make proto-go
```

Generated code is placed in `backend/gen/`.

## Project Structure

```
backend/
├── go.mod                    # Module definition
├── go.sum                    # Dependency checksums
├── gen/                      # Generated proto code
│   ├── common/
│   ├── services/
│   │   ├── databroker/
│   │   ├── locking/
│   │   ├── parking/
│   │   └── update/
│   └── vss/
├── parking-fee-service/      # Parking operations service
│   └── main.go
└── cloud-gateway/            # MQTT broker/router
    └── main.go
```

## Common Tasks

### Run Service

```bash
cd backend && go run ./parking-fee-service
```

### Run Tests

```bash
cd backend && go test ./...
```

### Run with Race Detector

```bash
cd backend && go test -race ./...
```

### Format Code

```bash
cd backend && go fmt ./...
```

### Lint Code

```bash
# Install golangci-lint
go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

# Run linter
cd backend && golangci-lint run
```

## IDE Setup

### VS Code

Install extensions:
- Go (official)

Settings (`.vscode/settings.json`):
```json
{
  "go.useLanguageServer": true,
  "go.lintTool": "golangci-lint",
  "go.formatTool": "goimports",
  "editor.formatOnSave": true
}
```

### GoLand

JetBrains GoLand provides excellent Go support out of the box.

## Dependencies

Key dependencies:

```go
require (
    google.golang.org/grpc v1.58.0
    google.golang.org/protobuf v1.31.0
)
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | HTTP server port | 8080 |
| `MQTT_BROKER` | MQTT broker URL | mqtt://localhost:1883 |
| `TLS_ENABLED` | Enable TLS | false |
| `LOG_LEVEL` | Logging level | info |

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `protoc-gen-go` not found | Add `$(go env GOPATH)/bin` to PATH |
| Module not found | Run `go mod download` |
| Import errors | Run `go mod tidy` |
| Build cache issues | Run `go clean -cache` |

## gRPC Server Example

```go
package main

import (
    "net"
    "google.golang.org/grpc"
    pb "github.com/sdv-parking-demo/backend/gen/services/parking"
)

func main() {
    lis, _ := net.Listen("tcp", ":50051")
    s := grpc.NewServer()
    pb.RegisterParkingAdaptorServer(s, &server{})
    s.Serve(lis)
}
```
