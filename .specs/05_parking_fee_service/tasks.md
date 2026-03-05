# Implementation Tasks: PARKING_FEE_SERVICE (Spec 05)

> Task breakdown for implementing the PARKING_FEE_SERVICE cloud REST API.
> Implements design from `.specs/05_parking_fee_service/design.md`.
> Validates against `.specs/05_parking_fee_service/test_spec.md`.

## Dependencies

| Spec | Dependency | Relationship |
|------|-----------|--------------|
| 01_project_setup | Group 2 (Go workspace) | Requires Go module structure, `go.work` includes `backend/parking-fee-service` |

## Test Commands

| Action | Command |
|--------|---------|
| Unit tests | `cd backend/parking-fee-service && go test ./... -v` |
| Lint | `cd backend/parking-fee-service && go vet ./...` |

## Task Groups

### Group 1: Write Failing Spec Tests

**Goal:** Create Go test files that encode all test specifications. Tests must compile but fail (red phase of red-green-refactor).

#### Task 1.1: Initialize Go module

Create `backend/parking-fee-service/go.mod` with module path `github.com/rhadp/parking-fee-service/backend/parking-fee-service` and Go 1.22+ directive. Add module to the root `go.work` file if it exists.

**Files:** `backend/parking-fee-service/go.mod`

#### Task 1.2: Create model type stubs

Create `backend/parking-fee-service/model.go` with empty or minimal struct definitions for `Operator`, `LatLon`, `AdapterMetadata`, and `ErrorResponse` so that test files compile.

**Files:** `backend/parking-fee-service/model.go`

#### Task 1.3: Write handler tests

Create `backend/parking-fee-service/handler_test.go` with test functions covering:

- `TestOperatorLookupInsideZone` (TS-05-1)
- `TestOperatorLookupNearZone` (TS-05-2)
- `TestOperatorLookupRemoteLocation` (TS-05-3)
- `TestAdapterMetadataValid` (TS-05-4)
- `TestAdapterMetadataUnknownOperator` (TS-05-E1)
- `TestHealthCheck` (TS-05-5)
- `TestInvalidLatitude` (TS-05-E2) -- table-driven sub-tests
- `TestInvalidLongitude` (TS-05-E3) -- table-driven sub-tests
- `TestUndefinedRoute` (TS-05-E4)
- `TestResponseFormatConsistency` (TS-05-P2)

Tests use `httptest.NewRecorder` and call handler functions or a test server. All tests should compile but fail because handlers are not yet implemented.

**Files:** `backend/parking-fee-service/handler_test.go`

#### Task 1.4: Write geofence tests

Create `backend/parking-fee-service/geofence_test.go` with test functions covering:

- `TestPointInPolygonInside` -- point known to be inside a rectangle
- `TestPointInPolygonOutside` -- point known to be far outside
- `TestPointInPolygonOnBoundary` -- point on a polygon edge
- `TestPointNearZone` -- point outside but within buffer distance
- `TestPointOutsideBuffer` -- point outside and beyond buffer distance
- `TestGeofenceProperties` (TS-05-P1) -- vertex inclusion, centroid inclusion, distant point exclusion

Tests should compile but fail because geofence functions are not yet implemented.

**Files:** `backend/parking-fee-service/geofence_test.go`

#### Task 1.5: Write store tests

Create `backend/parking-fee-service/store_test.go` with test functions covering:

- `TestFindOperatorsByLocation` -- delegates to geofence engine, returns matching operators
- `TestGetAdapterMetadata` -- returns metadata for known operator
- `TestGetAdapterMetadataUnknown` -- returns error/nil for unknown operator
- `TestAllOperatorsHaveAdapters` -- every operator has a corresponding adapter entry

Tests should compile but fail because the store is not yet implemented.

**Files:** `backend/parking-fee-service/store_test.go`

**Verification:** `cd backend/parking-fee-service && go test ./... -v` compiles but all tests fail (or are skipped with `t.Skip("not implemented")`).

