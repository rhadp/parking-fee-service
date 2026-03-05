# PRD: Project Setup (Phase 1.2)

> Extracted from the master PRD at `.specs/prd.md`. This spec covers Phase 1.2: Setup.

## Scope

Set up the monorepo structure, shared protocol buffer definitions, build system, local development infrastructure, and test framework for the SDV Parking Demo System.

## Deliverables

1. **Repository structure** as described in the master PRD:
   - `proto/` — shared .proto definitions (common.proto, update_service.proto, parking_adaptor.proto)
   - `gen/go/` — generated Go protobuf code (commonpb/, updateservicepb/, parkingadaptorpb/)
   - `rhivos/` — Rust workspace with skeleton crates (locking-service, cloud-gateway-client, update-service, parking-operator-adaptor)
   - `backend/` — Go modules (parking-fee-service, cloud-gateway)
   - `mock/` — Go mock CLI apps (parking-app-cli, companion-app-cli)
   - `aaos/`, `android/` — placeholder directories
   - `infra/` — local development infrastructure
   - `tests/` — test directories (setup, integration)

2. **Protocol Buffer definitions** — single source of truth for gRPC interfaces:
   - `common.proto`: AdapterState enum, AdapterInfo, ErrorDetails
   - `update_service.proto`: InstallAdapter, WatchAdapterStates, ListAdapters, RemoveAdapter, GetAdapterStatus
   - `parking_adaptor.proto`: StartSession, StopSession, GetStatus, GetRate

3. **Build system** — top-level Makefile with targets:
   - `make build` — build all components (Rust + Go)
   - `make test` — run all unit tests
   - `make lint` — run linters (cargo clippy + go vet)
   - `make check` — build + test + lint
   - `make proto` — regenerate Go code from .proto definitions
   - `make infra-up` / `make infra-down` — manage local infrastructure
   - `make clean` — remove build artifacts

4. **Local infrastructure** — docker-compose with:
   - NATS server (nats-server) on port 4222
   - Eclipse Kuksa Databroker on port 55556

5. **Go workspace** — `go.work` linking all Go modules

6. **Rust workspace** — `rhivos/Cargo.toml` workspace with skeleton crates

## Tech Stack

- Rust 1.75+ for RHIVOS services
- Go 1.22+ for backend services and mock CLI apps
- protoc 3.x + protoc-gen-go + protoc-gen-go-grpc for code generation
- Podman/Docker for local infrastructure
- GNU Make for build orchestration

## Local Development Ports

| Service | Protocol | Port |
|---------|----------|------|
| NATS Server | NATS | 4222 |
| Eclipse Kuksa Databroker | gRPC | 55556 |
| PARKING_FEE_SERVICE | HTTP | 8080 |
| CLOUD_GATEWAY | HTTP | 8081 |
| UPDATE_SERVICE | gRPC | 50051 |
| PARKING_OPERATOR_ADAPTOR | gRPC | 50052 |
