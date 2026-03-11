# Implementation Plan: PARKING_FEE_SERVICE (Spec 05)

<!-- AGENT INSTRUCTIONS
- Implement exactly ONE top-level task group per session
- Task group 1 writes failing tests from test_spec.md -- all subsequent groups
  implement code to make those tests pass
- Follow the git-flow: feature branch from develop -> implement -> test -> merge to develop -> push
- Update checkbox states as you go: [-] in progress, [x] complete
-->

## Overview

This plan implements the PARKING_FEE_SERVICE as a standalone Go HTTP server with geofence-based operator discovery and adapter metadata retrieval. Task group 1 writes all failing spec tests. Groups 2-4 implement functionality to make those tests pass. Group 5 runs integration tests and validates the complete service.

## Dependencies

| Spec | From Group | To Group | Relationship |
|------|-----------|----------|--------------|
| 01_project_setup | 2 | 1 | Uses repo structure and Go project skeleton from group 2 |

## Test Commands

- Spec tests: `cd backend/parking-fee-service && go test ./... -v`
- Unit tests: `cd backend/parking-fee-service && go test ./... -run 'Test[^I]' -v`
- Property tests: `cd backend/parking-fee-service && go test ./... -run 'TestProperty' -v`
- All tests: `cd backend/parking-fee-service && go test ./... -v`
- Linter: `cd backend/parking-fee-service && go vet ./...`

## Tasks

- [ ] 1. Write failing spec tests
  - [ ] 1.1 Initialize Go module and create model stubs
    - Create `backend/parking-fee-service/go.mod` with module path `github.com/rhadp/parking-fee-service/backend/parking-fee-service` and Go 1.22+ directive.
    - Add module to root `go.work` file if it exists.
    - Create `backend/parking-fee-service/model.go` with minimal struct stubs for `LatLon`, `Zone`, `Operator`, `AdapterMetadata`, `ErrorResponse`, and `RateType` constants so test files compile.
    - _Test Spec: setup for all tests_

  - [ ] 1.2 Write handler integration tests
    - Create `backend/parking-fee-service/handler_test.go` with test functions:
      - `TestOperatorLookupInsideZone` (TS-05-1)
      - `TestOperatorLookupNearZone` (TS-05-2)
      - `TestOperatorLookupRemoteLocation` (TS-05-3)
      - `TestAdapterMetadataValid` (TS-05-4)
      - `TestHealthCheck` (TS-05-5)
      - `TestOperatorLookupRateInfo` (TS-05-6)
      - `TestAdapterMetadataUnknownOperator` (TS-05-E1)
      - `TestInvalidLatitude` (TS-05-E2) -- table-driven sub-tests
      - `TestInvalidLongitude` (TS-05-E3) -- table-driven sub-tests
      - `TestUndefinedRoute` (TS-05-E4)
    - Tests use `httptest.NewRecorder` and must compile but fail.
    - _Test Spec: TS-05-1 through TS-05-6, TS-05-E1 through TS-05-E4_

  - [ ] 1.3 Write geofence unit tests
    - Create `backend/parking-fee-service/geofence_test.go` with test functions:
      - `TestPointInPolygonInside` -- point inside a rectangle
      - `TestPointInPolygonOutside` -- point far outside
      - `TestPointInPolygonOnBoundary` -- point on a polygon edge
      - `TestPointNearZone` -- point outside but within threshold
      - `TestPointOutsideBuffer` -- point outside and beyond threshold
      - `TestPropertyGeofenceCorrectness` (TS-05-P1) -- vertex/centroid/distant point
      - `TestPropertyProximityThreshold` (TS-05-P2) -- near/far matching
    - Tests must compile but fail.
    - _Test Spec: TS-05-P1, TS-05-P2_

  - [ ] 1.4 Write config and store tests
    - Create `backend/parking-fee-service/config_test.go` with:
      - `TestConfigLoadFromFile` (TS-05-7) -- load from temp JSON file
      - `TestConfigLoadDefault` (TS-05-7) -- load embedded default
      - `TestConfigLoadInvalidFile` (TS-05-E5) -- non-existent and invalid JSON
    - Create `backend/parking-fee-service/store_test.go` with:
      - `TestFindOperatorsByLocation` -- delegates to geofence, returns matches
      - `TestGetAdapterMetadata` -- returns metadata for known operator
      - `TestGetAdapterMetadataUnknown` -- returns error for unknown operator
    - Tests must compile but fail.
    - _Test Spec: TS-05-7, TS-05-E5_

  - [ ] 1.5 Write property tests for response format and adapter integrity
    - Add to `backend/parking-fee-service/handler_test.go`:
      - `TestPropertyResponseFormat` (TS-05-P3) -- all endpoints return JSON
      - `TestPropertyOperatorAdapterIntegrity` (TS-05-P4) -- every lookup result has valid adapter
    - Tests must compile but fail.
    - _Test Spec: TS-05-P3, TS-05-P4_

  - [ ] 1.V Verify task group 1
    - [ ] All spec tests exist and are syntactically valid
    - [ ] All spec tests FAIL (red) -- no implementation yet
    - [ ] No linter warnings introduced: `cd backend/parking-fee-service && go vet ./...`

