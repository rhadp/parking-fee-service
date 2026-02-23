package main

import (
	"encoding/json"
	"fmt"
	"math"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-04-29: Mock PARKING_OPERATOR HTTP server configurable port
// Requirement: 04-REQ-8.1
// ---------------------------------------------------------------------------

func TestOperator_ConfigurablePort(t *testing.T) {
	// The mock operator should create a router that responds to /health.
	// In the full implementation, the server listens on a configurable port.
	// Here we test the router via httptest.
	mux := NewRouter()
	srv := httptest.NewServer(mux)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/health")
	if err != nil {
		t.Fatalf("failed to reach mock operator: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200 from /health, got %d", resp.StatusCode)
	}

	var body map[string]string
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode /health response: %v", err)
	}
	if body["status"] != "ok" {
		t.Errorf("expected status 'ok', got %q", body["status"])
	}
}

// ---------------------------------------------------------------------------
// TS-04-30: POST /parking/start creates session
// Requirement: 04-REQ-8.2
// ---------------------------------------------------------------------------

func TestOperator_StartSession(t *testing.T) {
	mux := NewRouter()
	srv := httptest.NewServer(mux)
	defer srv.Close()

	reqBody := `{"vehicle_id":"VIN12345","zone_id":"zone-munich-central","timestamp":1708700000}`
	resp, err := http.Post(srv.URL+"/parking/start", "application/json", strings.NewReader(reqBody))
	if err != nil {
		t.Fatalf("failed to POST /parking/start: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	sessionID, ok := body["session_id"].(string)
	if !ok || sessionID == "" {
		t.Error("expected non-empty session_id in response")
	}

	status, ok := body["status"].(string)
	if !ok || status != "active" {
		t.Errorf("expected status 'active', got %q", status)
	}
}

// ---------------------------------------------------------------------------
// TS-04-31: POST /parking/stop calculates fee
// Requirement: 04-REQ-8.3
// ---------------------------------------------------------------------------

func TestOperator_StopSession(t *testing.T) {
	mux := NewRouter()
	srv := httptest.NewServer(mux)
	defer srv.Close()

	// Start a session first
	startBody := fmt.Sprintf(
		`{"vehicle_id":"VIN12345","zone_id":"zone-munich-central","timestamp":%d}`,
		time.Now().Unix())
	startResp, err := http.Post(srv.URL+"/parking/start", "application/json", strings.NewReader(startBody))
	if err != nil {
		t.Fatalf("failed to POST /parking/start: %v", err)
	}
	defer startResp.Body.Close()

	if startResp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200 from start, got %d", startResp.StatusCode)
	}

	var startResult map[string]interface{}
	if err := json.NewDecoder(startResp.Body).Decode(&startResult); err != nil {
		t.Fatalf("failed to decode start response: %v", err)
	}
	sessionID := startResult["session_id"].(string)

	// Brief delay so duration > 0
	time.Sleep(100 * time.Millisecond)

	// Stop the session
	stopBody := fmt.Sprintf(`{"session_id":"%s"}`, sessionID)
	stopResp, err := http.Post(srv.URL+"/parking/stop", "application/json", strings.NewReader(stopBody))
	if err != nil {
		t.Fatalf("failed to POST /parking/stop: %v", err)
	}
	defer stopResp.Body.Close()

	if stopResp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200 from stop, got %d", stopResp.StatusCode)
	}

	var stopResult map[string]interface{}
	if err := json.NewDecoder(stopResp.Body).Decode(&stopResult); err != nil {
		t.Fatalf("failed to decode stop response: %v", err)
	}

	if stopResult["session_id"] != sessionID {
		t.Errorf("expected session_id %q, got %v", sessionID, stopResult["session_id"])
	}
	if fee, ok := stopResult["fee"].(float64); !ok || fee < 0 {
		t.Errorf("expected fee >= 0, got %v", stopResult["fee"])
	}
	if dur, ok := stopResult["duration_seconds"].(float64); !ok || dur < 0 {
		t.Errorf("expected duration_seconds >= 0, got %v", stopResult["duration_seconds"])
	}
	if currency, ok := stopResult["currency"].(string); !ok || currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %v", stopResult["currency"])
	}
}

