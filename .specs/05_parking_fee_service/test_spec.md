# Test Specification: PARKING_FEE_SERVICE (Spec 05)

> Test specifications for the PARKING_FEE_SERVICE cloud REST API.
> Validates requirements from `.specs/05_parking_fee_service/requirements.md`.

## Test ID Convention

| Prefix  | Category           |
|---------|--------------------|
| TS-05-  | Functional tests   |
| TS-05-P | Property tests     |
| TS-05-E | Error/edge tests   |

## Test Environment

- **Test framework:** Go `testing` standard library
- **HTTP testing:** `net/http/httptest` for handler-level integration tests
- **Test location:** `backend/parking-fee-service/*_test.go`
- **Run command:** `cd backend/parking-fee-service && go test ./... -v`
- **Lint command:** `cd backend/parking-fee-service && go vet ./...`

## Functional Tests

### TS-05-1: Operator Lookup Returns Operators for Location Inside a Zone

**Requirement:** 05-REQ-1, 05-REQ-4

**Description:** Sending a `GET /operators` request with coordinates known to be inside the Munich Central Parking zone returns a non-empty array containing that operator.

**Preconditions:** Server is running with hardcoded demo data.

**Steps:**

1. Send `GET /operators?lat=48.1395&lon=11.5625` (inside muc-central zone).
2. Assert HTTP status is 200.
3. Assert `Content-Type` is `application/json`.
4. Parse JSON response as an array.
5. Assert array contains at least one entry with `operator_id` equal to `"muc-central"`.
6. Assert the entry contains `name`, `zone` (non-empty array), and `rate` fields.

**Expected result:** 200 OK with JSON array containing the `muc-central` operator.

### TS-05-2: Operator Lookup Returns Operators for Location Near a Zone (Fuzziness)

**Requirement:** 05-REQ-1, 05-REQ-4

**Description:** Sending a `GET /operators` request with coordinates slightly outside but within the buffer distance of a zone returns the nearby operator.

**Preconditions:** Server is running with hardcoded demo data. Buffer distance is 500 meters.

**Steps:**

1. Compute a test point approximately 200 meters outside the muc-central polygon boundary (e.g., lat=48.1425, lon=11.5625 -- slightly north of the northern edge at 48.1420).
2. Send `GET /operators?lat=48.1425&lon=11.5625`.
3. Assert HTTP status is 200.
4. Parse JSON response as an array.
5. Assert array contains at least one entry with `operator_id` equal to `"muc-central"`.

**Expected result:** 200 OK with JSON array containing the `muc-central` operator due to near-zone matching.

### TS-05-3: Operator Lookup Returns Empty List for Remote Location

**Requirement:** 05-REQ-1, 05-REQ-4

**Description:** Sending a `GET /operators` request with coordinates far from any zone returns an empty array.

**Preconditions:** Server is running with hardcoded demo data.

**Steps:**

1. Send `GET /operators?lat=52.5200&lon=13.4050` (Berlin, far from any Munich zone).
2. Assert HTTP status is 200.
3. Assert `Content-Type` is `application/json`.
4. Parse JSON response as an array.
5. Assert array is empty `[]`.

**Expected result:** 200 OK with empty JSON array.

### TS-05-4: Adapter Metadata Returns Correct Data for Valid Operator

**Requirement:** 05-REQ-2

**Description:** Requesting adapter metadata for a known operator returns the correct image reference, checksum, and version.

**Preconditions:** Server is running with hardcoded demo data.

**Steps:**

1. Send `GET /operators/muc-central/adapter`.
2. Assert HTTP status is 200.
3. Assert `Content-Type` is `application/json`.
4. Parse JSON response.
5. Assert `image_ref` is `"europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-central:v1.0.0"`.
6. Assert `checksum_sha256` starts with `"sha256:"` and has 64 hex characters after the prefix.
7. Assert `version` is `"v1.0.0"`.

**Expected result:** 200 OK with correct adapter metadata JSON.

### TS-05-5: Health Check Returns 200 OK

**Requirement:** 05-REQ-3

**Description:** The health endpoint returns a 200 status with `"status": "ok"`.

**Preconditions:** Server is running.

**Steps:**

1. Send `GET /health`.
2. Assert HTTP status is 200.
3. Assert `Content-Type` is `application/json`.
4. Parse JSON response.
5. Assert `status` field equals `"ok"`.

**Expected result:** 200 OK with `{"status": "ok"}`.

## Error and Edge Case Tests

### TS-05-E1: Adapter Metadata Returns 404 for Unknown Operator

**Requirement:** 05-REQ-2, 05-REQ-5

**Description:** Requesting adapter metadata for a non-existent operator ID returns 404.

**Steps:**

