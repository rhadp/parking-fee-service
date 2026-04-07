# Implementation Plan: PARKING_FEE_SERVICE

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md — all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the PARKING_FEE_SERVICE as a Go HTTP server in `backend/parking-fee-service/`. The service provides operator discovery (geofence-based), adapter metadata retrieval, and health check endpoints. Task group 1 writes failing tests. Groups 2-3 implement pure-function modules (model, config, geo, store). Group 4 implements HTTP handlers and wires up main. Group 5 verifies end-to-end wiring.

Ordering: tests first, then data types, then pure-function modules (geo, config, store), then HTTP handlers and main, then wiring verification.

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
    - `TestPointInPolygonInside` -- TS-05-2 (inside returns true)
    - `TestPointInPolygonOutside` -- TS-05-2 (outside returns false)
    - `TestProximityMatchingWithinThreshold` -- TS-05-3
    - `TestProximityThresholdUsed` -- TS-05-11
    - _Test Spec: TS-05-2, TS-05-3, TS-05-11_

  - [x] 1.3 Write config and store package tests
    - `TestLoadConfigFromFile` -- TS-05-9
    - `TestConfigStructureValidation` -- TS-05-10
    - `TestConfigFileMissingDefaults` -- TS-05-E5
    - `TestConfigInvalidJSON` -- TS-05-E6
    - `TestMultipleOperatorsReturned` -- TS-05-4
    - _Test Spec: TS-05-4, TS-05-9, TS-05-10, TS-05-E5, TS-05-E6_

  - [x] 1.4 Write handler integration tests (httptest)
    - `TestOperatorLookup` -- TS-05-1
    - `TestEmptyArrayNoMatches` -- TS-05-5
    - `TestAdapterMetadataRetrieval` -- TS-05-6
    - `TestAdapterMetadataHTTP200` -- TS-05-7
    - `TestHealthCheck` -- TS-05-8
    - `TestContentTypeHeader` -- TS-05-12
    - `TestOperatorResponseFields` -- TS-05-13
    - `TestErrorResponseFormat` -- TS-05-14
    - `TestMissingLatLon` -- TS-05-E1
    - `TestInvalidCoordinateRange` -- TS-05-E2
    - `TestNonNumericCoordinates` -- TS-05-E3
    - `TestUnknownOperatorID` -- TS-05-E4
    - _Test Spec: TS-05-1, TS-05-5, TS-05-6, TS-05-7, TS-05-8, TS-05-12, TS-05-13, TS-05-14, TS-05-E1, TS-05-E2, TS-05-E3, TS-05-E4_

  - [x] 1.5 Write property tests
    - `TestPropertyPointInPolygon` -- TS-05-P1
    - `TestPropertyProximityMatching` -- TS-05-P2
    - `TestPropertyOperatorZoneAssociation` -- TS-05-P3
    - `TestPropertyCoordinateValidation` -- TS-05-P4
    - `TestPropertyAdapterCompleteness` -- TS-05-P5
    - `TestPropertyConfigDefaults` -- TS-05-P6
    - _Test Spec: TS-05-P1 through TS-05-P6_

  - [x] 1.V Verify task group 1
    - [x] All test files compile: `cd backend && go test -v ./parking-fee-service/... -run NONE`
    - [x] All spec tests FAIL (red): `cd backend && go test -v ./parking-fee-service/... 2>&1 | grep FAIL`
    - [x] No linter warnings: `cd backend && go vet ./parking-fee-service/...`