// ---------------------------------------------------------------------------
// TS-04-32: GET /parking/{session_id}/status returns session status
// Requirement: 04-REQ-8.4
// ---------------------------------------------------------------------------

func TestOperator_SessionStatus(t *testing.T) {
	mux := NewRouter()
	srv := httptest.NewServer(mux)
	defer srv.Close()

	// Start a session
	startBody := `{"vehicle_id":"VIN12345","zone_id":"zone-munich-central","timestamp":1708700000}`
	startResp, err := http.Post(srv.URL+"/parking/start", "application/json", strings.NewReader(startBody))
	if err != nil {
		t.Fatalf("failed to POST /parking/start: %v", err)
	}
	defer startResp.Body.Close()

	if startResp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200 from start, got %d", startResp.StatusCode)
	}

	var startResult map[string]interface{}
	if err := json.NewDecoder(startResp.Body).Decode(&startResult); err != nil {
		t.Fatalf("failed to decode start response: %v", err)
	}
	sessionID := startResult["session_id"].(string)

	// Get status
	statusResp, err := http.Get(srv.URL + "/parking/" + sessionID + "/status")
	if err != nil {
		t.Fatalf("failed to GET /parking/{id}/status: %v", err)
	}
	defer statusResp.Body.Close()

	if statusResp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200 from status, got %d", statusResp.StatusCode)
	}

	var statusResult map[string]interface{}
	if err := json.NewDecoder(statusResp.Body).Decode(&statusResult); err != nil {
		t.Fatalf("failed to decode status response: %v", err)
	}

	if statusResult["session_id"] != sessionID {
		t.Errorf("expected session_id %q, got %v", sessionID, statusResult["session_id"])
	}
	if active, ok := statusResult["active"].(bool); !ok || !active {
		t.Errorf("expected active=true, got %v", statusResult["active"])
	}
	if startTime, ok := statusResult["start_time"].(float64); !ok || startTime <= 0 {
		t.Errorf("expected start_time > 0, got %v", statusResult["start_time"])
	}
	if fee, ok := statusResult["current_fee"].(float64); !ok || fee < 0 {
		t.Errorf("expected current_fee >= 0, got %v", statusResult["current_fee"])
	}
	if currency, ok := statusResult["currency"].(string); !ok || currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %v", statusResult["currency"])
	}
}

// ---------------------------------------------------------------------------
// TS-04-33: GET /rate/{zone_id} returns zone rate
// Requirement: 04-REQ-8.5
// ---------------------------------------------------------------------------

