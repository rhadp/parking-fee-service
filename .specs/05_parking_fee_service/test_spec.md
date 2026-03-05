# Test Specification: PARKING_FEE_SERVICE (Spec 05)

## Overview

This test specification defines concrete, language-agnostic test contracts for the PARKING_FEE_SERVICE. Each test case maps to a specific requirement acceptance criterion or correctness property from the design document. The coding agent will translate these contracts into Go test functions using the `testing` standard library and `net/http/httptest`.

## Test Environment

- **Test framework:** Go `testing` standard library
- **HTTP testing:** `net/http/httptest` for handler-level integration tests
- **Test location:** `backend/parking-fee-service/*_test.go`
- **Run command:** `cd backend/parking-fee-service && go test ./... -v`
- **Lint command:** `cd backend/parking-fee-service && go vet ./...`

## Test ID Convention

| Prefix  | Category           |
|---------|--------------------|
| TS-05-  | Functional tests   |
| TS-05-P | Property tests     |
| TS-05-E | Error/edge tests   |

## Test Cases

### TS-05-1: Operator Lookup Returns Operators for Location Inside a Zone

**Requirement:** 05-REQ-1.1, 05-REQ-1.3
**Type:** integration
**Description:** Sending a GET request with coordinates inside the Munich Central zone returns the muc-central operator with all required fields.

**Preconditions:**
- Server is initialized with default demo configuration (two Munich operators).

**Input:**
- `GET /operators?lat=48.1395&lon=11.5625` (inside zone-muc-central polygon)

**Expected:**
- HTTP status 200
- `Content-Type: application/json`
- JSON array containing at least one entry with `operator_id` = `"muc-central"`
- Entry includes `name`, `zone_id`, `rate_type` (`"per_hour"`), `rate_amount` (2.50), `rate_currency` (`"EUR"`)

**Assertion pseudocode:**
```
response = httptest.GET("/operators?lat=48.1395&lon=11.5625")
ASSERT response.status == 200
ASSERT response.header("Content-Type") contains "application/json"
operators = parseJSON(response.body)
ASSERT len(operators) >= 1
op = findByID(operators, "muc-central")
ASSERT op != nil
ASSERT op.name != ""
ASSERT op.zone_id == "zone-muc-central"
ASSERT op.rate_type == "per_hour"
ASSERT op.rate_amount == 2.50
ASSERT op.rate_currency == "EUR"
```

### TS-05-2: Operator Lookup Returns Operators for Location Near a Zone

**Requirement:** 05-REQ-3.1, 05-REQ-3.2
**Type:** integration
**Description:** Sending a GET request with coordinates slightly outside but within the proximity threshold of a zone returns the nearby operator.

**Preconditions:**
- Server is initialized with default demo configuration. Proximity threshold is 500 meters.

**Input:**
- `GET /operators?lat=48.1425&lon=11.5625` (approximately 55 meters north of the muc-central polygon northern edge at lat=48.1420)

**Expected:**
- HTTP status 200
- JSON array containing at least one entry with `operator_id` = `"muc-central"`

**Assertion pseudocode:**
```
response = httptest.GET("/operators?lat=48.1425&lon=11.5625")
ASSERT response.status == 200
operators = parseJSON(response.body)
ASSERT containsOperator(operators, "muc-central")
```

### TS-05-3: Operator Lookup Returns Empty List for Remote Location

**Requirement:** 05-REQ-1.2, 05-REQ-3.E1
**Type:** integration
**Description:** Sending a GET request with coordinates far from any zone returns an empty array.

**Preconditions:**
- Server is initialized with default demo configuration.

**Input:**
- `GET /operators?lat=52.5200&lon=13.4050` (Berlin, far from any Munich zone)

**Expected:**
- HTTP status 200
- `Content-Type: application/json`
- JSON body is an empty array `[]`

**Assertion pseudocode:**
```
response = httptest.GET("/operators?lat=52.5200&lon=13.4050")
ASSERT response.status == 200
ASSERT response.header("Content-Type") contains "application/json"
operators = parseJSON(response.body)
ASSERT len(operators) == 0
```

### TS-05-4: Adapter Metadata Returns Correct Data for Valid Operator

**Requirement:** 05-REQ-4.1, 05-REQ-4.2
**Type:** integration
**Description:** Requesting adapter metadata for a known operator returns the correct image reference, checksum, and version.

**Preconditions:**
- Server is initialized with default demo configuration.

**Input:**
- `GET /operators/muc-central/adapter`

