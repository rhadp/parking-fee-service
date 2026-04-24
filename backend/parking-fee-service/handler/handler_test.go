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

// Default test data matching the Munich demo config.
var testZones = []model.Zone{
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

var testOperators = []model.Operator{
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

// setupTestServer creates an httptest.Server with all routes registered.
func setupTestServer() *httptest.Server {
	s := store.NewStore(testZones, testOperators)

	mux := http.NewServeMux()
	mux.HandleFunc("GET /operators", handler.NewOperatorHandler(s, testZones, 500.0))
	mux.HandleFunc("GET /operators/{id}/adapter", handler.NewAdapterHandler(s))
	mux.HandleFunc("GET /health", handler.HealthHandler())

	return httptest.NewServer(mux)
}

// ---------------------------------------------------------------------------
// TS-05-1: Operator Lookup Returns Matching Operators
// ---------------------------------------------------------------------------

func TestOperatorLookup(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators?lat=48.1375&lon=11.5600")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}

	var operators []map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&operators); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(operators) < 1 {
		t.Fatal("expected at least 1 operator in response")
	}
	if operators[0]["id"] != "parkhaus-munich" {
		t.Errorf("operator id = %v, want \"parkhaus-munich\"", operators[0]["id"])
	}
	if operators[0]["zone_id"] != "munich-central" {
		t.Errorf("operator zone_id = %v, want \"munich-central\"", operators[0]["zone_id"])
	}
}

// ---------------------------------------------------------------------------
// TS-05-5: Empty Array for No Matches
// ---------------------------------------------------------------------------

func TestEmptyArrayNoMatches(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators?lat=0.0&lon=0.0")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}

	var operators []interface{}
	if err := json.NewDecoder(resp.Body).Decode(&operators); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(operators) != 0 {
		t.Errorf("expected 0 operators, got %d", len(operators))
	}
}

// ---------------------------------------------------------------------------
// TS-05-6: Adapter Metadata Retrieval
// ---------------------------------------------------------------------------

func TestAdapterMetadataRetrieval(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/parkhaus-munich/adapter")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}

	var adapter map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&adapter); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if v, ok := adapter["image_ref"].(string); !ok || v == "" {
		t.Error("image_ref is missing or empty")
	}
	if v, ok := adapter["checksum_sha256"].(string); !ok || v == "" {
		t.Error("checksum_sha256 is missing or empty")
	}
	if v, ok := adapter["version"].(string); !ok || v == "" {
		t.Error("version is missing or empty")
	}
}

// ---------------------------------------------------------------------------
// TS-05-7: Adapter Metadata HTTP 200
// ---------------------------------------------------------------------------

func TestAdapterMetadataHTTP200(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/parkhaus-munich/adapter")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}
}

// ---------------------------------------------------------------------------
// TS-05-8: Health Check
// ---------------------------------------------------------------------------

func TestHealthCheck(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/health")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("status = %q, want \"ok\"", body["status"])
	}
}

// ---------------------------------------------------------------------------
// TS-05-12: Content-Type Header
// ---------------------------------------------------------------------------

