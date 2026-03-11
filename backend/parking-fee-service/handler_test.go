package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"regexp"
	"strings"
	"testing"
)

// newTestRouter creates a router backed by the default demo configuration.
func newTestRouter(t *testing.T) http.Handler {
	t.Helper()
	cfg, err := LoadConfig("")
	if err != nil {
		t.Fatalf("failed to load default config: %v", err)
	}
	store := NewStore(cfg)
	if store == nil {
		t.Fatal("NewStore returned nil")
	}
	return NewRouter(store)
}

// --- TS-05-1: Operator Lookup Returns Operators for Location Inside a Zone ---

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
		t.Errorf("expected Content-Type application/json, got %s", ct)
	}

	var operators []Operator
	if err := json.Unmarshal(w.Body.Bytes(), &operators); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}
	if len(operators) < 1 {
		t.Fatal("expected at least 1 operator in response")
	}

	var found *Operator
	for i := range operators {
		if operators[i].ID == "muc-central" {
			found = &operators[i]
			break
		}
	}
	if found == nil {
		t.Fatal("expected muc-central operator in response")
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
		t.Errorf("expected rate_amount 2.50, got %v", found.RateAmount)
	}
	if found.RateCurrency != "EUR" {
		t.Errorf("expected rate_currency EUR, got %s", found.RateCurrency)
	}
}

// --- TS-05-2: Operator Lookup Returns Operators for Location Near a Zone ---

func TestOperatorLookupNearZone(t *testing.T) {
	router := newTestRouter(t)
	// ~55m north of the muc-central polygon northern edge at lat=48.1420
	req := httptest.NewRequest(http.MethodGet, "/operators?lat=48.1425&lon=11.5625", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", w.Code)
	}

	var operators []Operator
	if err := json.Unmarshal(w.Body.Bytes(), &operators); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}

	found := false
	for _, op := range operators {
		if op.ID == "muc-central" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected muc-central operator for near-zone location")
	}
}

// --- TS-05-3: Operator Lookup Returns Empty List for Remote Location ---

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
		t.Errorf("expected Content-Type application/json, got %s", ct)
	}

	var operators []Operator
	if err := json.Unmarshal(w.Body.Bytes(), &operators); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}
	if len(operators) != 0 {
		t.Errorf("expected empty array for remote location, got %d operators", len(operators))
	}
}

// --- TS-05-4: Adapter Metadata Returns Correct Data for Valid Operator ---

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
		t.Errorf("expected Content-Type application/json, got %s", ct)
	}

	var metadata AdapterMetadata
	if err := json.Unmarshal(w.Body.Bytes(), &metadata); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}
	if metadata.ImageRef != "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-central:v1.0.0" {
		t.Errorf("unexpected image_ref: %s", metadata.ImageRef)
	}

	checksumRegex := regexp.MustCompile(`^sha256:[0-9a-f]{64}$`)
	if !checksumRegex.MatchString(metadata.ChecksumSHA256) {
		t.Errorf("checksum_sha256 does not match expected pattern: %s", metadata.ChecksumSHA256)
	}
	if metadata.Version != "v1.0.0" {
		t.Errorf("expected version v1.0.0, got %s", metadata.Version)
	}
}

// --- TS-05-5: Health Check Returns 200 OK ---

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
		t.Errorf("expected Content-Type application/json, got %s", ct)
	}

	var body map[string]string
	if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("expected status 'ok', got '%s'", body["status"])
	}
}

// --- TS-05-6: Operator Lookup Includes Rate Information ---

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
		t.Fatalf("failed to parse response JSON: %v", err)
	}

	var found *Operator
	for i := range operators {
		if operators[i].ID == "muc-airport" {
			found = &operators[i]
			break
		}
	}
	if found == nil {
		t.Fatal("expected muc-airport operator in response")
	}
	if found.RateType != RateFlatFee {
		t.Errorf("expected rate_type flat_fee, got %s", found.RateType)
	}
	if found.RateAmount != 5.00 {
		t.Errorf("expected rate_amount 5.00, got %v", found.RateAmount)
	}
	if found.RateCurrency != "EUR" {
		t.Errorf("expected rate_currency EUR, got %s", found.RateCurrency)
	}
}

// --- TS-05-E1: Adapter Metadata Returns 404 for Unknown Operator ---

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
		t.Errorf("expected Content-Type application/json, got %s", ct)
	}

	var body ErrorResponse
	if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse error response JSON: %v", err)
	}
	if body.Error == "" {
		t.Error("expected non-empty error message")
	}
}

// --- TS-05-E2: Invalid Latitude Returns 400 ---