**Expected:**
- HTTP status 200
- `Content-Type: application/json`
- `image_ref` = `"europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-central:v1.0.0"`
- `checksum_sha256` starts with `"sha256:"` followed by 64 hex characters
- `version` = `"v1.0.0"`

**Assertion pseudocode:**
```
response = httptest.GET("/operators/muc-central/adapter")
ASSERT response.status == 200
ASSERT response.header("Content-Type") contains "application/json"
metadata = parseJSON(response.body)
ASSERT metadata.image_ref == "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-central:v1.0.0"
ASSERT metadata.checksum_sha256 matches regex "^sha256:[0-9a-f]{64}$"
ASSERT metadata.version == "v1.0.0"
```

### TS-05-5: Health Check Returns 200 OK

**Requirement:** 05-REQ-5.1
**Type:** integration
**Description:** The health endpoint returns a 200 status with `"status": "ok"`.

**Preconditions:**
- Server is running.

**Input:**
- `GET /health`

**Expected:**
- HTTP status 200
- `Content-Type: application/json`
- JSON body contains `"status": "ok"`

**Assertion pseudocode:**
```
response = httptest.GET("/health")
ASSERT response.status == 200
ASSERT response.header("Content-Type") contains "application/json"
body = parseJSON(response.body)
ASSERT body.status == "ok"
```

### TS-05-6: Operator Lookup Includes Rate Information

**Requirement:** 05-REQ-6.1, 05-REQ-6.2
**Type:** integration
**Description:** The operator lookup returns rate_type, rate_amount, and rate_currency for each operator.

**Preconditions:**
- Server is initialized with default demo configuration containing per_hour and flat_fee operators.

**Input:**
- `GET /operators?lat=48.3525&lon=11.7850` (inside zone-muc-airport polygon)

**Expected:**
- HTTP status 200
- JSON array containing at least one entry with `operator_id` = `"muc-airport"`
- Entry has `rate_type` = `"flat_fee"`, `rate_amount` = 5.00, `rate_currency` = `"EUR"`

**Assertion pseudocode:**
```
response = httptest.GET("/operators?lat=48.3525&lon=11.7850")
ASSERT response.status == 200
operators = parseJSON(response.body)
op = findByID(operators, "muc-airport")
ASSERT op != nil
ASSERT op.rate_type == "flat_fee"
ASSERT op.rate_amount == 5.00
ASSERT op.rate_currency == "EUR"
```

### TS-05-7: Configuration Loading from File

**Requirement:** 05-REQ-7.1, 05-REQ-7.2
**Type:** unit
**Description:** The configuration loader correctly reads and parses a JSON config file, and falls back to embedded defaults when no file is specified.

**Preconditions:**
- A valid JSON config file exists at a temporary path.

**Input:**
- Call `LoadConfig(tempFilePath)` with a valid JSON config file.
- Call `LoadConfig("")` with no file path to test default fallback.

**Expected:**
- Config from file: parsed config matches file contents (correct number of zones, operators, settings).
- Default config: returns a valid config with at least 2 zones, 2 operators, and a proximity threshold of 500.

**Assertion pseudocode:**
```
// From file
cfg = LoadConfig(tempFilePath)
ASSERT cfg.Settings.ProximityThresholdMeters == 500
ASSERT len(cfg.Zones) >= 1
ASSERT len(cfg.Operators) >= 1

// Default
cfg = LoadConfig("")
ASSERT cfg.Settings.ProximityThresholdMeters == 500
ASSERT len(cfg.Zones) >= 2
ASSERT len(cfg.Operators) >= 2
```

## Edge Case Tests

### TS-05-E1: Adapter Metadata Returns 404 for Unknown Operator

**Requirement:** 05-REQ-4.E1, 05-REQ-8.3
**Type:** integration
**Description:** Requesting adapter metadata for a non-existent operator ID returns 404.

**Preconditions:**
- Server is initialized with default demo configuration.

**Input:**
- `GET /operators/nonexistent-operator/adapter`

**Expected:**
- HTTP status 404
- `Content-Type: application/json`
- JSON body contains `error` field with a descriptive message

**Assertion pseudocode:**
```
response = httptest.GET("/operators/nonexistent-operator/adapter")
ASSERT response.status == 404
ASSERT response.header("Content-Type") contains "application/json"
body = parseJSON(response.body)
ASSERT body.error != ""
```

### TS-05-E2: Invalid Latitude Returns 400

**Requirement:** 05-REQ-1.E1, 05-REQ-8.3
**Type:** integration
**Description:** Sending a request with invalid latitude values returns 400.