---

### Group 2: Data Model and In-Memory Store

**Goal:** Implement data model types and the in-memory store with hardcoded Munich demo data.

#### Task 2.1: Implement data model types

Complete `backend/parking-fee-service/model.go` with full struct definitions including JSON tags:

- `Operator` with fields `ID`, `Name`, `Zone`, `Rate`
- `LatLon` with fields `Lat`, `Lon`
- `AdapterMetadata` with fields `ImageRef`, `ChecksumSHA256`, `Version`
- `ErrorResponse` with field `Error`

**Files:** `backend/parking-fee-service/model.go`

#### Task 2.2: Implement in-memory store

Create `backend/parking-fee-service/store.go` with:

- `Store` struct holding `[]Operator` and `map[string]AdapterMetadata`
- `NewStore()` constructor that initializes hardcoded demo data (two Munich operators as specified in design.md)
- `FindOperatorsByLocation(lat, lon float64) []Operator` method (calls geofence engine; stub the call initially, wire up in Group 3)
- `GetAdapterMetadata(operatorID string) (*AdapterMetadata, bool)` method

**Files:** `backend/parking-fee-service/store.go`

**Verification:** Store tests (Task 1.5) for `TestGetAdapterMetadata` and `TestGetAdapterMetadataUnknown` pass.

---

### Group 3: Geofence Engine

**Goal:** Implement point-in-polygon and near-zone matching logic.

#### Task 3.1: Implement point-in-polygon (ray casting)

Create `backend/parking-fee-service/geofence.go` with:

- `PointInPolygon(point LatLon, polygon []LatLon) bool` -- ray-casting algorithm
- Points on the boundary are treated as inside

**Files:** `backend/parking-fee-service/geofence.go`

#### Task 3.2: Implement Haversine distance

Add to `backend/parking-fee-service/geofence.go`:

- `HaversineDistance(a, b LatLon) float64` -- returns distance in meters between two lat/lon points
- `DistanceToSegment(point, segA, segB LatLon) float64` -- minimum distance from point to line segment

**Files:** `backend/parking-fee-service/geofence.go`

#### Task 3.3: Implement near-zone matching

Add to `backend/parking-fee-service/geofence.go`:

- `PointNearPolygon(point LatLon, polygon []LatLon, bufferMeters float64) bool` -- returns true if point is within bufferMeters of any polygon edge
- `PointInOrNearPolygon(point LatLon, polygon []LatLon, bufferMeters float64) bool` -- combines point-in-polygon and near-zone checks

**Files:** `backend/parking-fee-service/geofence.go`

#### Task 3.4: Wire geofence into store

Update `Store.FindOperatorsByLocation` to use `PointInOrNearPolygon` with the default buffer distance of 500 meters.

**Files:** `backend/parking-fee-service/store.go`

**Verification:** All geofence tests (Task 1.4) and store location tests (Task 1.5 `TestFindOperatorsByLocation`) pass.

---

### Group 4: HTTP Handlers

**Goal:** Implement HTTP handler functions for all three endpoints plus error handling.

#### Task 4.1: Implement health handler

Create `backend/parking-fee-service/handler.go` with:

- `HandleHealth(w http.ResponseWriter, r *http.Request)` -- returns `{"status": "ok"}` with 200

**Files:** `backend/parking-fee-service/handler.go`

#### Task 4.2: Implement operator lookup handler

Add to `backend/parking-fee-service/handler.go`:

- `HandleOperatorLookup(store *Store) http.HandlerFunc` -- returns a handler that:
  1. Parses and validates `lat` and `lon` query parameters
  2. Returns 400 for missing, non-numeric, or out-of-range values
  3. Calls `store.FindOperatorsByLocation(lat, lon)`
  4. Returns JSON array (empty array if no matches)

**Files:** `backend/parking-fee-service/handler.go`

#### Task 4.3: Implement adapter metadata handler

