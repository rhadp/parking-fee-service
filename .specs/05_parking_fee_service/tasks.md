# Implementation Plan: PARKING_FEE_SERVICE

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from main -> implement -> test -> merge to main -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the PARKING_FEE_SERVICE as a Go HTTP server in `backend/parking-fee-service/`. The service provides operator discovery (geofence-based), adapter metadata retrieval, and health check endpoints. Task group 1 writes failing tests. Groups 2-3 implement pure-function modules (model, config, geo, store). Group 4 implements HTTP handlers and wires up main. Group 5 runs integration validation.

Ordering: tests first, then data types, then pure-function modules (geo, config, store), then HTTP handlers and main, then integration validation.

## Test Commands

- Spec tests: `cd backend && go test -v ./parking-fee-service/...`
- Property tests: `cd backend && go test -v ./parking-fee-service/... -run Property`
- All tests: `cd backend && go test -v ./...`
- Linter: `cd backend && go vet ./parking-fee-service/...`

## Tasks

- [x] 1. Write failing spec tests
  - [x] 1.1 Set up Go module and test file structure
    - Ensure `backend/parking-fee-service/` has `go.mod` (or is part of Go workspace)
    - Create package directories: `model/`, `config/`, `geo/`, `store/`, `handler/`
    - Create test files: `geo/geo_test.go`, `config/config_test.go`, `store/store_test.go`, `handler/handler_test.go`
    - _Test Spec: TS-05-1 through TS-05-16_

  - [x] 1.2 Write geo package tests
    - `TestPointInPolygonInside` — TS-05-2 (inside returns true)
    - `TestPointInPolygonOutside` — TS-05-2 (outside returns false)
    - `TestProximityMatchingWithinThreshold` — TS-05-3
    - `TestProximityThresholdUsed` — TS-05-11
    - _Test Spec: TS-05-2, TS-05-3, TS-05-11_

  - [x] 1.3 Write config and store package tests
    - `TestLoadConfigFromFile` — TS-05-9
    - `TestConfigStructureValidation` — TS-05-10
    - `TestConfigFileMissingDefaults` — TS-05-E5
    - `TestConfigInvalidJSON` — TS-05-E6
    - `TestMultipleOperatorsReturned` — TS-05-4
    - _Test Spec: TS-05-4, TS-05-9, TS-05-10, TS-05-E5, TS-05-E6_

  - [x] 1.4 Write handler integration tests (httptest)
    - `TestOperatorLookup` — TS-05-1
    - `TestEmptyArrayNoMatches` — TS-05-5
    - `TestAdapterMetadataRetrieval` — TS-05-6
    - `TestAdapterMetadataHTTP200` — TS-05-7
    - `TestHealthCheck` — TS-05-8
    - `TestContentTypeHeader` — TS-05-12
    - `TestOperatorResponseFields` — TS-05-13
    - `TestErrorResponseFormat` — TS-05-14
    - `TestMissingLatLon` — TS-05-E1
    - `TestInvalidCoordinateRange` — TS-05-E2
    - `TestNonNumericCoordinates` — TS-05-E3
    - `TestUnknownOperatorID` — TS-05-E4
    - _Test Spec: TS-05-1, TS-05-5, TS-05-6, TS-05-7, TS-05-8, TS-05-12, TS-05-13, TS-05-14, TS-05-E1, TS-05-E2, TS-05-E3, TS-05-E4_

  - [x] 1.5 Write property tests
    - `TestPropertyPointInPolygon` — TS-05-P1
    - `TestPropertyProximityMatching` — TS-05-P2
    - `TestPropertyOperatorZoneAssociation` — TS-05-P3
    - `TestPropertyCoordinateValidation` — TS-05-P4
    - `TestPropertyAdapterCompleteness` — TS-05-P5
    - `TestPropertyConfigDefaults` — TS-05-P6
    - _Test Spec: TS-05-P1 through TS-05-P6_

  - [x] 1.V Verify task group 1
    - [x] All test files compile: `cd backend && go test -v ./parking-fee-service/... -run NONE`
    - [x] All spec tests FAIL (red): `cd backend && go test -v ./parking-fee-service/... 2>&1 | grep FAIL`
    - [x] No linter warnings: `cd backend && go vet ./parking-fee-service/...`