**Preconditions:**
- Server is initialized.

**Input (table-driven):**

| Sub-case | lat | lon | Reason |
|----------|-----|-----|--------|
| E2a | *(missing)* | 11.58 | Missing lat parameter |
| E2b | `"abc"` | 11.58 | Non-numeric lat |
| E2c | 91.0 | 11.58 | lat > 90 |
| E2d | -91.0 | 11.58 | lat < -90 |

**Expected:**
- HTTP status 400 for each sub-case
- `Content-Type: application/json`
- JSON body contains `error` field

**Assertion pseudocode:**
```
FOR EACH (lat, lon, reason) IN test_cases:
    response = httptest.GET("/operators?lat={lat}&lon={lon}")
    ASSERT response.status == 400
    ASSERT response.header("Content-Type") contains "application/json"
    body = parseJSON(response.body)
    ASSERT body.error != ""
```

### TS-05-E3: Invalid Longitude Returns 400

**Requirement:** 05-REQ-1.E2, 05-REQ-8.3
**Type:** integration
**Description:** Sending a request with invalid longitude values returns 400.

**Preconditions:**
- Server is initialized.

**Input (table-driven):**

| Sub-case | lat | lon | Reason |
|----------|-----|-----|--------|
| E3a | 48.14 | *(missing)* | Missing lon parameter |
| E3b | 48.14 | `"xyz"` | Non-numeric lon |
| E3c | 48.14 | 181.0 | lon > 180 |
| E3d | 48.14 | -181.0 | lon < -180 |

**Expected:**
- HTTP status 400 for each sub-case
- `Content-Type: application/json`
- JSON body contains `error` field

**Assertion pseudocode:**
```
FOR EACH (lat, lon, reason) IN test_cases:
    response = httptest.GET("/operators?lat={lat}&lon={lon}")
    ASSERT response.status == 400
    ASSERT response.header("Content-Type") contains "application/json"
    body = parseJSON(response.body)
    ASSERT body.error != ""
```

### TS-05-E4: Undefined Route Returns 404

**Requirement:** 05-REQ-8.E1
**Type:** integration
**Description:** Requesting an undefined path returns 404 with a JSON error body.

**Preconditions:**
- Server is initialized.

**Input:**
- `GET /nonexistent-path`

**Expected:**
- HTTP status 404
- `Content-Type: application/json`
- JSON body contains `error` field

**Assertion pseudocode:**
```
response = httptest.GET("/nonexistent-path")
ASSERT response.status == 404
ASSERT response.header("Content-Type") contains "application/json"
body = parseJSON(response.body)
ASSERT body.error != ""
```

### TS-05-E5: Invalid Config File Causes Exit Error

**Requirement:** 05-REQ-7.E1
**Type:** unit
**Description:** Attempting to load a non-existent or malformed config file returns an error.

**Preconditions:**
- No file exists at the specified path, or the file contains invalid JSON.

**Input:**
- Call `LoadConfig("/nonexistent/path.json")` -- file does not exist.
- Call `LoadConfig(tempFileWithInvalidJSON)` -- file contains `{invalid`.

**Expected:**
- Both calls return a non-nil error.

**Assertion pseudocode:**
```
_, err = LoadConfig("/nonexistent/path.json")
ASSERT err != nil

_, err = LoadConfig(tempFileWithInvalidJSON)
ASSERT err != nil
```

## Property Test Cases

### TS-05-P1: Geofence Point-in-Polygon Correctness

**Property:** Property 1 from design.md
**Validates:** 05-REQ-2.1, 05-REQ-2.2
**Type:** property
**Description:** Verifies the correctness of the point-in-polygon algorithm using known geometric properties.

**For any:** Convex polygon defined by known vertices in the Munich area.
**Invariant:**
1. Every vertex of the polygon is classified as inside or on-boundary (match = true).
2. The centroid of any convex polygon is classified as inside (match = true).
3. A point at (0, 0) is classified as outside for all Munich-area polygons.
4. For a rectangular polygon, the geometric center is inside.

**Assertion pseudocode:**
```
FOR EACH polygon IN demo_polygons:
    FOR EACH vertex IN polygon:
        ASSERT PointInOrNearPolygon(vertex, polygon, 1.0) == true
    centroid = computeCentroid(polygon)
    ASSERT PointInPolygon(centroid, polygon) == true
    ASSERT PointInPolygon({0, 0}, polygon) == false
```

### TS-05-P2: Proximity Threshold Matching