Add to `backend/parking-fee-service/handler.go`:

- `HandleAdapterMetadata(store *Store) http.HandlerFunc` -- returns a handler that:
  1. Extracts operator ID from URL path
  2. Calls `store.GetAdapterMetadata(id)`
  3. Returns 404 if not found
  4. Returns adapter metadata JSON if found

**Files:** `backend/parking-fee-service/handler.go`

#### Task 4.4: Implement JSON response helpers and error handling

Add to `backend/parking-fee-service/handler.go`:

- `writeJSON(w http.ResponseWriter, status int, data any)` -- sets Content-Type and writes JSON
- `writeError(w http.ResponseWriter, status int, message string)` -- writes JSON error response
- Default 404 handler for undefined routes returning JSON error

**Files:** `backend/parking-fee-service/handler.go`

**Verification:** All handler tests (Task 1.3) pass.

---

### Group 5: Server Setup and Configuration

**Goal:** Create the main entry point that wires everything together and starts the HTTP server.

#### Task 5.1: Implement main.go

Create `backend/parking-fee-service/main.go` with:

- `main()` function that:
  1. Creates a new `Store` with demo data
  2. Sets up `http.ServeMux` with routes:
     - `GET /health` -> `HandleHealth`
     - `GET /operators` -> `HandleOperatorLookup(store)`
     - `GET /operators/{id}/adapter` -> `HandleAdapterMetadata(store)`
  3. Registers default 404 handler for undefined routes
  4. Starts HTTP server on `:8080`
  5. Logs startup message to stdout

**Files:** `backend/parking-fee-service/main.go`

**Verification:**

1. `cd backend/parking-fee-service && go build .` succeeds.
2. `cd backend/parking-fee-service && go vet ./...` passes.
3. `cd backend/parking-fee-service && go test ./... -v` all tests pass.
4. Manual smoke test: start server and `curl http://localhost:8080/health` returns `{"status":"ok"}`.

---

### Group 6: Checkpoint

**Goal:** Final validation that all requirements are met and all tests pass.

#### Task 6.1: Run full test suite

Run `cd backend/parking-fee-service && go test ./... -v` and confirm all tests pass.

#### Task 6.2: Run linter

Run `cd backend/parking-fee-service && go vet ./...` and confirm no issues.

#### Task 6.3: Verify endpoint behavior

Start the server and manually verify:

1. `GET /health` returns 200 with `{"status":"ok"}`
2. `GET /operators?lat=48.1395&lon=11.5625` returns muc-central operator
3. `GET /operators?lat=52.52&lon=13.40` returns empty array
4. `GET /operators/muc-central/adapter` returns adapter metadata
5. `GET /operators/unknown/adapter` returns 404
6. `GET /operators?lat=abc&lon=11.58` returns 400

#### Task 6.4: Review Definition of Done

Confirm all items in the design.md Definition of Done are satisfied.

---

## Traceability

| Task | Requirement(s) | Test(s) |
|------|----------------|---------|
| 1.3, 4.2 | 05-REQ-1 | TS-05-1, TS-05-2, TS-05-3, TS-05-E2, TS-05-E3 |
| 1.3, 4.3 | 05-REQ-2 | TS-05-4, TS-05-E1 |
| 1.3, 4.1 | 05-REQ-3 | TS-05-5 |
| 1.4, 3.1, 3.2, 3.3 | 05-REQ-4 | TS-05-P1, TS-05-1, TS-05-2, TS-05-3 |
| 1.3, 4.4 | 05-REQ-5 | TS-05-E1, TS-05-E2, TS-05-E3, TS-05-E4 |
| 4.4 | 05-REQ-6 | TS-05-P2 |
| 2.1, 2.2 | 05-REQ-1, 05-REQ-2 | TS-05-1, TS-05-4 |
| 3.4 | 05-REQ-1, 05-REQ-4 | TS-05-1, TS-05-2, TS-05-3 |
| 5.1 | All | All |
