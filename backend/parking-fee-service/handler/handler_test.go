package handler_test

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/config"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/handler"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// setupTestServer creates an httptest.Server wired with the default Munich config.
func setupTestServer(t *testing.T) *httptest.Server {
	t.Helper()
	cfg := config.DefaultConfig()
	s := store.NewStore(cfg.Zones, cfg.Operators)

	mux := http.NewServeMux()
	mux.Handle("GET /health", handler.HealthHandler())
	mux.Handle("GET /operators", handler.NewOperatorHandler(s, cfg.Zones, cfg.ProximityThreshold))
	mux.Handle("GET /operators/{id}/adapter", handler.NewAdapterHandler(s))

	return httptest.NewServer(mux)
}

// getJSON performs a GET request and decodes the JSON body into v.
func getJSON(t *testing.T, server *httptest.Server, path string, v any) *http.Response {
	t.Helper()
	resp, err := http.Get(server.URL + path)
	if err != nil {
		t.Fatalf("GET %s: %v", path, err)
	}
	body, err := io.ReadAll(resp.Body)
	resp.Body.Close()
	if err != nil {
		t.Fatalf("reading body: %v", err)
	}
	if v != nil && len(body) > 0 {
		if err := json.Unmarshal(body, v); err != nil {
			t.Logf("response body: %s", body)
			t.Fatalf("json.Unmarshal: %v", err)
		}
	}
	return resp
}

// getRaw performs a GET request and returns the response with raw body bytes.
func getRaw(t *testing.T, server *httptest.Server, path string) (*http.Response, []byte) {
	t.Helper()
	resp, err := http.Get(server.URL + path)
	if err != nil {
		t.Fatalf("GET %s: %v", path, err)
	}
	body, err := io.ReadAll(resp.Body)
	resp.Body.Close()
	if err != nil {
		t.Fatalf("reading body: %v", err)
	}
	return resp, body
}

// TS-05-1: GET /operators?lat=&lon= returns matching operators.
func TestOperatorLookup(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	var body []map[string]any
	resp := getJSON(t, srv, "/operators?lat=48.1375&lon=11.5600", &body)

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}
	if len(body) < 1 {
		t.Fatalf("response array has %d operators, want >= 1", len(body))
	}

	// Find parkhaus-munich in the results.
	var found bool
	for _, op := range body {
		if op["id"] == "parkhaus-munich" {
			found = true
			if op["zone_id"] != "munich-central" {
				t.Errorf("parkhaus-munich zone_id = %v, want munich-central", op["zone_id"])
			}
			if op["name"] == nil || op["name"] == "" {
				t.Error("parkhaus-munich name is empty")
			}
			if op["rate"] == nil {
				t.Error("parkhaus-munich rate is nil")
			}
		}
	}
	if !found {
		t.Errorf("parkhaus-munich not found in response; got %+v", body)
	}
}

// TS-05-5: No matches returns empty JSON array with HTTP 200.
func TestEmptyArrayNoMatches(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	var body []any
	resp := getJSON(t, srv, "/operators?lat=0.0&lon=0.0", &body)

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}
	if len(body) != 0 {
		t.Errorf("response array has %d elements, want 0; body: %+v", len(body), body)
	}
}

// TS-05-6: GET /operators/{id}/adapter returns adapter metadata.
func TestAdapterMetadataRetrieval(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	var body map[string]any
	resp := getJSON(t, srv, "/operators/parkhaus-munich/adapter", &body)

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}
	if v, ok := body["image_ref"].(string); !ok || v == "" {
		t.Errorf("image_ref = %v, want non-empty string", body["image_ref"])
	}
	if v, ok := body["checksum_sha256"].(string); !ok || v == "" {
		t.Errorf("checksum_sha256 = %v, want non-empty string", body["checksum_sha256"])
	}
	if v, ok := body["version"].(string); !ok || v == "" {
		t.Errorf("version = %v, want non-empty string", body["version"])
	}
}

// TS-05-7: Successful adapter metadata retrieval returns HTTP 200.
func TestAdapterMetadataHTTP200(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	resp, _ := getRaw(t, srv, "/operators/parkhaus-munich/adapter")
	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}
}