func TestOperator_ZoneRate(t *testing.T) {
	mux := NewRouter()
	srv := httptest.NewServer(mux)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/rate/zone-munich-central")
	if err != nil {
		t.Fatalf("failed to GET /rate/zone-munich-central: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}

	var body map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if rate, ok := body["rate_per_hour"].(float64); !ok || rate != 2.50 {
		t.Errorf("expected rate_per_hour 2.50, got %v", body["rate_per_hour"])
	}
	if currency, ok := body["currency"].(string); !ok || currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %v", body["currency"])
	}
	if zoneName, ok := body["zone_name"].(string); !ok || zoneName != "Munich Central" {
		t.Errorf("expected zone_name 'Munich Central', got %v", body["zone_name"])
	}
}

// ---------------------------------------------------------------------------
// TS-04-E14: Mock operator stop unknown session returns 404
// Requirement: 04-REQ-8.E1
// ---------------------------------------------------------------------------

func TestEdge_StopUnknownSession404(t *testing.T) {
	mux := NewRouter()
	srv := httptest.NewServer(mux)
	defer srv.Close()

	body := `{"session_id":"nonexistent-id"}`
	resp, err := http.Post(srv.URL+"/parking/stop", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatalf("failed to POST /parking/stop: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Fatalf("expected 404 for unknown session, got %d", resp.StatusCode)
	}
}

// ---------------------------------------------------------------------------
// TS-04-E15: Mock operator status unknown session returns 404
// Requirement: 04-REQ-8.E2
// ---------------------------------------------------------------------------

func TestEdge_StatusUnknownSession404(t *testing.T) {
	mux := NewRouter()
	srv := httptest.NewServer(mux)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/parking/nonexistent-id/status")
	if err != nil {
		t.Fatalf("failed to GET /parking/nonexistent-id/status: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Fatalf("expected 404 for unknown session status, got %d", resp.StatusCode)
	}
}

// ---------------------------------------------------------------------------
// TS-04-E16: Mock operator rate unknown zone returns 404
// Requirement: 04-REQ-8.E3
// ---------------------------------------------------------------------------

func TestEdge_RateUnknownZone404(t *testing.T) {
	mux := NewRouter()
	srv := httptest.NewServer(mux)
	defer srv.Close()

	resp, err := http.Get(srv.URL + "/rate/unknown-zone-id")
	if err != nil {
		t.Fatalf("failed to GET /rate/unknown-zone-id: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Fatalf("expected 404 for unknown zone, got %d", resp.StatusCode)
	}
}

// ---------------------------------------------------------------------------
// TS-04-P7: Mock Operator Fee Accuracy (property test)
// Property: For any parking session, fee == rate_per_hour * (duration_seconds / 3600.0)
// Validates: 04-REQ-8.3
// ---------------------------------------------------------------------------

func TestProperty_FeeAccuracy(t *testing.T) {
	mux := NewRouter()
	srv := httptest.NewServer(mux)
	defer srv.Close()

	zones := []struct {
		zoneID       string
		expectedRate float64
	}{
		{"zone-munich-central", 2.50},
		{"zone-munich-west", 1.50},
	}

	for _, zone := range zones {
		t.Run(zone.zoneID, func(t *testing.T) {
			// Verify the rate
			rateResp, err := http.Get(srv.URL + "/rate/" + zone.zoneID)
			if err != nil {
				t.Fatalf("failed to GET rate: %v", err)
			}
			defer rateResp.Body.Close()

			if rateResp.StatusCode != http.StatusOK {
				t.Fatalf("expected 200 from rate, got %d", rateResp.StatusCode)
			}

			var rateBody map[string]interface{}
			if err := json.NewDecoder(rateResp.Body).Decode(&rateBody); err != nil {
				t.Fatalf("failed to decode rate: %v", err)
			}
			rate := rateBody["rate_per_hour"].(float64)
			if rate != zone.expectedRate {
				t.Errorf("expected rate %f, got %f", zone.expectedRate, rate)
			}

			// Start a session
			now := time.Now().Unix()
			startBody := fmt.Sprintf(
				`{"vehicle_id":"VIN12345","zone_id":"%s","timestamp":%d}`,
				zone.zoneID, now)
			startResp, err := http.Post(srv.URL+"/parking/start", "application/json",
				strings.NewReader(startBody))
			if err != nil {
				t.Fatalf("failed to start session: %v", err)
			}
			defer startResp.Body.Close()

			if startResp.StatusCode != http.StatusOK {
				t.Fatalf("expected 200 from start, got %d", startResp.StatusCode)
			}

			var startResult map[string]interface{}
			if err := json.NewDecoder(startResp.Body).Decode(&startResult); err != nil {
				t.Fatalf("failed to decode start: %v", err)
			}
			sessionID := startResult["session_id"].(string)

			// Wait a known duration
			time.Sleep(200 * time.Millisecond)

			// Stop the session
			stopBody := fmt.Sprintf(`{"session_id":"%s"}`, sessionID)
			stopResp, err := http.Post(srv.URL+"/parking/stop", "application/json",
				strings.NewReader(stopBody))
			if err != nil {
				t.Fatalf("failed to stop session: %v", err)
			}
			defer stopResp.Body.Close()

			if stopResp.StatusCode != http.StatusOK {
				t.Fatalf("expected 200 from stop, got %d", stopResp.StatusCode)
			}

			var stopResult map[string]interface{}
			if err := json.NewDecoder(stopResp.Body).Decode(&stopResult); err != nil {
				t.Fatalf("failed to decode stop: %v", err)
			}

			fee := stopResult["fee"].(float64)
			durationSec := stopResult["duration_seconds"].(float64)

			// Property: fee == rate * (duration_seconds / 3600.0)
			expectedFee := rate * (durationSec / 3600.0)
			if math.Abs(fee-expectedFee) > 0.01 {
				t.Errorf("fee accuracy: expected %.4f, got %.4f (rate=%.2f, duration=%.0fs)",
					expectedFee, fee, rate, durationSec)
			}
		})
	}
}
