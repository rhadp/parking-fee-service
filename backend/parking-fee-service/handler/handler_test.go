// Package handler_test contains integration tests for HTTP handlers.
package handler_test

import (
	"encoding/json"
	"fmt"
	"io"
	"math/rand"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/config"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/handler"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// newTestServer builds a test HTTP server wired with the default Munich config.
func newTestServer(t *testing.T) *httptest.Server {
	t.Helper()
	cfg := config.DefaultConfig()
	st := store.NewStore(cfg.Zones, cfg.Operators)
	mux := http.NewServeMux()
	mux.Handle("GET /operators", handler.NewOperatorHandler(st, cfg.Zones, cfg.ProximityThreshold))
	mux.Handle("GET /operators/{id}/adapter", handler.NewAdapterHandler(st))
	mux.Handle("GET /health", handler.HealthHandler())
	return httptest.NewServer(mux)
}

// get performs a GET request against the test server and returns the response.
func get(t *testing.T, srv *httptest.Server, path string) *http.Response {
	t.Helper()
	resp, err := http.Get(srv.URL + path)
	if err != nil {
		t.Fatalf("GET %s failed: %v", path, err)
	}
	return resp
}

// decodeJSON parses the response body as JSON into v.
func decodeJSON(t *testing.T, resp *http.Response, v any) {
	t.Helper()
	defer resp.Body.Close()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("failed to read response body: %v", err)
	}
	if err := json.Unmarshal(body, v); err != nil {
		t.Fatalf("failed to decode JSON %q: %v", string(body), err)
	}
}

// TestOperatorLookup verifies that a coordinate inside the munich-central zone
// returns the corresponding operator.
// TS-05-1
func TestOperatorLookup(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp := get(t, srv, "/operators?lat=48.1375&lon=11.5600")
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}

	var operators []map[string]any
	decodeJSON(t, resp, &operators)
	if len(operators) < 1 {
		t.Fatalf("expected at least 1 operator, got %d", len(operators))
	}

	// Find parkhaus-munich in the response.
	found := false
	for _, op := range operators {
		if op["id"] == "parkhaus-munich" {
			found = true
			if op["zone_id"] != "munich-central" {
				t.Errorf("expected zone_id=munich-central, got %v", op["zone_id"])
			}
		}
	}
	if !found {
		t.Errorf("expected parkhaus-munich in operator list, got %v", operators)
	}
}

// TestEmptyArrayNoMatches verifies that coordinates far from all zones return
// an empty JSON array with HTTP 200.
// TS-05-5
func TestEmptyArrayNoMatches(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp := get(t, srv, "/operators?lat=0.0&lon=0.0")
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}

	var operators []map[string]any
	decodeJSON(t, resp, &operators)
	if len(operators) != 0 {
		t.Errorf("expected empty array for coordinates far from zones, got %v", operators)
	}
}

// TestAdapterMetadataRetrieval verifies that GET /operators/{id}/adapter returns
// all required adapter fields.
// TS-05-6
func TestAdapterMetadataRetrieval(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp := get(t, srv, "/operators/parkhaus-munich/adapter")
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}

	var adapter map[string]any
	decodeJSON(t, resp, &adapter)

	if adapter["image_ref"] == "" || adapter["image_ref"] == nil {
		t.Errorf("expected non-empty image_ref, got %v", adapter["image_ref"])
	}
	if adapter["checksum_sha256"] == "" || adapter["checksum_sha256"] == nil {
		t.Errorf("expected non-empty checksum_sha256, got %v", adapter["checksum_sha256"])
	}
	if adapter["version"] == "" || adapter["version"] == nil {
		t.Errorf("expected non-empty version, got %v", adapter["version"])
	}
}

// TestAdapterMetadataHTTP200 verifies that a successful adapter request returns HTTP 200.
// TS-05-7
func TestAdapterMetadataHTTP200(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp := get(t, srv, "/operators/parkhaus-munich/adapter")
	resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected HTTP 200 for adapter lookup, got %d", resp.StatusCode)
	}
}

// TestHealthCheck verifies that GET /health returns HTTP 200 with {"status":"ok"}.
// TS-05-8
func TestHealthCheck(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp := get(t, srv, "/health")
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}

	var body map[string]string
	decodeJSON(t, resp, &body)
	if body["status"] != "ok" {
		t.Errorf(`expected {"status":"ok"}, got %v`, body)
	}
}