- [ ] 2. Model, config, and store modules
  - [ ] 2.1 Implement model package
    - Define types: `Coordinate`, `Zone`, `Rate`, `AdapterMeta`, `Operator`, `Config`
    - Add JSON struct tags for all fields
    - Define `OperatorResponse` struct (excludes `Adapter` field) for lookup responses
    - _Requirements: 05-REQ-5.2_

  - [ ] 2.2 Implement config package
    - `LoadConfig(path string) (*model.Config, error)`: read JSON file, unmarshal into Config
    - `DefaultConfig() *model.Config`: built-in Munich demo data (2 zones, 2 operators)
    - If file not found: return DefaultConfig(), log warning
    - If invalid JSON: return error
    - Default port: 8080, default proximity threshold: 500m
    - Support `CONFIG_PATH` env var in main
    - _Requirements: 05-REQ-4.1, 05-REQ-4.2, 05-REQ-4.3, 05-REQ-4.E1, 05-REQ-4.E2_

  - [ ] 2.3 Implement store package
    - `NewStore(zones []model.Zone, operators []model.Operator) *Store`
    - `GetZone(id string) (*model.Zone, bool)` -- lookup by zone ID
    - `GetOperator(id string) (*model.Operator, bool)` -- lookup by operator ID
    - `GetOperatorsByZoneIDs(zoneIDs []string) []model.Operator` -- returns all operators matching any zone ID
    - Index zones by ID, operators by ID, operators by zone ID
    - _Requirements: 05-REQ-1.4, 05-REQ-2.1_

  - [ ] 2.V Verify task group 2
    - [ ] Config and store tests pass: `cd backend && go test -v ./parking-fee-service/config/... ./parking-fee-service/store/...`
    - [ ] All existing tests still pass: `cd backend && go test -v ./...`
    - [ ] No linter warnings: `cd backend && go vet ./parking-fee-service/...`
    - [ ] _Test Spec: TS-05-4, TS-05-9, TS-05-10, TS-05-E5, TS-05-E6, TS-05-P3, TS-05-P5, TS-05-P6_

- [ ] 3. Geo module
  - [ ] 3.1 Implement PointInPolygon
    - Ray casting algorithm for point-in-polygon test
    - Takes `model.Coordinate` point and `[]model.Coordinate` polygon
    - Returns `bool`
    - _Requirements: 05-REQ-1.2_

  - [ ] 3.2 Implement HaversineDistance
    - Great-circle distance between two `model.Coordinate` values
    - Returns distance in meters
    - Uses `math` stdlib
    - _Requirements: 05-REQ-1.3_

  - [ ] 3.3 Implement DistanceToPolygonEdge
    - Minimum distance from a point to the nearest edge of a polygon
    - Iterates over polygon edges, computes perpendicular distance to each segment
    - Returns distance in meters
    - _Requirements: 05-REQ-1.3_

  - [ ] 3.4 Implement FindMatchingZones
    - For each zone: check PointInPolygon first, then proximity if outside
    - Returns list of matching zone IDs
    - Uses configured proximity threshold
    - _Requirements: 05-REQ-1.1, 05-REQ-1.3, 05-REQ-1.5_

  - [ ] 3.V Verify task group 3
    - [ ] Geo tests pass: `cd backend && go test -v ./parking-fee-service/geo/...`
    - [ ] Property tests pass: `cd backend && go test -v ./parking-fee-service/... -run Property`
    - [ ] All existing tests still pass: `cd backend && go test -v ./...`
    - [ ] No linter warnings: `cd backend && go vet ./parking-fee-service/...`
    - [ ] _Test Spec: TS-05-2, TS-05-3, TS-05-11, TS-05-P1, TS-05-P2_