- [x] 2. Model, config, and store modules
  - [x] 2.1 Implement model package
    - Define types: `Coordinate`, `Zone`, `Rate`, `AdapterMeta`, `Operator`, `Config`
    - Add JSON struct tags for all fields
    - Define `OperatorResponse` struct (excludes `Adapter` field) for lookup responses
    - _Requirements: 05-REQ-5.2_

  - [x] 2.2 Implement config package
    - `LoadConfig(path string) (*Config, error)`: read JSON file, unmarshal into Config
    - `DefaultConfig() *Config`: built-in Munich demo data (2 zones, 2 operators)
    - If file not found: return DefaultConfig, log warning
    - If invalid JSON: return error
    - Default port: 8080, default proximity threshold: 500m
    - Support `CONFIG_PATH` env var in main
    - _Requirements: 05-REQ-4.1, 05-REQ-4.2, 05-REQ-4.3, 05-REQ-4.E1, 05-REQ-4.E2_

  - [x] 2.3 Implement store package
    - `NewStore(zones []Zone, operators []Operator) *Store`
    - `GetZone(id string) (*Zone, bool)` — lookup by zone ID
    - `GetOperator(id string) (*Operator, bool)` — lookup by operator ID
    - `GetOperatorsByZoneIDs(zoneIDs []string) []Operator` — returns all operators matching any zone ID
    - Index zones by ID, operators by ID, operators by zone ID
    - _Requirements: 05-REQ-1.4, 05-REQ-2.1_

  - [x] 2.V Verify task group 2
    - [x] Config and store tests pass: `cd backend && go test -v ./parking-fee-service/config/... ./parking-fee-service/store/...`
    - [x] All existing tests still pass: `cd backend && go test -v ./...`
    - [x] No linter warnings: `cd backend && go vet ./parking-fee-service/...`
    - [x] _Test Spec: TS-05-4, TS-05-9, TS-05-10, TS-05-E5, TS-05-E6, TS-05-P3, TS-05-P5, TS-05-P6_

- [x] 3. Geo module
  - [x] 3.1 Implement PointInPolygon
    - Ray casting algorithm for point-in-polygon test
    - Takes `Coordinate` point and `[]Coordinate` polygon
    - Returns `bool`
    - _Requirements: 05-REQ-1.2_

  - [x] 3.2 Implement HaversineDistance
    - Great-circle distance between two `Coordinate` values
    - Returns distance in meters
    - Uses `math` stdlib
    - _Requirements: 05-REQ-1.3_

  - [x] 3.3 Implement DistanceToPolygonEdge
    - Minimum distance from a point to the nearest edge of a polygon
    - Iterates over polygon edges, computes perpendicular distance to each segment
    - Returns distance in meters
    - _Requirements: 05-REQ-1.3_

  - [x] 3.4 Implement FindMatchingZones
    - For each zone: check PointInPolygon first, then proximity if outside
    - Returns list of matching zone IDs
    - Uses configured proximity threshold
    - _Requirements: 05-REQ-1.1, 05-REQ-1.3, 05-REQ-1.5_

  - [x] 3.V Verify task group 3
    - [x] Geo tests pass: `cd backend && go test -v ./parking-fee-service/geo/...`
    - [x] Property tests pass: `cd backend && go test -v ./parking-fee-service/... -run Property`
    - [x] All existing tests still pass: `cd backend && go test -v ./...`
    - [x] No linter warnings: `cd backend && go vet ./parking-fee-service/...`
    - [x] _Test Spec: TS-05-2, TS-05-3, TS-05-11, TS-05-P1, TS-05-P2_

- [x] 4. HTTP handlers and main
  - [x] 4.1 Implement handler package
    - `NewOperatorHandler(store *Store, zones []Zone, threshold float64) http.HandlerFunc`:
      - Parse and validate lat/lon query params
      - Call FindMatchingZones, then GetOperatorsByZoneIDs
      - Return JSON array of OperatorResponse (excludes adapter)
      - Handle errors: missing params (400), invalid coords (400)
    - `NewAdapterHandler(store *Store) http.HandlerFunc`:
      - Extract operator ID from URL path
      - Call GetOperator, return adapter metadata JSON
      - Handle errors: unknown ID (404)
    - `HealthHandler() http.HandlerFunc`:
      - Return `{"status":"ok"}` with 200
    - Set `Content-Type: application/json` on all responses
    - Use `{"error":"<message>"}` format for errors
    - _Requirements: 05-REQ-1.1, 05-REQ-1.E1, 05-REQ-1.E2, 05-REQ-1.E3, 05-REQ-2.1, 05-REQ-2.E1, 05-REQ-3.1, 05-REQ-5.1, 05-REQ-5.2, 05-REQ-5.3_

  - [x] 4.2 Implement main package
    - Read `CONFIG_PATH` env var (default "config.json")
    - Call LoadConfig, create Store
    - Register routes using Go 1.22 ServeMux patterns:
      - `GET /operators` → OperatorHandler
      - `GET /operators/{id}/adapter` → AdapterHandler
      - `GET /health` → HealthHandler
    - Start HTTP server on configured port
    - Log version, port, zone count, operator count at startup
    - Handle SIGTERM/SIGINT with `http.Server.Shutdown()`
    - Use `log/slog` for structured logging
    - _Requirements: 05-REQ-4.1, 05-REQ-6.1, 05-REQ-6.2_

  - [x] 4.V Verify task group 4
    - [x] All handler tests pass: `cd backend && go test -v ./parking-fee-service/handler/...`
    - [x] All spec tests pass: `cd backend && go test -v ./parking-fee-service/...`
    - [x] Binary builds: `cd backend && go build ./parking-fee-service/...`
    - [x] All existing tests still pass: `cd backend && go test -v ./...`
    - [x] No linter warnings: `cd backend && go vet ./parking-fee-service/...`
    - [x] _Test Spec: TS-05-1, TS-05-5, TS-05-6, TS-05-7, TS-05-8, TS-05-12, TS-05-13, TS-05-14, TS-05-15, TS-05-16, TS-05-E1, TS-05-E2, TS-05-E3, TS-05-E4, TS-05-P4_