- [ ] 2. Data models and configuration loading
  - [ ] 2.1 Implement data model types
    - Complete `backend/parking-fee-service/model.go` with full struct definitions including JSON tags for `LatLon`, `Zone`, `Operator`, `RateType`, `AdapterMetadata`, `ErrorResponse`.
    - _Requirements: 05-REQ-6.1, 05-REQ-6.2_

  - [ ] 2.2 Implement configuration loader
    - Create `backend/parking-fee-service/config.go` with:
      - `Config`, `Settings`, `OperatorConfig` structs
      - `LoadConfig(filePath string) (*Config, error)` function
      - Embedded default config JSON via `embed` package
      - Fallback to embedded default when `filePath` is empty
      - Error return for missing or invalid config files
    - _Requirements: 05-REQ-7.1, 05-REQ-7.2, 05-REQ-7.E1_

  - [ ] 2.3 Create default configuration file
    - Create `backend/parking-fee-service/config.json` with the demo data (two Munich zones, two operators with rate info and adapter metadata).
    - _Requirements: 05-REQ-7.1_

  - [ ] 2.4 Implement in-memory store
    - Create `backend/parking-fee-service/store.go` with:
      - `Store` struct holding zones, operators, adapter metadata, and settings
      - `NewStore(cfg *Config) *Store` constructor
      - `FindOperatorsByLocation(lat, lon float64) []Operator` (stub geofence call, wire in group 3)
      - `GetAdapterMetadata(operatorID string) (*AdapterMetadata, bool)`
    - _Requirements: 05-REQ-1.1, 05-REQ-4.1_

  - [ ] 2.V Verify task group 2
    - [ ] Config tests pass: `cd backend/parking-fee-service && go test ./... -run 'TestConfig' -v`
    - [ ] Store metadata tests pass: `cd backend/parking-fee-service && go test ./... -run 'TestGetAdapter' -v`
    - [ ] All existing tests still pass: `cd backend/parking-fee-service && go test ./... -v` (remaining tests still fail as expected)
    - [ ] No linter warnings: `cd backend/parking-fee-service && go vet ./...`

- [ ] 3. Geofence matching (point-in-polygon + proximity)
  - [ ] 3.1 Implement point-in-polygon (ray casting)
    - Create `backend/parking-fee-service/geofence.go` with:
      - `PointInPolygon(point LatLon, polygon []LatLon) bool` -- ray-casting algorithm
      - Points on boundary treated as inside (using epsilon check)
    - _Requirements: 05-REQ-2.1, 05-REQ-2.2_

  - [ ] 3.2 Implement Haversine distance and distance-to-segment
    - Add to `backend/parking-fee-service/geofence.go`:
      - `HaversineDistance(a, b LatLon) float64` -- geodesic distance in meters
      - `DistanceToSegment(point, segA, segB LatLon) float64` -- min distance to segment in meters
      - `MinDistanceToPolygon(point LatLon, polygon []LatLon) float64`
    - _Requirements: 05-REQ-3.1_

  - [ ] 3.3 Implement proximity matching and wire into store
    - Add to `backend/parking-fee-service/geofence.go`:
      - `PointInOrNearPolygon(point LatLon, polygon []LatLon, thresholdMeters float64) bool`
    - Update `Store.FindOperatorsByLocation` to use `PointInOrNearPolygon` with the configured proximity threshold.
    - _Requirements: 05-REQ-3.1, 05-REQ-3.2_

  - [ ] 3.V Verify task group 3
    - [ ] Geofence tests pass: `cd backend/parking-fee-service && go test ./... -run 'TestPoint|TestProperty(Geofence|Proximity)' -v`
    - [ ] Store location tests pass: `cd backend/parking-fee-service && go test ./... -run 'TestFindOperators' -v`
    - [ ] All existing tests still pass: `cd backend/parking-fee-service && go test ./... -v`
    - [ ] No linter warnings: `cd backend/parking-fee-service && go vet ./...`

