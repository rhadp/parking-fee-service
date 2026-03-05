# PRD: Mock CLI Apps

> Extracted from the master PRD at `.specs/prd.md`. This spec covers the mock PARKING_APP CLI and mock COMPANION_APP CLI.

## Scope

Implement mock CLI applications in Go that simulate the PARKING_APP (AAOS) and COMPANION_APP (mobile) for testing backend services and RHIVOS components without real Android builds.

## Rationale

The two Android applications (PARKING_APP on AAOS and COMPANION_APP on mobile Android) differ fundamentally from the rest of the codebase. To avoid coupling their development lifecycle to the backend-services and RHIVOS components:

- Mock apps expose the same gRPC interface stubs and follow the same messaging protocols as the real apps
- Can be run interactively for manual testing or scripted for automated integration tests
- Written in Go (same language as backend services) to minimize toolchain overhead
- Share the same `.proto` definitions and message schemas as the real Android apps

## Mock PARKING_APP CLI

Simulates the PARKING_APP running on AAOS IVI. Subcommands:

- `lookup --lat=<lat> --lon=<lon>` — query PARKING_FEE_SERVICE for operators at location
- `adapter-info --operator-id=<id>` — get adapter metadata from PARKING_FEE_SERVICE
- `install --image-ref=<ref> --checksum=<sha256>` — trigger adapter install via UPDATE_SERVICE (gRPC)
- `watch` — watch adapter state changes via UPDATE_SERVICE (gRPC streaming)
- `list` — list installed adapters via UPDATE_SERVICE
- `remove --adapter-id=<id>` — remove adapter via UPDATE_SERVICE
- `status --adapter-id=<id>` — get adapter status via UPDATE_SERVICE
- `start-session --zone-id=<zone>` — manually start parking session via PARKING_OPERATOR_ADAPTOR (gRPC)
- `stop-session --session-id=<id>` — manually stop parking session via PARKING_OPERATOR_ADAPTOR (gRPC)

### Connections

- PARKING_FEE_SERVICE: REST (HTTP, port 8080)
- UPDATE_SERVICE: gRPC (network TCP, port 50051)
- PARKING_OPERATOR_ADAPTOR: gRPC (network TCP, port 50052)
- DATA_BROKER: gRPC (network TCP, port 55556) — for reading vehicle state

## Mock COMPANION_APP CLI

Simulates the COMPANION_APP on a mobile device. Subcommands:

- `lock --vin=<vin>` — send lock command via CLOUD_GATEWAY REST API
- `unlock --vin=<vin>` — send unlock command via CLOUD_GATEWAY REST API
- `status --vin=<vin>` — query vehicle status via CLOUD_GATEWAY REST API

### Connections

- CLOUD_GATEWAY: REST (HTTP, port 8081)
- Uses bearer token authentication

## Tech Stack

- Language: Go 1.22+
- gRPC client: google.golang.org/grpc
- HTTP client: net/http
- CLI framework: standard flag package or cobra

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Requires Go module structure, proto definitions, and generated gRPC code |
| 05_parking_fee_service | 2 | 1 | Mock PARKING_APP CLI calls PARKING_FEE_SERVICE REST API |
| 06_cloud_gateway | 2 | 1 | Mock COMPANION_APP CLI calls CLOUD_GATEWAY REST API |
| 07_update_service | 2 | 1 | Mock PARKING_APP CLI calls UPDATE_SERVICE gRPC API |
| 08_parking_operator_adaptor | 2 | 1 | Mock PARKING_APP CLI calls PARKING_OPERATOR_ADAPTOR gRPC API |