- [ ] 5. Checkpoint - All Tests Green
  - All unit, integration, and property tests pass
  - Binary starts, serves requests, shuts down cleanly
  - Ask the user if questions arise

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 05-REQ-1.1 | TS-05-1 | 4.1 | handler::TestOperatorLookup |
| 05-REQ-1.2 | TS-05-2 | 3.1 | geo::TestPointInPolygonInside |
| 05-REQ-1.3 | TS-05-3 | 3.2, 3.3, 3.4 | geo::TestProximityMatchingWithinThreshold |
| 05-REQ-1.4 | TS-05-4 | 2.3 | store::TestMultipleOperatorsReturned |
| 05-REQ-1.5 | TS-05-5 | 4.1 | handler::TestEmptyArrayNoMatches |
| 05-REQ-1.E1 | TS-05-E1 | 4.1 | handler::TestMissingLatLon |
| 05-REQ-1.E2 | TS-05-E2 | 4.1 | handler::TestInvalidCoordinateRange |
| 05-REQ-1.E3 | TS-05-E3 | 4.1 | handler::TestNonNumericCoordinates |
| 05-REQ-2.1 | TS-05-6 | 4.1 | handler::TestAdapterMetadataRetrieval |
| 05-REQ-2.2 | TS-05-7 | 4.1 | handler::TestAdapterMetadataHTTP200 |
| 05-REQ-2.E1 | TS-05-E4 | 4.1 | handler::TestUnknownOperatorID |
| 05-REQ-3.1 | TS-05-8 | 4.1 | handler::TestHealthCheck |
| 05-REQ-4.1 | TS-05-9 | 2.2 | config::TestLoadConfigFromFile |
| 05-REQ-4.2 | TS-05-10 | 2.2 | config::TestConfigStructureValidation |
| 05-REQ-4.3 | TS-05-11 | 3.4 | geo::TestProximityThresholdUsed |
| 05-REQ-4.E1 | TS-05-E5 | 2.2 | config::TestConfigFileMissingDefaults |
| 05-REQ-4.E2 | TS-05-E6 | 2.2 | config::TestConfigInvalidJSON |
| 05-REQ-5.1 | TS-05-12 | 4.1 | handler::TestContentTypeHeader |
| 05-REQ-5.2 | TS-05-13 | 4.1 | handler::TestOperatorResponseFields |
| 05-REQ-5.3 | TS-05-14 | 4.1 | handler::TestErrorResponseFormat |
| 05-REQ-6.1 | TS-05-15 | 4.2 | main::TestStartupLogging |
| 05-REQ-6.2 | TS-05-16 | 4.2 | main::TestGracefulShutdown |
| Property 1 | TS-05-P1 | 3.1, 3.4 | geo::TestPropertyPointInPolygon |
| Property 2 | TS-05-P2 | 3.2, 3.3, 3.4 | geo::TestPropertyProximityMatching |
| Property 3 | TS-05-P3 | 2.3 | store::TestPropertyOperatorZoneAssociation |
| Property 4 | TS-05-P4 | 4.1 | handler::TestPropertyCoordinateValidation |
| Property 5 | TS-05-P5 | 2.3 | store::TestPropertyAdapterCompleteness |
| Property 6 | TS-05-P6 | 2.2 | config::TestPropertyConfigDefaults |

## Notes

- The PARKING_FEE_SERVICE uses only Go standard library — no external dependencies. This simplifies the build and eliminates dependency management concerns.
- Property tests in Go use `testing/quick` or table-driven tests with boundary values. Go does not have a direct equivalent to Rust's `proptest`, so property tests are approximated with randomized table-driven tests.
- Integration tests use `net/http/httptest` for in-process HTTP testing — no need to start a separate server process for most tests.
- Startup logging and graceful shutdown tests (TS-05-15, TS-05-16) may require starting the binary as a subprocess and capturing output/signals.
- The `OperatorResponse` type excludes the `Adapter` field to prevent leaking adapter metadata in operator lookup responses (per design doc: the client must call `/operators/{id}/adapter` separately).
- Go 1.22 `ServeMux` supports `GET /path` and `GET /path/{param}` pattern matching natively — no need for a third-party router.