- [ ] 4. REST API endpoints
  - [ ] 4.1 Implement JSON response helpers and recovery middleware
    - Create `backend/parking-fee-service/handler.go` with:
      - `writeJSON(w http.ResponseWriter, status int, data any)` -- sets Content-Type, writes JSON
      - `writeError(w http.ResponseWriter, status int, message string)` -- writes JSON error response
      - `recoveryMiddleware(next http.Handler) http.Handler` -- recovers panics, returns 500
    - _Requirements: 05-REQ-8.1, 05-REQ-8.2, 05-REQ-8.E2_

  - [ ] 4.2 Implement health handler
    - Add `HandleHealth(w http.ResponseWriter, r *http.Request)` to handler.go.
    - Returns `{"status": "ok"}` with HTTP 200.
    - _Requirements: 05-REQ-5.1_

  - [ ] 4.3 Implement operator lookup handler
    - Add `HandleOperatorLookup(store *Store) http.HandlerFunc` to handler.go:
      - Parse and validate `lat` and `lon` query parameters
      - Return 400 for missing, non-numeric, or out-of-range values
      - Call `store.FindOperatorsByLocation(lat, lon)`
      - Return JSON array (empty array if no matches)
    - _Requirements: 05-REQ-1.1, 05-REQ-1.2, 05-REQ-1.3, 05-REQ-1.E1, 05-REQ-1.E2_

  - [ ] 4.4 Implement adapter metadata handler and default 404 handler
    - Add `HandleAdapterMetadata(store *Store) http.HandlerFunc` to handler.go:
      - Extract operator ID from URL path
      - Return 404 if not found, otherwise return adapter metadata JSON
    - Add default 404 handler for undefined routes returning JSON error.
    - _Requirements: 05-REQ-4.1, 05-REQ-4.E1, 05-REQ-8.E1_

  - [ ] 4.5 Implement main.go server entry point
    - Create `backend/parking-fee-service/main.go` with:
      - Load configuration (from file path env var or embedded default)
      - Create store from config
      - Set up `http.ServeMux` with routes
      - Apply recovery middleware
      - Start HTTP server on configured port
      - Log startup message to stdout
    - _Requirements: 05-REQ-7.1, 05-REQ-7.2_

  - [ ] 4.V Verify task group 4
    - [ ] All handler tests pass: `cd backend/parking-fee-service && go test ./... -run 'Test(Operator|Adapter|Health|Invalid|Undefined|Property)' -v`
    - [ ] All existing tests still pass: `cd backend/parking-fee-service && go test ./... -v`
    - [ ] Build succeeds: `cd backend/parking-fee-service && go build .`
    - [ ] No linter warnings: `cd backend/parking-fee-service && go vet ./...`

- [ ] 5. Integration tests
  - [ ] 5.1 Run full test suite and verify all tests pass
    - Execute `cd backend/parking-fee-service && go test ./... -v`
    - All TS-05-* tests must pass.
    - _Test Spec: TS-05-1 through TS-05-7, TS-05-E1 through TS-05-E5, TS-05-P1 through TS-05-P4_

  - [ ] 5.2 Run linter
    - Execute `cd backend/parking-fee-service && go vet ./...` -- no issues.

  - [ ] 5.3 Manual smoke test
    - Start server: `cd backend/parking-fee-service && go run .`
    - Verify: `curl http://localhost:8080/health` returns `{"status":"ok"}`
    - Verify: `curl "http://localhost:8080/operators?lat=48.1395&lon=11.5625"` returns muc-central
    - Verify: `curl http://localhost:8080/operators/muc-central/adapter` returns metadata
    - Verify: `curl "http://localhost:8080/operators?lat=52.52&lon=13.40"` returns `[]`
    - Verify: `curl http://localhost:8080/operators/unknown/adapter` returns 404

  - [ ] 5.V Verify task group 5
    - [ ] All tests pass: `cd backend/parking-fee-service && go test ./... -v`
    - [ ] No linter warnings: `cd backend/parking-fee-service && go vet ./...`
    - [ ] Build succeeds: `cd backend/parking-fee-service && go build .`
    - [ ] All Definition of Done criteria from design.md are satisfied