1. Send `GET /operators/nonexistent-operator/adapter`.
2. Assert HTTP status is 404.
3. Assert `Content-Type` is `application/json`.
4. Parse JSON response.
5. Assert `error` field is present and contains a descriptive message.

**Expected result:** 404 with JSON error body.

### TS-05-E2: Invalid Latitude Returns 400

**Requirement:** 05-REQ-1, 05-REQ-5

**Description:** Sending a request with invalid latitude values returns 400.

**Test cases (table-driven):**

| Sub-case | lat | lon | Reason |
|----------|-----|-----|--------|
| E2a | *(missing)* | 11.58 | Missing lat parameter |
| E2b | `"abc"` | 11.58 | Non-numeric lat |
| E2c | 91.0 | 11.58 | lat > 90 |
| E2d | -91.0 | 11.58 | lat < -90 |

**Steps (for each sub-case):**

1. Send `GET /operators` with the specified parameters.
2. Assert HTTP status is 400.
3. Assert `Content-Type` is `application/json`.
4. Assert response body contains `error` field with a descriptive message.

**Expected result:** 400 with JSON error body for each sub-case.

### TS-05-E3: Invalid Longitude Returns 400

**Requirement:** 05-REQ-1, 05-REQ-5

**Description:** Sending a request with invalid longitude values returns 400.

**Test cases (table-driven):**

| Sub-case | lat | lon | Reason |
|----------|-----|-----|--------|
| E3a | 48.14 | *(missing)* | Missing lon parameter |
| E3b | 48.14 | `"xyz"` | Non-numeric lon |
| E3c | 48.14 | 181.0 | lon > 180 |
| E3d | 48.14 | -181.0 | lon < -180 |

**Steps (for each sub-case):**

1. Send `GET /operators` with the specified parameters.
2. Assert HTTP status is 400.
3. Assert `Content-Type` is `application/json`.
4. Assert response body contains `error` field with a descriptive message.

**Expected result:** 400 with JSON error body for each sub-case.

### TS-05-E4: Undefined Route Returns 404

**Requirement:** 05-REQ-5

**Description:** Requesting an undefined path returns 404 with a JSON error body.

**Steps:**

1. Send `GET /nonexistent-path`.
2. Assert HTTP status is 404.
3. Assert `Content-Type` is `application/json`.
4. Assert response body contains `error` field.

**Expected result:** 404 with JSON error body.

## Property Tests

### TS-05-P1: Geofence Point-in-Polygon Correctness

**Requirement:** 05-REQ-4

**Description:** Verifies the correctness of the point-in-polygon algorithm using known geometric properties.

**Properties tested:**

1. **Vertex inclusion:** Every vertex of a polygon is classified as inside or on-boundary (match = true).
2. **Centroid inclusion:** The centroid of any convex polygon is classified as inside (match = true).
3. **Distant point exclusion:** A point at (0, 0) is classified as outside for all Munich-area polygons (match = false, assuming buffer distance does not reach the equator/prime meridian intersection).
4. **Symmetry:** For a rectangular polygon, a point at the geometric center is inside.

**Implementation:** Table-driven Go test with multiple polygons and test points.

**Expected result:** All property assertions hold for every test case.

### TS-05-P2: Response Format Consistency

**Requirement:** 05-REQ-6

**Description:** Every endpoint response has `Content-Type: application/json` and valid JSON body.

**Properties tested:**

1. For each endpoint (`/health`, `/operators?lat=48.14&lon=11.58`, `/operators/muc-central/adapter`, `/operators/unknown/adapter`, `/operators`), the response `Content-Type` header contains `application/json`.
2. For each response, the body is valid JSON (parseable without error).

**Implementation:** Iterate over a table of requests and assert header and JSON validity for each.

**Expected result:** All responses have correct Content-Type and valid JSON.

## Traceability

| Test ID   | Requirement(s)       | Category    |
|-----------|----------------------|-------------|
| TS-05-1   | 05-REQ-1, 05-REQ-4  | Functional  |
| TS-05-2   | 05-REQ-1, 05-REQ-4  | Functional  |
| TS-05-3   | 05-REQ-1, 05-REQ-4  | Functional  |
| TS-05-4   | 05-REQ-2            | Functional  |
| TS-05-5   | 05-REQ-3            | Functional  |
| TS-05-E1  | 05-REQ-2, 05-REQ-5  | Error/Edge  |
| TS-05-E2  | 05-REQ-1, 05-REQ-5  | Error/Edge  |
| TS-05-E3  | 05-REQ-1, 05-REQ-5  | Error/Edge  |
| TS-05-E4  | 05-REQ-5            | Error/Edge  |
| TS-05-P1  | 05-REQ-4            | Property    |
| TS-05-P2  | 05-REQ-6            | Property    |