// TS-05-8: GET /health returns HTTP 200 with {"status":"ok"}.
func TestHealthCheck(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	var body map[string]any
	resp := getJSON(t, srv, "/health", &body)

	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}
	if body["status"] != "ok" {
		t.Errorf(`body["status"] = %v, want "ok"`, body["status"])
	}
}

// TS-05-12: All responses set Content-Type: application/json.
func TestContentTypeHeader(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	endpoints := []string{
		"/health",
		"/operators?lat=48.137&lon=11.575",
		"/operators/parkhaus-munich/adapter",
	}
	for _, ep := range endpoints {
		t.Run(ep, func(t *testing.T) {
			resp, _ := getRaw(t, srv, ep)
			ct := resp.Header.Get("Content-Type")
			if !strings.HasPrefix(ct, "application/json") {
				t.Errorf("Content-Type = %q, want application/json", ct)
			}
		})
	}
}

// TS-05-13: Operator lookup response includes id, name, zone_id, rate, but NOT adapter.
func TestOperatorResponseFields(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	resp, body := getRaw(t, srv, "/operators?lat=48.1375&lon=11.5600")
	if resp.StatusCode != http.StatusOK {
		t.Errorf("status = %d, want 200", resp.StatusCode)
	}

	var operators []map[string]any
	if err := json.Unmarshal(body, &operators); err != nil {
		t.Fatalf("json.Unmarshal: %v; body: %s", err, body)
	}
	if len(operators) == 0 {
		t.Fatal("response array is empty")
	}

	op := operators[0]

	if id, ok := op["id"].(string); !ok || id == "" {
		t.Error("operator.id is missing or empty")
	}
	if name, ok := op["name"].(string); !ok || name == "" {
		t.Error("operator.name is missing or empty")
	}
	if zoneID, ok := op["zone_id"].(string); !ok || zoneID == "" {
		t.Error("operator.zone_id is missing or empty")
	}
	rate, ok := op["rate"].(map[string]any)
	if !ok || rate == nil {
		t.Error("operator.rate is missing or not an object")
	} else {
		if rateType, ok := rate["type"].(string); !ok || (rateType != "per-hour" && rateType != "flat-fee") {
			t.Errorf("rate.type = %v, want per-hour or flat-fee", rate["type"])
		}
		if amt, ok := rate["amount"].(float64); !ok || amt <= 0 {
			t.Errorf("rate.amount = %v, want > 0", rate["amount"])
		}
		if cur, ok := rate["currency"].(string); !ok || cur == "" {
			t.Errorf("rate.currency = %v, want non-empty", rate["currency"])
		}
	}

	// adapter must NOT be present in the operator lookup response.
	if _, hasAdapter := op["adapter"]; hasAdapter {
		t.Error("operator.adapter should not be present in lookup response")
	}
}

// TS-05-14: Error responses use {"error":"<message>"} format.
func TestErrorResponseFormat(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	resp, body := getRaw(t, srv, "/operators")
	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("status = %d, want 400", resp.StatusCode)
	}

	var errBody map[string]any
	if err := json.Unmarshal(body, &errBody); err != nil {
		t.Fatalf("error body is not valid JSON: %v; body: %s", err, body)
	}
	if msg, ok := errBody["error"].(string); !ok || msg == "" {
		t.Errorf(`errBody["error"] = %v, want non-empty string`, errBody["error"])
	}
}

// TS-05-E1: Missing lat or lon parameters return HTTP 400.
func TestMissingLatLon(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	urls := []string{
		"/operators",
		"/operators?lat=48.137",
		"/operators?lon=11.575",
	}
	for _, url := range urls {
		t.Run(url, func(t *testing.T) {
			resp, body := getRaw(t, srv, url)
			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("status = %d, want 400", resp.StatusCode)
			}
			var errBody map[string]any
			if err := json.Unmarshal(body, &errBody); err != nil {
				t.Fatalf("error body not valid JSON: %v; body: %s", err, body)
			}
			wantMsg := "lat and lon query parameters are required"
			if errBody["error"] != wantMsg {
				t.Errorf(`error = %q, want %q`, errBody["error"], wantMsg)
			}
		})
	}
}