- [ ] 4. HTTP handlers and main
  - [ ] 4.1 Implement handler package
    - `NewOperatorHandler(store *store.Store, zones []model.Zone, threshold float64) http.HandlerFunc`:
      - Parse and validate lat/lon query params
      - Call `geo.FindMatchingZones`, then `store.GetOperatorsByZoneIDs`
      - Return JSON array of `model.OperatorResponse` (excludes adapter)
      - Handle errors: missing params (400), invalid coords (400)
    - `NewAdapterHandler(store *store.Store) http.HandlerFunc`:
      - Extract operator ID from URL path via `r.PathValue("id")`
      - Call `store.GetOperator`, return adapter metadata JSON
      - Handle errors: unknown ID (404)
    - `HealthHandler() http.HandlerFunc`:
      - Return `{"status":"ok"}` with 200
    - Set `Content-Type: application/json` on all responses
    - Use `{"error":"<message>"}` format for errors
    - _Requirements: 05-REQ-1.1, 05-REQ-1.E1, 05-REQ-1.E2, 05-REQ-1.E3, 05-REQ-2.1, 05-REQ-2.E1, 05-REQ-3.1, 05-REQ-5.1, 05-REQ-5.2, 05-REQ-5.3_

  - [ ] 4.2 Implement main package
    - Read `CONFIG_PATH` env var (default "config.json")
    - Call `config.LoadConfig`, create `store.Store`
    - Register routes using Go 1.22 ServeMux patterns:
      - `GET /operators` -> OperatorHandler
      - `GET /operators/{id}/adapter` -> AdapterHandler
      - `GET /health` -> HealthHandler
    - Start HTTP server on configured port
    - Log version, port, zone count, operator count at startup
    - Handle SIGTERM/SIGINT with `http.Server.Shutdown()`
    - Use `log/slog` for structured logging
    - _Requirements: 05-REQ-4.1, 05-REQ-6.1, 05-REQ-6.2_

  - [ ] 4.V Verify task group 4
    - [ ] All handler tests pass: `cd backend && go test -v ./parking-fee-service/handler/...`
    - [ ] All spec tests pass: `cd backend && go test -v ./parking-fee-service/...`
    - [ ] Binary builds: `cd backend && go build ./parking-fee-service/...`
    - [ ] All existing tests still pass: `cd backend && go test -v ./...`
    - [ ] No linter warnings: `cd backend && go vet ./parking-fee-service/...`
    - [ ] _Test Spec: TS-05-1, TS-05-5, TS-05-6, TS-05-7, TS-05-8, TS-05-12, TS-05-13, TS-05-14, TS-05-15, TS-05-16, TS-05-E1, TS-05-E2, TS-05-E3, TS-05-E4, TS-05-P4_

- [ ] 5. Wiring verification
  - [ ] 5.1 Run full test suite
    - All unit, integration, and property tests pass: `cd backend && go test -v ./parking-fee-service/...`
    - All tests in the repo pass: `cd backend && go test -v ./...`
    - _Test Spec: TS-05-1 through TS-05-16, TS-05-E1 through TS-05-E6, TS-05-P1 through TS-05-P6_

  - [ ] 5.2 Run smoke tests
    - Build binary: `cd backend && go build -o parking-fee-service ./parking-fee-service/cmd/`
    - Start service, verify `/health`, `/operators?lat=48.1375&lon=11.5600`, `/operators/parkhaus-munich/adapter` via curl or test harness
    - Verify SIGTERM graceful shutdown
    - _Test Spec: TS-05-SMOKE-1, TS-05-SMOKE-2, TS-05-SMOKE-3_

  - [ ] 5.3 Verify lint and vet
    - `cd backend && go vet ./parking-fee-service/...`
    - No warnings or errors
    - _Requirements: all_

  - [ ] 5.4 Verify config fallback
    - Start service without config file, verify it uses built-in Munich demo data
    - Start service with `CONFIG_PATH=/tmp/custom.json`, verify it uses custom data
    - _Test Spec: TS-05-SMOKE-2_

  - [ ] 5.V Verify task group 5
    - [ ] Zero test failures: `cd backend && go test -v ./parking-fee-service/... 2>&1 | grep -c FAIL` returns 0
    - [ ] Binary starts and serves traffic on port 8080
    - [ ] Binary shuts down cleanly on SIGTERM
    - [ ] No linter warnings: `cd backend && go vet ./parking-fee-service/...`

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

- The PARKING_FEE_SERVICE uses only Go standard library -- no external dependencies. This simplifies the build and eliminates dependency management concerns.
- Property tests in Go use `testing/quick` or table-driven tests with boundary values. Go does not have a direct equivalent to Rust's `proptest`, so property tests are approximated with randomized table-driven tests.
- Integration tests use `net/http/httptest` for in-process HTTP testing -- no need to start a separate server process for most tests.
- Startup logging and graceful shutdown tests (TS-05-15, TS-05-16) may require starting the binary as a subprocess and capturing output/signals.
- The `OperatorResponse` type excludes the `Adapter` field to prevent leaking adapter metadata in operator lookup responses (per design doc: the client must call `/operators/{id}/adapter` separately).
- Go 1.22 `ServeMux` supports `GET /path` and `GET /path/{param}` pattern matching natively -- no need for a third-party router.
