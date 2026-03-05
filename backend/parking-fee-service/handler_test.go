package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"regexp"
	"strings"
	"testing"
)

// newTestRouter creates a router initialized with the default demo config for integration tests.
func newTestRouter(t *testing.T) http.Handler {
	t.Helper()
	cfg, err := LoadConfig("")
	if err != nil {
		t.Fatalf("failed to load default config: %v", err)
	}
	store := NewStore(cfg)
	return NewRouter(store)
}

// TS-05-1: Operator Lookup Returns Operators for Location Inside a Zone
func TestOperatorLookupInsideZone(t *testing.T) {
	router := newTestRouter(t)

	req := httptest.NewRequest(http.MethodGet, "/operators?lat=48.1395&lon=11.5625", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", w.Code)
	}
	ct := w.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Fatalf("expected Content-Type application/json, got %s", ct)
	}

	var operators []Operator
	if err := json.Unmarshal(w.Body.Bytes(), &operators); err != nil {
		t.Fatalf("failed to parse JSON: %v", err)
	}
	if len(operators) < 1 {
		t.Fatal("expected at least 1 operator, got 0")
	}

	var found *Operator
	for i := range operators {
		if operators[i].ID == "muc-central" {
			found = &operators[i]
			break
		}
	}
	if found == nil {
		t.Fatal("expected operator muc-central in results")
	}
	if found.Name == "" {
		t.Error("expected non-empty name")
	}
	if found.ZoneID != "zone-muc-central" {
		t.Errorf("expected zone_id zone-muc-central, got %s", found.ZoneID)
	}
	if found.RateType != RatePerHour {
		t.Errorf("expected rate_type per_hour, got %s", found.RateType)
	}
	if found.RateAmount != 2.50 {
		t.Errorf("expected rate_amount 2.50, got %f", found.RateAmount)
	}
	if found.RateCurrency != "EUR" {
		t.Errorf("expected rate_currency EUR, got %s", found.RateCurrency)
	}
}

// TS-05-2: Operator Lookup Returns Operators for Location Near a Zone
func TestOperatorLookupNearZone(t *testing.T) {
	router := newTestRouter(t)

	// ~55m north of northern edge at lat=48.1420
	req := httptest.NewRequest(http.MethodGet, "/operators?lat=48.1425&lon=11.5625", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", w.Code)
	}

	var operators []Operator
	if err := json.Unmarshal(w.Body.Bytes(), &operators); err != nil {
		t.Fatalf("failed to parse JSON: %v", err)
	}

	found := false
	for _, op := range operators {
		if op.ID == "muc-central" {
			found = true
			break
		}
	}
	if !found {
		t.Fatal("expected operator muc-central in near-zone results")
	}
}

// TS-05-3: Operator Lookup Returns Empty List for Remote Location
func TestOperatorLookupRemoteLocation(t *testing.T) {
	router := newTestRouter(t)

	req := httptest.NewRequest(http.MethodGet, "/operators?lat=52.5200&lon=13.4050", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", w.Code)
	}
	ct := w.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Fatalf("expected Content-Type application/json, got %s", ct)
	}

	var operators []Operator
	if err := json.Unmarshal(w.Body.Bytes(), &operators); err != nil {
		t.Fatalf("failed to parse JSON: %v", err)
	}
	if len(operators) != 0 {
		t.Fatalf("expected 0 operators for remote location, got %d", len(operators))
	}
}

// TS-05-4: Adapter Metadata Returns Correct Data for Valid Operator
func TestAdapterMetadataValid(t *testing.T) {
	router := newTestRouter(t)

	req := httptest.NewRequest(http.MethodGet, "/operators/muc-central/adapter", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", w.Code)
	}
	ct := w.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Fatalf("expected Content-Type application/json, got %s", ct)
	}

	var metadata AdapterMetadata
	if err := json.Unmarshal(w.Body.Bytes(), &metadata); err != nil {
		t.Fatalf("failed to parse JSON: %v", err)
	}
	if metadata.ImageRef != "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-central:v1.0.0" {
		t.Errorf("unexpected image_ref: %s", metadata.ImageRef)
	}
	checksumRe := regexp.MustCompile(`^sha256:[0-9a-f]{64}$`)
	if !checksumRe.MatchString(metadata.ChecksumSHA256) {
		t.Errorf("checksum_sha256 does not match expected pattern: %s", metadata.ChecksumSHA256)
	}
	if metadata.Version != "v1.0.0" {
		t.Errorf("unexpected version: %s", metadata.Version)
	}
}