// TS-05-E2: Out-of-range coordinates return HTTP 400.
func TestInvalidCoordinateRange(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	cases := []struct{ lat, lon string }{
		{"91.0", "11.575"},
		{"-91.0", "11.575"},
		{"48.137", "181.0"},
		{"48.137", "-181.0"},
	}
	for _, tc := range cases {
		url := fmt.Sprintf("/operators?lat=%s&lon=%s", tc.lat, tc.lon)
		t.Run(url, func(t *testing.T) {
			resp, body := getRaw(t, srv, url)
			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("status = %d, want 400", resp.StatusCode)
			}
			var errBody map[string]any
			if err := json.Unmarshal(body, &errBody); err != nil {
				t.Fatalf("error body not valid JSON: %v; body: %s", err, body)
			}
			if errBody["error"] != "invalid coordinates" {
				t.Errorf(`error = %q, want "invalid coordinates"`, errBody["error"])
			}
		})
	}
}

// TS-05-E3: Non-numeric lat/lon values return HTTP 400.
func TestNonNumericCoordinates(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	cases := []struct{ lat, lon string }{
		{"abc", "11.575"},
		{"48.137", "xyz"},
	}
	for _, tc := range cases {
		url := fmt.Sprintf("/operators?lat=%s&lon=%s", tc.lat, tc.lon)
		t.Run(url, func(t *testing.T) {
			resp, body := getRaw(t, srv, url)
			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("status = %d, want 400", resp.StatusCode)
			}
			var errBody map[string]any
			if err := json.Unmarshal(body, &errBody); err != nil {
				t.Fatalf("error body not valid JSON: %v; body: %s", err, body)
			}
			if errBody["error"] != "invalid coordinates" {
				t.Errorf(`error = %q, want "invalid coordinates"`, errBody["error"])
			}
		})
	}
}

// TS-05-E4: Unknown operator ID returns HTTP 404.
func TestUnknownOperatorID(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	resp, body := getRaw(t, srv, "/operators/nonexistent-operator/adapter")
	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("status = %d, want 404", resp.StatusCode)
	}
	var errBody map[string]any
	if err := json.Unmarshal(body, &errBody); err != nil {
		t.Fatalf("error body not valid JSON: %v; body: %s", err, body)
	}
	if errBody["error"] != "operator not found" {
		t.Errorf(`error = %q, want "operator not found"`, errBody["error"])
	}
}

// TS-05-P4: Coordinate validation property.
// For any lat outside [-90,90] or lon outside [-180,180], the handler returns HTTP 400.
func TestPropertyCoordinateValidation(t *testing.T) {
	srv := setupTestServer(t)
	defer srv.Close()

	outOfRange := []struct {
		lat, lon float64
	}{
		{91.0, 0.0},
		{-91.0, 0.0},
		{0.0, 181.0},
		{0.0, -181.0},
		{90.1, 180.1},
		{-90.001, -0.001},
		{1000.0, 1000.0},
		{-1000.0, -1000.0},
	}
	for _, tc := range outOfRange {
		url := fmt.Sprintf("/operators?lat=%g&lon=%g", tc.lat, tc.lon)
		t.Run(url, func(t *testing.T) {
			resp, _ := getRaw(t, srv, url)
			if resp.StatusCode != http.StatusBadRequest {
				t.Errorf("lat=%g lon=%g: status = %d, want 400", tc.lat, tc.lon, resp.StatusCode)
			}
		})
	}

	// Valid boundary values should NOT return 400.
	validBoundary := []struct {
		lat, lon float64
	}{
		{90.0, 180.0},
		{-90.0, -180.0},
		{0.0, 0.0},
	}
	for _, tc := range validBoundary {
		url := fmt.Sprintf("/operators?lat=%g&lon=%g", tc.lat, tc.lon)
		t.Run("valid_"+url, func(t *testing.T) {
			resp, _ := getRaw(t, srv, url)
			if resp.StatusCode == http.StatusBadRequest {
				t.Errorf("lat=%g lon=%g: got 400, valid coordinates should not be rejected", tc.lat, tc.lon)
			}
		})
	}
}
