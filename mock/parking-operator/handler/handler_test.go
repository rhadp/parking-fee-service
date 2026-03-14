package handler_test

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/rhadp/parking-fee-service/mock/parking-operator/handler"
	"github.com/rhadp/parking-fee-service/mock/parking-operator/store"
)

// newTestMux wires the three handlers to a ServeMux for use in httptest.
func newTestMux(s *store.Store) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", handler.StartHandler(s))
	mux.HandleFunc("POST /parking/stop", handler.StopHandler(s))
	mux.HandleFunc("GET /parking/status/{session_id}", handler.StatusHandler(s))
	return mux
}

// seedSession starts a session and returns its session_id via the HTTP API.
func seedSession(t *testing.T, mux http.Handler, vehicleID, zoneID string, startTS int64) string {
	t.Helper()
	body := map[string]any{
		"vehicle_id": vehicleID,
		"zone_id":    zoneID,
		"timestamp":  startTS,
	}
	b, _ := json.Marshal(body)
	req := httptest.NewRequest("POST", "/parking/start", strings.NewReader(string(b)))
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("seedSession: POST /parking/start returned %d, want 200", w.Code)
	}
	var resp map[string]any
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("seedSession: decode response: %v", err)
	}
	sid, _ := resp["session_id"].(string)
	if sid == "" {
		t.Fatal("seedSession: session_id not in response")
	}
	return sid
}

// TS-09-5: PARKING_OPERATOR Start Session — HTTP handler level
// Requirement: 09-REQ-2.2
func TestStartSession(t *testing.T) {
	s := store.NewStore()
	mux := newTestMux(s)

	body := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	req := httptest.NewRequest("POST", "/parking/start", strings.NewReader(body))
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200", w.Code)
	}
	var resp map[string]any
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if resp["session_id"] == "" || resp["session_id"] == nil {
		t.Error("session_id is empty, want a UUID")
	}
	if resp["status"] != "active" {
		t.Errorf("status = %q, want %q", resp["status"], "active")
	}
	rate, ok := resp["rate"].(map[string]any)
	if !ok {
		t.Fatalf("rate is not an object: %T", resp["rate"])
	}
	if rate["rate_type"] != "per_hour" {
		t.Errorf("rate_type = %q, want %q", rate["rate_type"], "per_hour")
	}
	if rate["amount"] != 2.50 {
		t.Errorf("amount = %v, want 2.50", rate["amount"])
	}
	if rate["currency"] != "EUR" {
		t.Errorf("currency = %q, want %q", rate["currency"], "EUR")
	}
}

// TS-09-6: PARKING_OPERATOR Stop Session — computes duration and total_amount
// Requirement: 09-REQ-2.3
func TestStopSession(t *testing.T) {
	s := store.NewStore()
	mux := newTestMux(s)

	sid := seedSession(t, mux, "VIN001", "zone-1", 1700000000)

	stopBody, _ := json.Marshal(map[string]any{"session_id": sid, "timestamp": 1700003600})
	req := httptest.NewRequest("POST", "/parking/stop", strings.NewReader(string(stopBody)))
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200", w.Code)
	}
	var resp map[string]any
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if resp["status"] != "stopped" {
		t.Errorf("status = %q, want %q", resp["status"], "stopped")
	}
	// duration_seconds is encoded as JSON number → float64
	if ds, _ := resp["duration_seconds"].(float64); int64(ds) != 3600 {
		t.Errorf("duration_seconds = %v, want 3600", resp["duration_seconds"])
	}
	if ta, _ := resp["total_amount"].(float64); ta != 2.50 {
		t.Errorf("total_amount = %v, want 2.50", ta)
	}
}

// TS-09-7: PARKING_OPERATOR Get Status — returns session info
// Requirement: 09-REQ-2.4
func TestGetStatus(t *testing.T) {
	s := store.NewStore()
	mux := newTestMux(s)

	sid := seedSession(t, mux, "VIN001", "zone-1", 1700000000)

	req := httptest.NewRequest("GET", "/parking/status/"+sid, nil)
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status = %d, want 200", w.Code)
	}
	var resp map[string]any
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if resp["session_id"] != sid {
		t.Errorf("session_id = %q, want %q", resp["session_id"], sid)
	}
	if resp["status"] != "active" {
		t.Errorf("status = %q, want %q", resp["status"], "active")
	}
}

// TS-09-E3: Stop with unknown session_id → HTTP 404
// Requirement: 09-REQ-2.E1
func TestStopUnknownSession(t *testing.T) {
	s := store.NewStore()
	mux := newTestMux(s)

	body := `{"session_id":"nonexistent","timestamp":1700000000}`
	req := httptest.NewRequest("POST", "/parking/stop", strings.NewReader(body))
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusNotFound {
		t.Errorf("status = %d, want 404", w.Code)
	}
	var resp map[string]string
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if resp["error"] != "session not found" {
		t.Errorf("error = %q, want %q", resp["error"], "session not found")
	}
}

// TS-09-E4: Status with unknown session_id → HTTP 404
// Requirement: 09-REQ-2.E2
func TestStatusUnknownSession(t *testing.T) {
	s := store.NewStore()
	mux := newTestMux(s)

	req := httptest.NewRequest("GET", "/parking/status/nonexistent", nil)
	w := httptest.NewRecorder()
	mux.ServeHTTP(w, req)

	if w.Code != http.StatusNotFound {
		t.Errorf("status = %d, want 404", w.Code)
	}
	var resp map[string]string
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if resp["error"] != "session not found" {
		t.Errorf("error = %q, want %q", resp["error"], "session not found")
	}
}

// TS-09-E5: Invalid JSON body → HTTP 400
// Requirement: 09-REQ-2.E3
func TestInvalidJSON(t *testing.T) {
	s := store.NewStore()
	mux := newTestMux(s)

	endpoints := []struct {
		method string
		path   string
	}{
		{"POST", "/parking/start"},
		{"POST", "/parking/stop"},
	}

	for _, ep := range endpoints {
		req := httptest.NewRequest(ep.method, ep.path, strings.NewReader("not json"))
		w := httptest.NewRecorder()
		mux.ServeHTTP(w, req)

		if w.Code != http.StatusBadRequest {
			t.Errorf("%s %s: status = %d, want 400", ep.method, ep.path, w.Code)
		}
		var resp map[string]string
		if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
			t.Fatalf("%s %s: decode: %v", ep.method, ep.path, err)
		}
		if resp["error"] != "invalid request body" {
			t.Errorf("%s %s: error = %q, want %q", ep.method, ep.path, resp["error"], "invalid request body")
		}
	}
}