// TestContentTypeHeader verifies that all endpoints set Content-Type: application/json.
// TS-05-12
// Addresses Skeptic minor finding: uses HasPrefix to tolerate charset suffixes.
func TestContentTypeHeader(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	endpoints := []string{
		"/health",
		"/operators?lat=48.137&lon=11.575",
		"/operators/parkhaus-munich/adapter",
	}

	for _, path := range endpoints {
		t.Run(path, func(t *testing.T) {
			resp := get(t, srv, path)
			resp.Body.Close()
			ct := resp.Header.Get("Content-Type")
			if !strings.HasPrefix(ct, "application/json") {
				t.Errorf("endpoint %s: expected Content-Type application/json, got %q", path, ct)
			}
		})
	}
}

// TestOperatorResponseFields verifies that the operator lookup response includes
// id, name, zone_id, and rate fields, but does NOT include the adapter field.
// TS-05-13
func TestOperatorResponseFields(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp := get(t, srv, "/operators?lat=48.1375&lon=11.5600")
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}

	var operators []map[string]any
	decodeJSON(t, resp, &operators)
	if len(operators) < 1 {
		t.Fatalf("expected at least 1 operator in response")
	}

	op := operators[0]

	// Required fields must be present and non-empty.
	if v, ok := op["id"]; !ok || v == "" {
		t.Errorf("operator missing or empty id field: %v", op)
	}
	if v, ok := op["name"]; !ok || v == "" {
		t.Errorf("operator missing or empty name field: %v", op)
	}
	if v, ok := op["zone_id"]; !ok || v == "" {
		t.Errorf("operator missing or empty zone_id field: %v", op)
	}
	rate, ok := op["rate"].(map[string]any)
	if !ok {
		t.Fatalf("operator missing rate object: %v", op)
	}
	rateType, _ := rate["type"].(string)
	if rateType != "per-hour" && rateType != "flat-fee" {
		t.Errorf("rate.type must be per-hour or flat-fee, got %q", rateType)
	}
	amount, _ := rate["amount"].(float64)
	if amount <= 0 {
		t.Errorf("rate.amount must be > 0, got %v", rate["amount"])
	}
	currency, _ := rate["currency"].(string)
	if currency != "EUR" {
		t.Errorf("rate.currency must be EUR, got %q", currency)
	}

	// The adapter field must NOT be present in lookup responses.
	if _, hasAdapter := op["adapter"]; hasAdapter {
		t.Errorf("operator lookup response must NOT include adapter field: %v", op)
	}
}

// TestErrorResponseFormat verifies that error responses use {"error":"<message>"} format.
// TS-05-14
func TestErrorResponseFormat(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp := get(t, srv, "/operators") // missing lat/lon
	if resp.StatusCode != http.StatusBadRequest {
		t.Fatalf("expected 400, got %d", resp.StatusCode)
	}

	var errBody map[string]string
	decodeJSON(t, resp, &errBody)
	if errMsg, ok := errBody["error"]; !ok || errMsg == "" {
		t.Errorf("expected error field in response, got %v", errBody)
	}
}

// TestMissingLatLon verifies that missing lat or lon query parameters return HTTP 400.
// TS-05-E1
func TestMissingLatLon(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	cases := []struct {
		path string
	}{
		{"/operators"},
		{"/operators?lat=48.137"},
		{"/operators?lon=11.575"},
	}

	for _, tc := range cases {
		t.Run(tc.path, func(t *testing.T) {
			resp := get(t, srv, tc.path)
			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("path %s: expected 400, got %d", tc.path, resp.StatusCode)
			}
			var errBody map[string]string
			decodeJSON(t, resp, &errBody)
			want := "lat and lon query parameters are required"
			if errBody["error"] != want {
				t.Errorf("path %s: expected error %q, got %q", tc.path, want, errBody["error"])
			}
		})
	}
}

// TestInvalidCoordinateRange verifies that out-of-range lat/lon return HTTP 400.
// TS-05-E2
func TestInvalidCoordinateRange(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	cases := []struct {
		lat string
		lon string
	}{
		{"91.0", "11.575"},   // lat > 90
		{"-91.0", "11.575"},  // lat < -90
		{"48.137", "181.0"},  // lon > 180
		{"48.137", "-181.0"}, // lon < -180
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("lat=%s,lon=%s", tc.lat, tc.lon), func(t *testing.T) {
			path := fmt.Sprintf("/operators?lat=%s&lon=%s", tc.lat, tc.lon)
			resp := get(t, srv, path)
			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("expected 400 for lat=%s,lon=%s, got %d", tc.lat, tc.lon, resp.StatusCode)
			}
			var errBody map[string]string
			decodeJSON(t, resp, &errBody)
			if errBody["error"] != "invalid coordinates" {
				t.Errorf("expected error 'invalid coordinates', got %q", errBody["error"])
			}
		})
	}
}