// TS-05-5: Health Check Returns 200 OK
func TestHealthCheck(t *testing.T) {
	router := newTestRouter(t)

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", w.Code)
	}
	ct := w.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Fatalf("expected Content-Type application/json, got %s", ct)
	}

	var body map[string]string
	if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse JSON: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("expected status ok, got %s", body["status"])
	}
}

// TS-05-6: Operator Lookup Includes Rate Information
func TestOperatorLookupRateInfo(t *testing.T) {
	router := newTestRouter(t)

	// Inside zone-muc-airport polygon
	req := httptest.NewRequest(http.MethodGet, "/operators?lat=48.3525&lon=11.7850", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", w.Code)
	}

	var operators []Operator
	if err := json.Unmarshal(w.Body.Bytes(), &operators); err != nil {
		t.Fatalf("failed to parse JSON: %v", err)
	}

	var found *Operator
	for i := range operators {
		if operators[i].ID == "muc-airport" {
			found = &operators[i]
			break
		}
	}
	if found == nil {
		t.Fatal("expected operator muc-airport in results")
	}
	if found.RateType != RateFlatFee {
		t.Errorf("expected rate_type flat_fee, got %s", found.RateType)
	}
	if found.RateAmount != 5.00 {
		t.Errorf("expected rate_amount 5.00, got %f", found.RateAmount)
	}
	if found.RateCurrency != "EUR" {
		t.Errorf("expected rate_currency EUR, got %s", found.RateCurrency)
	}
}

// TS-05-E1: Adapter Metadata Returns 404 for Unknown Operator
func TestAdapterMetadataUnknownOperator(t *testing.T) {
	router := newTestRouter(t)

	req := httptest.NewRequest(http.MethodGet, "/operators/nonexistent-operator/adapter", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusNotFound {
		t.Fatalf("expected status 404, got %d", w.Code)
	}
	ct := w.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Fatalf("expected Content-Type application/json, got %s", ct)
	}

	var body ErrorResponse
	if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse JSON: %v", err)
	}
	if body.Error == "" {
		t.Error("expected non-empty error message")
	}
}

// TS-05-E2: Invalid Latitude Returns 400
func TestInvalidLatitude(t *testing.T) {
	router := newTestRouter(t)

	cases := []struct {
		name string
		url  string
	}{
		{"missing lat", "/operators?lon=11.58"},
		{"non-numeric lat", "/operators?lat=abc&lon=11.58"},
		{"lat > 90", "/operators?lat=91.0&lon=11.58"},
		{"lat < -90", "/operators?lat=-91.0&lon=11.58"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, tc.url, nil)
			w := httptest.NewRecorder()
			router.ServeHTTP(w, req)

			if w.Code != http.StatusBadRequest {
				t.Fatalf("expected status 400, got %d", w.Code)
			}
			ct := w.Header().Get("Content-Type")
			if !strings.Contains(ct, "application/json") {
				t.Fatalf("expected Content-Type application/json, got %s", ct)
			}

			var body ErrorResponse
			if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
				t.Fatalf("failed to parse JSON: %v", err)
			}
			if body.Error == "" {
				t.Error("expected non-empty error message")
			}
		})
	}
}