**Property:** Property 2 from design.md
**Validates:** 05-REQ-3.1, 05-REQ-3.E1
**Type:** property
**Description:** Points within the proximity threshold match; points beyond it do not.

**For any:** Point outside a known polygon at a computed distance.
**Invariant:** If point distance to polygon < threshold, match is true. If distance > threshold, match is false.

**Assertion pseudocode:**
```
polygon = demo_polygon_muc_central
// Point ~55m north of northern edge
nearPoint = {lat: 48.1425, lon: 11.5625}
ASSERT PointInOrNearPolygon(nearPoint, polygon, 500.0) == true

// Point ~50km away
farPoint = {lat: 48.5, lon: 11.5}
ASSERT PointInOrNearPolygon(farPoint, polygon, 500.0) == false
```

### TS-05-P3: Response Format Consistency

**Property:** Property 3 from design.md
**Validates:** 05-REQ-8.1, 05-REQ-8.2
**Type:** property
**Description:** Every endpoint response has `Content-Type: application/json` and valid JSON body.

**For any:** Request to any endpoint (success and error paths).
**Invariant:** Response has `Content-Type` containing `application/json` and body is valid JSON.

**Assertion pseudocode:**
```
endpoints = [
    "/health",
    "/operators?lat=48.14&lon=11.58",
    "/operators/muc-central/adapter",
    "/operators/unknown/adapter",
    "/operators",
    "/nonexistent-path"
]
FOR EACH endpoint IN endpoints:
    response = httptest.GET(endpoint)
    ASSERT response.header("Content-Type") contains "application/json"
    ASSERT isValidJSON(response.body)
```

### TS-05-P4: Operator-Adapter Integrity

**Property:** Property 4 from design.md
**Validates:** 05-REQ-4.1, 05-REQ-4.2
**Type:** property
**Description:** Every operator returned by location lookup has a corresponding valid adapter metadata entry.

**For any:** Operator returned by the location lookup endpoint.
**Invariant:** `GET /operators/{operator_id}/adapter` returns HTTP 200 with valid adapter metadata.

**Assertion pseudocode:**
```
response = httptest.GET("/operators?lat=48.1395&lon=11.5625")
operators = parseJSON(response.body)
FOR EACH op IN operators:
    adapterResp = httptest.GET("/operators/" + op.operator_id + "/adapter")
    ASSERT adapterResp.status == 200
    metadata = parseJSON(adapterResp.body)
    ASSERT metadata.image_ref != ""
    ASSERT metadata.checksum_sha256 matches "^sha256:[0-9a-f]{64}$"
    ASSERT metadata.version != ""
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 05-REQ-1.1 | TS-05-1 | integration |
| 05-REQ-1.2 | TS-05-3 | integration |
| 05-REQ-1.3 | TS-05-1 | integration |
| 05-REQ-1.E1 | TS-05-E2 | integration |
| 05-REQ-1.E2 | TS-05-E3 | integration |
| 05-REQ-2.1 | TS-05-P1 | property |
| 05-REQ-2.2 | TS-05-P1 | property |
| 05-REQ-2.E1 | TS-05-P1 | property |
| 05-REQ-3.1 | TS-05-2, TS-05-P2 | integration, property |
| 05-REQ-3.2 | TS-05-2, TS-05-7 | integration, unit |
| 05-REQ-3.E1 | TS-05-3, TS-05-P2 | integration, property |
| 05-REQ-4.1 | TS-05-4 | integration |
| 05-REQ-4.2 | TS-05-4, TS-05-P4 | integration, property |
| 05-REQ-4.E1 | TS-05-E1 | integration |
| 05-REQ-5.1 | TS-05-5 | integration |
| 05-REQ-5.E1 | TS-05-5 | integration |
| 05-REQ-6.1 | TS-05-6 | integration |
| 05-REQ-6.2 | TS-05-1, TS-05-6 | integration |
| 05-REQ-7.1 | TS-05-7 | unit |
| 05-REQ-7.2 | TS-05-7 | unit |
| 05-REQ-7.E1 | TS-05-E5 | unit |
| 05-REQ-8.1 | TS-05-P3 | property |
| 05-REQ-8.2 | TS-05-P3 | property |
| 05-REQ-8.3 | TS-05-E1, TS-05-E2, TS-05-E3 | integration |
| 05-REQ-8.E1 | TS-05-E4 | integration |
| 05-REQ-8.E2 | TS-05-P3 | property |
| Property 1 | TS-05-P1 | property |
| Property 2 | TS-05-P2 | property |
| Property 3 | TS-05-P3 | property |
| Property 4 | TS-05-P4 | property |
