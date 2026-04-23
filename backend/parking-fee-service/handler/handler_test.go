package handler_test

import (
	"encoding/json"
	"fmt"
	"math/rand"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/handler"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// testSetup creates a store and mux with default Munich demo data for testing.
func testSetup() (*store.Store, *http.ServeMux) {
	zones := []model.Zone{
		{
			ID:   "munich-central",
			Name: "Munich Central Station Area",
			Polygon: []model.Coordinate{
				{Lat: 48.1400, Lon: 11.5550},
				{Lat: 48.1400, Lon: 11.5650},
				{Lat: 48.1350, Lon: 11.5650},
				{Lat: 48.1350, Lon: 11.5550},
			},
		},
		{
			ID:   "munich-marienplatz",
			Name: "Marienplatz Area",
			Polygon: []model.Coordinate{
				{Lat: 48.1380, Lon: 11.5730},
				{Lat: 48.1380, Lon: 11.5790},
				{Lat: 48.1350, Lon: 11.5790},
				{Lat: 48.1350, Lon: 11.5730},
			},
		},
	}

	operators := []model.Operator{
		{
			ID:     "parkhaus-munich",
			Name:   "Parkhaus Muenchen GmbH",
			ZoneID: "munich-central",
			Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
				ChecksumSHA256: "sha256:abc123def456",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "city-park-munich",
			Name:   "CityPark Muenchen",
			ZoneID: "munich-marienplatz",
			Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "us-docker.pkg.dev/sdv-demo/adapters/citypark-munich:v1.0.0",
				ChecksumSHA256: "sha256:789ghi012jkl",
				Version:        "1.0.0",
			},
		},
	}

	s := store.NewStore(zones, operators)
	threshold := 500.0

	mux := http.NewServeMux()
	mux.HandleFunc("GET /operators", handler.NewOperatorHandler(s, zones, threshold))
	mux.HandleFunc("GET /operators/{id}/adapter", handler.NewAdapterHandler(s))
	mux.HandleFunc("GET /health", handler.HealthHandler())

	return s, mux
}

// TS-05-1: Operator lookup returns matching operators.
func TestOperatorLookup(t *testing.T) {
	_, mux := testSetup()

	req := httptest.NewRequest("GET", "/operators?lat=48.1375&lon=11.5600", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("GET /operators?lat=48.1375&lon=11.5600 status = %d, want 200", rec.Code)
	}

	var body []map[string]interface{}
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(body) < 1 {
		t.Fatal("expected at least 1 operator in response, got 0")
	}

	found := false
	for _, op := range body {
		if op["id"] == "parkhaus-munich" && op["zone_id"] == "munich-central" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected operator 'parkhaus-munich' with zone_id 'munich-central' in response")
	}
}

// TS-05-5: Empty array for no matches.
func TestEmptyArrayNoMatches(t *testing.T) {
	_, mux := testSetup()

	req := httptest.NewRequest("GET", "/operators?lat=0.0&lon=0.0", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("GET /operators?lat=0.0&lon=0.0 status = %d, want 200", rec.Code)
	}

	var body []map[string]interface{}
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(body) != 0 {
		t.Errorf("expected empty array, got %d operators", len(body))
	}
}