// TS-05-E3: Invalid Longitude Returns 400
func TestInvalidLongitude(t *testing.T) {
	router := newTestRouter(t)

	cases := []struct {
		name string
		url  string
	}{
		{"missing lon", "/operators?lat=48.14"},
		{"non-numeric lon", "/operators?lat=48.14&lon=xyz"},
		{"lon > 180", "/operators?lat=48.14&lon=181.0"},
		{"lon < -180", "/operators?lat=48.14&lon=-181.0"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, tc.url, nil)
			w := httptest.NewRecorder()
			router.ServeHTTP(w, req)

			if w.Code != http.StatusBadRequest {
				t.Fatalf("expected status 400, got %d", w.Code)
			}
			ct := w.Header().Get("Content-Type")
			if !strings.Contains(ct, "application/json") {
				t.Fatalf("expected Content-Type application/json, got %s", ct)
			}

			var body ErrorResponse
			if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
				t.Fatalf("failed to parse JSON: %v", err)
			}
			if body.Error == "" {
				t.Error("expected non-empty error message")
			}
		})
	}
}

// TS-05-E4: Undefined Route Returns 404
func TestUndefinedRoute(t *testing.T) {
	router := newTestRouter(t)

	req := httptest.NewRequest(http.MethodGet, "/nonexistent-path", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusNotFound {
		t.Fatalf("expected status 404, got %d", w.Code)
	}
	ct := w.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Fatalf("expected Content-Type application/json, got %s", ct)
	}

	var body ErrorResponse
	if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse JSON: %v", err)
	}
	if body.Error == "" {
		t.Error("expected non-empty error message")
	}
}

// TS-05-P3: Response Format Consistency (Property Test)
func TestPropertyResponseFormat(t *testing.T) {
	router := newTestRouter(t)

	endpoints := []string{
		"/health",
		"/operators?lat=48.14&lon=11.58",
		"/operators/muc-central/adapter",
		"/operators/unknown/adapter",
		"/operators",
		"/nonexistent-path",
	}

	for _, endpoint := range endpoints {
		t.Run(endpoint, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, endpoint, nil)
			w := httptest.NewRecorder()
			router.ServeHTTP(w, req)

			ct := w.Header().Get("Content-Type")
			if !strings.Contains(ct, "application/json") {
				t.Errorf("endpoint %s: expected Content-Type application/json, got %s", endpoint, ct)
			}
			if !json.Valid(w.Body.Bytes()) {
				t.Errorf("endpoint %s: response body is not valid JSON: %s", endpoint, w.Body.String())
			}
		})
	}
}

// TS-05-P4: Operator-Adapter Integrity (Property Test)
func TestPropertyOperatorAdapterIntegrity(t *testing.T) {
	router := newTestRouter(t)

	// Get operators at Munich Central
	req := httptest.NewRequest(http.MethodGet, "/operators?lat=48.1395&lon=11.5625", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected status 200 for operator lookup, got %d", w.Code)
	}

	var operators []Operator
	if err := json.Unmarshal(w.Body.Bytes(), &operators); err != nil {
		t.Fatalf("failed to parse operators JSON: %v", err)
	}
	if len(operators) == 0 {
		t.Fatal("expected at least one operator for integrity check")
	}

	checksumRe := regexp.MustCompile(`^sha256:[0-9a-f]{64}$`)

	for _, op := range operators {
		t.Run(op.ID, func(t *testing.T) {
			adapterReq := httptest.NewRequest(http.MethodGet, "/operators/"+op.ID+"/adapter", nil)
			adapterW := httptest.NewRecorder()
			router.ServeHTTP(adapterW, adapterReq)

			if adapterW.Code != http.StatusOK {
				t.Fatalf("expected status 200 for adapter of operator %s, got %d", op.ID, adapterW.Code)
			}

			var metadata AdapterMetadata
			if err := json.Unmarshal(adapterW.Body.Bytes(), &metadata); err != nil {
				t.Fatalf("failed to parse adapter metadata JSON: %v", err)
			}
			if metadata.ImageRef == "" {
				t.Error("expected non-empty image_ref")
			}
			if !checksumRe.MatchString(metadata.ChecksumSHA256) {
				t.Errorf("checksum_sha256 does not match pattern: %s", metadata.ChecksumSHA256)
			}
			if metadata.Version == "" {
				t.Error("expected non-empty version")
			}
		})
	}
}