func TestContentTypeHeader(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	endpoints := []string{
		"/health",
		"/operators?lat=48.137&lon=11.575",
		"/operators/parkhaus-munich/adapter",
	}

	for _, ep := range endpoints {
		t.Run(ep, func(t *testing.T) {
			resp, err := http.Get(srv.URL + ep)
			if err != nil {
				t.Fatal(err)
			}
			defer resp.Body.Close()

			ct := resp.Header.Get("Content-Type")
			if ct != "application/json" {
				t.Errorf("Content-Type = %q, want \"application/json\"", ct)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-05-13: Operator Lookup Response Fields
// ---------------------------------------------------------------------------

func TestOperatorResponseFields(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators?lat=48.1375&lon=11.5600")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	var operators []map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&operators); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if len(operators) < 1 {
		t.Fatal("expected at least 1 operator")
	}

	op := operators[0]

	// Required fields present and non-empty.
	if v, ok := op["id"].(string); !ok || v == "" {
		t.Error("id is missing or empty")
	}
	if v, ok := op["name"].(string); !ok || v == "" {
		t.Error("name is missing or empty")
	}
	if v, ok := op["zone_id"].(string); !ok || v == "" {
		t.Error("zone_id is missing or empty")
	}

	// Rate sub-object.
	rate, ok := op["rate"].(map[string]interface{})
	if !ok {
		t.Fatal("rate is missing or not an object")
	}
	rateType, ok := rate["type"].(string)
	if !ok || (rateType != "per-hour" && rateType != "flat-fee") {
		t.Errorf("rate.type = %v, want \"per-hour\" or \"flat-fee\"", rate["type"])
	}
	amount, ok := rate["amount"].(float64)
	if !ok || amount <= 0 {
		t.Errorf("rate.amount = %v, want > 0", rate["amount"])
	}
	currency, ok := rate["currency"].(string)
	if !ok || currency != "EUR" {
		t.Errorf("rate.currency = %v, want \"EUR\"", rate["currency"])
	}

	// The adapter field MUST NOT be present in the lookup response.
	if _, exists := op["adapter"]; exists {
		t.Error("adapter field should NOT be present in operator lookup response")
	}
}

// ---------------------------------------------------------------------------
// TS-05-14: Error Response Format
// ---------------------------------------------------------------------------

func TestErrorResponseFormat(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 400 {
		t.Errorf("status = %d, want 400", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if body["error"] == "" {
		t.Error("error field should not be empty")
	}
}

// ---------------------------------------------------------------------------
// TS-05-E1: Missing lat/lon Parameters
// ---------------------------------------------------------------------------

func TestMissingLatLon(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	urls := []string{
		"/operators",
		"/operators?lat=48.137",
		"/operators?lon=11.575",
	}

	for _, u := range urls {
		t.Run(u, func(t *testing.T) {
			resp, err := http.Get(srv.URL + u)
			if err != nil {
				t.Fatal(err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != 400 {
				t.Errorf("status = %d, want 400", resp.StatusCode)
			}

			var body map[string]string
			if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode error response: %v", err)
			}
			if body["error"] != "lat and lon query parameters are required" {
				t.Errorf("error = %q, want %q",
					body["error"], "lat and lon query parameters are required")
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-05-E2: Invalid Coordinate Range
// ---------------------------------------------------------------------------

func TestInvalidCoordinateRange(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	cases := []struct {
		lat string
		lon string
	}{
		{"91.0", "11.575"},
		{"-91.0", "11.575"},
		{"48.137", "181.0"},
		{"48.137", "-181.0"},
	}

	for _, c := range cases {
		t.Run(fmt.Sprintf("lat=%s&lon=%s", c.lat, c.lon), func(t *testing.T) {
			resp, err := http.Get(srv.URL + "/operators?lat=" + c.lat + "&lon=" + c.lon)
			if err != nil {
				t.Fatal(err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != 400 {
				t.Errorf("status = %d, want 400", resp.StatusCode)
			}

			var body map[string]string
			if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode error response: %v", err)
			}
			if body["error"] != "invalid coordinates" {
				t.Errorf("error = %q, want %q", body["error"], "invalid coordinates")
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-05-E3: Non-Numeric Coordinates
// ---------------------------------------------------------------------------

func TestNonNumericCoordinates(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	cases := []struct {
		lat string
		lon string
	}{
		{"abc", "11.575"},
		{"48.137", "xyz"},
	}

	for _, c := range cases {
		t.Run(fmt.Sprintf("lat=%s&lon=%s", c.lat, c.lon), func(t *testing.T) {
			resp, err := http.Get(srv.URL + "/operators?lat=" + c.lat + "&lon=" + c.lon)
			if err != nil {
				t.Fatal(err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != 400 {
				t.Errorf("status = %d, want 400", resp.StatusCode)
			}

			var body map[string]string
			if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
				t.Fatalf("failed to decode error response: %v", err)
			}
			if body["error"] != "invalid coordinates" {
				t.Errorf("error = %q, want %q", body["error"], "invalid coordinates")
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-05-E4: Unknown Operator ID
// ---------------------------------------------------------------------------

func TestUnknownOperatorID(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/operators/nonexistent-operator/adapter")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 404 {
		t.Errorf("status = %d, want 404", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if body["error"] != "operator not found" {
		t.Errorf("error = %q, want %q", body["error"], "operator not found")
	}
}

// ---------------------------------------------------------------------------
// TS-05-P4: Property – Coordinate Validation
// ---------------------------------------------------------------------------

// TestPropertyCoordinateValidation verifies that out-of-range coordinates
// always produce HTTP 400.
func TestPropertyCoordinateValidation(t *testing.T) {
	srv := setupTestServer()
	defer srv.Close()

	rng := rand.New(rand.NewSource(42))

	// Generate random out-of-range coordinates.
	for i := 0; i < 20; i++ {
		var lat, lon float64
		outOfRange := false

		// Randomly decide which parameter is out of range.
		switch rng.Intn(4) {
		case 0: // lat > 90
			lat = 90.0 + rng.Float64()*100
			lon = rng.Float64()*360 - 180
			outOfRange = true
		case 1: // lat < -90
			lat = -90.0 - rng.Float64()*100
			lon = rng.Float64()*360 - 180
			outOfRange = true
		case 2: // lon > 180
			lat = rng.Float64()*180 - 90
			lon = 180.0 + rng.Float64()*100
			outOfRange = true
		case 3: // lon < -180
			lat = rng.Float64()*180 - 90
			lon = -180.0 - rng.Float64()*100
			outOfRange = true
		}

		if !outOfRange {
			continue
		}

		url := fmt.Sprintf("/operators?lat=%f&lon=%f", lat, lon)
		t.Run(fmt.Sprintf("case_%d", i), func(t *testing.T) {
			resp, err := http.Get(srv.URL + url)
			if err != nil {
				t.Fatal(err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != 400 {
				t.Errorf("GET %s: status = %d, want 400 (lat=%.2f, lon=%.2f)",
					url, resp.StatusCode, lat, lon)
			}
		})
	}
}