- [ ] 6. Checkpoint -- PARKING_FEE_SERVICE Complete
  - Ensure all tests pass, ask the user if questions arise.
  - Review Definition of Done from design.md.

### Checkbox States

| Syntax   | Meaning                |
|----------|------------------------|
| `- [ ]`  | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [X]`  | Completed              |
| `- [-]`  | In progress            |
| `- [~]`  | Queued                 |

## Traceability

| Requirement | Test Spec Entry | Implemented By Task | Verified By Test |
|-------------|-----------------|---------------------|------------------|
| 05-REQ-1.1 | TS-05-1 | 4.3 | `TestOperatorLookupInsideZone` |
| 05-REQ-1.2 | TS-05-3 | 4.3 | `TestOperatorLookupRemoteLocation` |
| 05-REQ-1.3 | TS-05-1 | 2.1, 4.3 | `TestOperatorLookupInsideZone` |
| 05-REQ-1.E1 | TS-05-E2 | 4.3 | `TestInvalidLatitude` |
| 05-REQ-1.E2 | TS-05-E3 | 4.3 | `TestInvalidLongitude` |
| 05-REQ-2.1 | TS-05-P1 | 3.1 | `TestPropertyGeofenceCorrectness` |
| 05-REQ-2.2 | TS-05-P1 | 3.1 | `TestPointInPolygonOnBoundary` |
| 05-REQ-2.E1 | TS-05-P1 | 3.1 | `TestPropertyGeofenceCorrectness` |
| 05-REQ-3.1 | TS-05-2, TS-05-P2 | 3.2, 3.3 | `TestOperatorLookupNearZone`, `TestPropertyProximityThreshold` |
| 05-REQ-3.2 | TS-05-2, TS-05-7 | 2.2, 3.3 | `TestOperatorLookupNearZone`, `TestConfigLoadDefault` |
| 05-REQ-3.E1 | TS-05-3, TS-05-P2 | 3.3 | `TestOperatorLookupRemoteLocation`, `TestPropertyProximityThreshold` |
| 05-REQ-4.1 | TS-05-4 | 4.4 | `TestAdapterMetadataValid` |
| 05-REQ-4.2 | TS-05-4, TS-05-P4 | 2.3 | `TestAdapterMetadataValid`, `TestPropertyOperatorAdapterIntegrity` |
| 05-REQ-4.E1 | TS-05-E1 | 4.4 | `TestAdapterMetadataUnknownOperator` |
| 05-REQ-5.1 | TS-05-5 | 4.2 | `TestHealthCheck` |
| 05-REQ-5.E1 | TS-05-5 | 4.2 | `TestHealthCheck` |
| 05-REQ-6.1 | TS-05-6 | 2.1 | `TestOperatorLookupRateInfo` |
| 05-REQ-6.2 | TS-05-1, TS-05-6 | 2.1, 4.3 | `TestOperatorLookupInsideZone`, `TestOperatorLookupRateInfo` |
| 05-REQ-7.1 | TS-05-7 | 2.2, 2.3 | `TestConfigLoadFromFile` |
| 05-REQ-7.2 | TS-05-7 | 2.2 | `TestConfigLoadDefault` |
| 05-REQ-7.E1 | TS-05-E5 | 2.2 | `TestConfigLoadInvalidFile` |
| 05-REQ-8.1 | TS-05-P3 | 4.1 | `TestPropertyResponseFormat` |
| 05-REQ-8.2 | TS-05-P3 | 4.1 | `TestPropertyResponseFormat` |
| 05-REQ-8.3 | TS-05-E1, TS-05-E2, TS-05-E3 | 4.3, 4.4 | `TestAdapterMetadataUnknownOperator`, `TestInvalidLatitude`, `TestInvalidLongitude` |
| 05-REQ-8.E1 | TS-05-E4 | 4.4 | `TestUndefinedRoute` |
| 05-REQ-8.E2 | TS-05-P3 | 4.1 | `TestPropertyResponseFormat` |

## Notes

- All tests use Go standard library (`testing`, `net/http/httptest`). No external test dependencies.
- The geofence engine uses ray-casting for point-in-polygon and Haversine formula for distance calculations. These are pure functions and highly testable.
- Configuration loading supports both file-based and embedded defaults. The embedded default ensures the service runs without any external files for development and testing.
- Task group 1 creates stub implementations (empty function bodies, minimal types) so tests compile but fail. Subsequent groups fill in the real implementations.