// TestNonNumericCoordinates verifies that non-numeric lat/lon return HTTP 400.
// TS-05-E3
func TestNonNumericCoordinates(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	cases := []struct {
		lat string
		lon string
	}{
		{"abc", "11.575"},
		{"48.137", "xyz"},
		{"not-a-number", "not-a-number"},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("lat=%s,lon=%s", tc.lat, tc.lon), func(t *testing.T) {
			path := fmt.Sprintf("/operators?lat=%s&lon=%s", tc.lat, tc.lon)
			resp := get(t, srv, path)
			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("expected 400 for non-numeric lat=%s,lon=%s, got %d", tc.lat, tc.lon, resp.StatusCode)
			}
			var errBody map[string]string
			decodeJSON(t, resp, &errBody)
			if errBody["error"] != "invalid coordinates" {
				t.Errorf("expected error 'invalid coordinates', got %q", errBody["error"])
			}
		})
	}
}

// TestUnknownOperatorID verifies that an unknown operator ID returns HTTP 404.
// TS-05-E4
func TestUnknownOperatorID(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	resp := get(t, srv, "/operators/nonexistent-operator/adapter")
	if resp.StatusCode != http.StatusNotFound {
		t.Fatalf("expected 404 for unknown operator, got %d", resp.StatusCode)
	}

	var errBody map[string]string
	decodeJSON(t, resp, &errBody)
	if errBody["error"] != "operator not found" {
		t.Errorf("expected error 'operator not found', got %q", errBody["error"])
	}
}

// TestPropertyCoordinateValidation is a property test verifying that out-of-range
// coordinates always return HTTP 400.
// TS-05-P4
func TestPropertyCoordinateValidation(t *testing.T) {
	srv := newTestServer(t)
	defer srv.Close()

	// Fixed out-of-range cases as representative property samples.
	outOfRangeCases := []struct {
		lat float64
		lon float64
	}{
		{lat: 90.001, lon: 0.0},
		{lat: -90.001, lon: 0.0},
		{lat: 0.0, lon: 180.001},
		{lat: 0.0, lon: -180.001},
		{lat: 180.0, lon: 0.0},
		{lat: -180.0, lon: 0.0},
		{lat: 0.0, lon: 360.0},
		{lat: 91.0, lon: 181.0},
	}

	// Add random out-of-range values.
	rng := rand.New(rand.NewSource(42))
	for i := 0; i < 20; i++ {
		// Random lat outside [-90, 90]: pick from [91, 1000] or [-1000, -91]
		lat := 91.0 + rng.Float64()*909.0
		if rng.Intn(2) == 0 {
			lat = -lat
		}
		outOfRangeCases = append(outOfRangeCases, struct{ lat, lon float64 }{lat: lat, lon: 0.0})

		// Random lon outside [-180, 180]
		lon := 181.0 + rng.Float64()*819.0
		if rng.Intn(2) == 0 {
			lon = -lon
		}
		outOfRangeCases = append(outOfRangeCases, struct{ lat, lon float64 }{lat: 0.0, lon: lon})
	}

	for _, tc := range outOfRangeCases {
		path := fmt.Sprintf("/operators?lat=%f&lon=%f", tc.lat, tc.lon)
		resp := get(t, srv, path)
		resp.Body.Close()
		if resp.StatusCode != http.StatusBadRequest {
			t.Errorf("expected 400 for lat=%f, lon=%f (out of range), got %d",
				tc.lat, tc.lon, resp.StatusCode)
		}
	}

	// Valid coordinates should not return 400 (may return 200 with empty array).
	validCases := []struct{ lat, lon float64 }{
		{0.0, 0.0},
		{90.0, 180.0},
		{-90.0, -180.0},
		{48.137, 11.575},
	}
	for _, tc := range validCases {
		path := fmt.Sprintf("/operators?lat=%f&lon=%f", tc.lat, tc.lon)
		resp := get(t, srv, path)
		resp.Body.Close()
		if resp.StatusCode == http.StatusBadRequest {
			t.Errorf("did not expect 400 for valid lat=%f, lon=%f, got %d",
				tc.lat, tc.lon, resp.StatusCode)
		}
	}
}

// Ensure model is imported (used for type assertions in tests above via DefaultConfig).
var _ model.Config