// TS-05-6: Adapter metadata retrieval returns image_ref, checksum_sha256, and version.
func TestAdapterMetadataRetrieval(t *testing.T) {
	_, mux := testSetup()

	req := httptest.NewRequest("GET", "/operators/parkhaus-munich/adapter", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("GET /operators/parkhaus-munich/adapter status = %d, want 200", rec.Code)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if body["image_ref"] == nil || body["image_ref"] == "" {
		t.Error("image_ref is missing or empty")
	}
	if body["checksum_sha256"] == nil || body["checksum_sha256"] == "" {
		t.Error("checksum_sha256 is missing or empty")
	}
	if body["version"] == nil || body["version"] == "" {
		t.Error("version is missing or empty")
	}
}

// TS-05-7: Adapter metadata HTTP 200.
func TestAdapterMetadataHTTP200(t *testing.T) {
	_, mux := testSetup()

	req := httptest.NewRequest("GET", "/operators/parkhaus-munich/adapter", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("GET /operators/parkhaus-munich/adapter status = %d, want 200", rec.Code)
	}
}

// TS-05-8: Health check returns HTTP 200 with {"status":"ok"}.
func TestHealthCheck(t *testing.T) {
	_, mux := testSetup()

	req := httptest.NewRequest("GET", "/health", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("GET /health status = %d, want 200", rec.Code)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode health response: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("health status = %v, want 'ok'", body["status"])
	}
}

// TS-05-12: Content-Type header is application/json on all responses.
func TestContentTypeHeader(t *testing.T) {
	_, mux := testSetup()

	endpoints := []string{
		"/health",
		"/operators?lat=48.137&lon=11.575",
		"/operators/parkhaus-munich/adapter",
	}

	for _, endpoint := range endpoints {
		t.Run(endpoint, func(t *testing.T) {
			req := httptest.NewRequest("GET", endpoint, nil)
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)

			ct := rec.Header().Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("Content-Type = %q, want 'application/json'", ct)
			}
		})
	}
}

// TestContentTypeHeaderOnErrors verifies Content-Type: application/json is set
// on error responses (400, 404) as required by 05-REQ-5.1 ("ALL responses").
func TestContentTypeHeaderOnErrors(t *testing.T) {
	_, mux := testSetup()

	cases := []struct {
		name       string
		url        string
		wantStatus int
	}{
		{"missing params (400)", "/operators", http.StatusBadRequest},
		{"invalid coordinates (400)", "/operators?lat=999&lon=999", http.StatusBadRequest},
		{"non-numeric coords (400)", "/operators?lat=abc&lon=def", http.StatusBadRequest},
		{"unknown operator (404)", "/operators/nonexistent/adapter", http.StatusNotFound},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", tc.url, nil)
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)

			if rec.Code != tc.wantStatus {
				t.Fatalf("status = %d, want %d", rec.Code, tc.wantStatus)
			}

			ct := rec.Header().Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("Content-Type = %q, want 'application/json'", ct)
			}
		})
	}
}

// TS-05-13: Operator lookup response includes correct fields and excludes adapter.
func TestOperatorResponseFields(t *testing.T) {
	_, mux := testSetup()

	req := httptest.NewRequest("GET", "/operators?lat=48.1375&lon=11.5600", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200", rec.Code)
	}

	var body []map[string]interface{}
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(body) < 1 {
		t.Fatal("expected at least 1 operator")
	}

	op := body[0]
	if op["id"] == nil || op["id"] == "" {
		t.Error("id is missing or empty")
	}
	if op["name"] == nil || op["name"] == "" {
		t.Error("name is missing or empty")
	}
	if op["zone_id"] == nil || op["zone_id"] == "" {
		t.Error("zone_id is missing or empty")
	}

	rate, ok := op["rate"].(map[string]interface{})
	if !ok {
		t.Fatal("rate field is missing or not an object")
	}
	rateType, _ := rate["type"].(string)
	if rateType != "per-hour" && rateType != "flat-fee" {
		t.Errorf("rate.type = %q, want 'per-hour' or 'flat-fee'", rateType)
	}
	if rate["amount"] == nil {
		t.Error("rate.amount is missing")
	}
	if amount, ok := rate["amount"].(float64); !ok || amount <= 0 {
		t.Errorf("rate.amount = %v, want > 0", rate["amount"])
	}
	if rate["currency"] != "EUR" {
		t.Errorf("rate.currency = %v, want 'EUR'", rate["currency"])
	}

	// Adapter field must NOT be present
	if _, exists := op["adapter"]; exists {
		t.Error("adapter field should NOT be present in operator lookup response")
	}
}