func TestInvalidLatitude(t *testing.T) {
	router := newTestRouter(t)

	tests := []struct {
		name string
		url  string
	}{
		{"missing lat", "/operators?lon=11.58"},
		{"non-numeric lat", "/operators?lat=abc&lon=11.58"},
		{"lat > 90", "/operators?lat=91.0&lon=11.58"},
		{"lat < -90", "/operators?lat=-91.0&lon=11.58"},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, tc.url, nil)
			w := httptest.NewRecorder()
			router.ServeHTTP(w, req)

			if w.Code != http.StatusBadRequest {
				t.Fatalf("expected status 400, got %d", w.Code)
			}
			ct := w.Header().Get("Content-Type")
			if !strings.Contains(ct, "application/json") {
				t.Errorf("expected Content-Type application/json, got %s", ct)
			}
			var body ErrorResponse
			if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
				t.Fatalf("failed to parse error response: %v", err)
			}
			if body.Error == "" {
				t.Error("expected non-empty error message")
			}
		})
	}
}

// --- TS-05-E3: Invalid Longitude Returns 400 ---

func TestInvalidLongitude(t *testing.T) {
	router := newTestRouter(t)

	tests := []struct {
		name string
		url  string
	}{
		{"missing lon", "/operators?lat=48.14"},
		{"non-numeric lon", "/operators?lat=48.14&lon=xyz"},
		{"lon > 180", "/operators?lat=48.14&lon=181.0"},
		{"lon < -180", "/operators?lat=48.14&lon=-181.0"},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodGet, tc.url, nil)
			w := httptest.NewRecorder()
			router.ServeHTTP(w, req)

			if w.Code != http.StatusBadRequest {
				t.Fatalf("expected status 400, got %d", w.Code)
			}
			ct := w.Header().Get("Content-Type")
			if !strings.Contains(ct, "application/json") {
				t.Errorf("expected Content-Type application/json, got %s", ct)
			}
			var body ErrorResponse
			if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
				t.Fatalf("failed to parse error response: %v", err)
			}
			if body.Error == "" {
				t.Error("expected non-empty error message")
			}
		})
	}
}

// --- TS-05-E4: Undefined Route Returns 404 ---

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
		t.Errorf("expected Content-Type application/json, got %s", ct)
	}

	var body ErrorResponse
	if err := json.Unmarshal(w.Body.Bytes(), &body); err != nil {
		t.Fatalf("failed to parse error response JSON: %v", err)
	}
	if body.Error == "" {
		t.Error("expected non-empty error message")
	}
}

// --- TS-05-P3: Response Format Consistency ---

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
				t.Errorf("expected Content-Type application/json for %s, got %s", endpoint, ct)
			}

			if !json.Valid(w.Body.Bytes()) {
				t.Errorf("response body is not valid JSON for %s: %s", endpoint, w.Body.String())
			}
		})
	}
}

// --- TS-05-P4: Operator-Adapter Integrity ---

func TestPropertyOperatorAdapterIntegrity(t *testing.T) {
	router := newTestRouter(t)

	// Get operators for a location inside muc-central zone
	req := httptest.NewRequest(http.MethodGet, "/operators?lat=48.1395&lon=11.5625", nil)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d", w.Code)
	}

	var operators []Operator
	if err := json.Unmarshal(w.Body.Bytes(), &operators); err != nil {
		t.Fatalf("failed to parse operators response: %v", err)
	}
	if len(operators) == 0 {
		t.Fatal("expected at least one operator to verify adapter integrity")
	}

	checksumRegex := regexp.MustCompile(`^sha256:[0-9a-f]{64}$`)

	for _, op := range operators {
		t.Run(fmt.Sprintf("adapter_%s", op.ID), func(t *testing.T) {
			adapterReq := httptest.NewRequest(http.MethodGet, "/operators/"+op.ID+"/adapter", nil)
			aw := httptest.NewRecorder()
			router.ServeHTTP(aw, adapterReq)

			if aw.Code != http.StatusOK {
				t.Fatalf("expected status 200 for adapter of %s, got %d", op.ID, aw.Code)
			}

			var metadata AdapterMetadata
			if err := json.Unmarshal(aw.Body.Bytes(), &metadata); err != nil {
				t.Fatalf("failed to parse adapter metadata: %v", err)
			}
			if metadata.ImageRef == "" {
				t.Error("expected non-empty image_ref")
			}
			if !checksumRegex.MatchString(metadata.ChecksumSHA256) {
				t.Errorf("checksum_sha256 does not match pattern: %s", metadata.ChecksumSHA256)
			}
			if metadata.Version == "" {
				t.Error("expected non-empty version")
			}
		})
	}
}