// TS-05-14: Error response format uses {"error":"<message>"}.
func TestErrorResponseFormat(t *testing.T) {
	_, mux := testSetup()

	req := httptest.NewRequest("GET", "/operators", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Fatalf("GET /operators (no params) status = %d, want 400", rec.Code)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if body["error"] == nil || body["error"] == "" {
		t.Error("error field is missing or empty in error response")
	}
}

// TS-05-E1: Missing lat/lon query parameters return HTTP 400.
func TestMissingLatLon(t *testing.T) {
	_, mux := testSetup()

	cases := []struct {
		name string
		url  string
	}{
		{"no params", "/operators"},
		{"missing lon", "/operators?lat=48.137"},
		{"missing lat", "/operators?lon=11.575"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", tc.url, nil)
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)

			if rec.Code != http.StatusBadRequest {
				t.Errorf("GET %s status = %d, want 400", tc.url, rec.Code)
			}

			var body map[string]interface{}
			if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}
			if body["error"] != "lat and lon query parameters are required" {
				t.Errorf("error = %v, want 'lat and lon query parameters are required'", body["error"])
			}
		})
	}
}

// TS-05-E2: Invalid coordinate range returns HTTP 400.
func TestInvalidCoordinateRange(t *testing.T) {
	_, mux := testSetup()

	cases := []struct {
		name string
		lat  string
		lon  string
	}{
		{"lat > 90", "91.0", "11.575"},
		{"lat < -90", "-91.0", "11.575"},
		{"lon > 180", "48.137", "181.0"},
		{"lon < -180", "48.137", "-181.0"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			url := fmt.Sprintf("/operators?lat=%s&lon=%s", tc.lat, tc.lon)
			req := httptest.NewRequest("GET", url, nil)
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)

			if rec.Code != http.StatusBadRequest {
				t.Errorf("GET %s status = %d, want 400", url, rec.Code)
			}

			var body map[string]interface{}
			if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}
			if body["error"] != "invalid coordinates" {
				t.Errorf("error = %v, want 'invalid coordinates'", body["error"])
			}
		})
	}
}

// TS-05-E3: Non-numeric lat/lon values return HTTP 400.
func TestNonNumericCoordinates(t *testing.T) {
	_, mux := testSetup()

	cases := []struct {
		name string
		lat  string
		lon  string
	}{
		{"lat is text", "abc", "11.575"},
		{"lon is text", "48.137", "xyz"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			url := fmt.Sprintf("/operators?lat=%s&lon=%s", tc.lat, tc.lon)
			req := httptest.NewRequest("GET", url, nil)
			rec := httptest.NewRecorder()
			mux.ServeHTTP(rec, req)

			if rec.Code != http.StatusBadRequest {
				t.Errorf("GET %s status = %d, want 400", url, rec.Code)
			}

			var body map[string]interface{}
			if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode response: %v", err)
			}
			if body["error"] != "invalid coordinates" {
				t.Errorf("error = %v, want 'invalid coordinates'", body["error"])
			}
		})
	}
}

// TS-05-E4: Unknown operator ID returns HTTP 404.
func TestUnknownOperatorID(t *testing.T) {
	_, mux := testSetup()

	req := httptest.NewRequest("GET", "/operators/nonexistent-operator/adapter", nil)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Fatalf("GET /operators/nonexistent-operator/adapter status = %d, want 404", rec.Code)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if body["error"] != "operator not found" {
		t.Errorf("error = %v, want 'operator not found'", body["error"])
	}
}

// TS-05-P4: Property test for coordinate validation.
// For any latitude outside [-90,90] or longitude outside [-180,180], the handler returns HTTP 400.
func TestPropertyCoordinateValidation(t *testing.T) {
	_, mux := testSetup()

	rng := rand.New(rand.NewSource(42))

	for i := 0; i < 100; i++ {
		lat := (rng.Float64()*400 - 200) // range [-200, 200]
		lon := (rng.Float64()*800 - 400) // range [-400, 400]

		url := fmt.Sprintf("/operators?lat=%f&lon=%f", lat, lon)
		req := httptest.NewRequest("GET", url, nil)
		rec := httptest.NewRecorder()
		mux.ServeHTTP(rec, req)

		outOfRange := lat < -90 || lat > 90 || lon < -180 || lon > 180
		if outOfRange && rec.Code != http.StatusBadRequest {
			t.Errorf("iteration %d: lat=%f lon=%f status = %d, want 400 (out of range)",
				i, lat, lon, rec.Code)
		}
		if !outOfRange && rec.Code != http.StatusOK {
			t.Errorf("iteration %d: lat=%f lon=%f status = %d, want 200 (in range)",
				i, lat, lon, rec.Code)
		}
	}
}
