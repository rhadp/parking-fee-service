package server

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"regexp"
	"strings"
	"testing"
)

// UUID regex pattern for validating session IDs.
var uuidRegex = regexp.MustCompile(
	`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`,
)

// ---------------------------------------------------------------------------
// TS-09-14: Parking Operator Start Session
// Requirement: 09-REQ-8.2
// ---------------------------------------------------------------------------

func TestStartSession(t *testing.T) {
	t.Helper()

	srv := New()
	ts := httptest.NewServer(srv)
	defer ts.Close()

	body := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	resp, err := http.Post(ts.URL+"/parking/start", "application/json",
		strings.NewReader(body))
	if err != nil {
		t.Fatalf("POST /parking/start failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200, got %d", resp.StatusCode)
	}

	var result map[string]any
	data, _ := io.ReadAll(resp.Body)
	if err := json.Unmarshal(data, &result); err != nil {
		t.Fatalf("failed to parse response JSON: %v\nbody: %s", err, string(data))
	}

	// Verify session_id is a UUID
	sessionID, ok := result["session_id"].(string)
	if !ok || sessionID == "" {
		t.Errorf("expected session_id string, got %v", result["session_id"])
	}
	if !uuidRegex.MatchString(sessionID) {
		t.Errorf("session_id %q does not match UUID format", sessionID)
	}

	// Verify status is "active"
	if status, _ := result["status"].(string); status != "active" {
		t.Errorf("expected status 'active', got %q", status)
	}

	// Verify rate fields
	rate, ok := result["rate"].(map[string]any)
	if !ok {
		t.Fatalf("expected rate object, got %v", result["rate"])
	}
	if amount, _ := rate["amount"].(float64); amount != 2.50 {
		t.Errorf("expected rate.amount 2.50, got %v", amount)
	}
	if currency, _ := rate["currency"].(string); currency != "EUR" {
		t.Errorf("expected rate.currency 'EUR', got %q", currency)
	}
}

// ---------------------------------------------------------------------------
// TS-09-15: Parking Operator Stop Session
// Requirement: 09-REQ-8.3
// ---------------------------------------------------------------------------

func TestStopSession(t *testing.T) {
	t.Helper()

	srv := New()
	ts := httptest.NewServer(srv)
	defer ts.Close()

	// First, start a session
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	startResp, err := http.Post(ts.URL+"/parking/start", "application/json",
		strings.NewReader(startBody))
	if err != nil {
		t.Fatalf("POST /parking/start failed: %v", err)
	}
	defer startResp.Body.Close()

	if startResp.StatusCode != http.StatusOK {
		t.Fatalf("start session: expected status 200, got %d", startResp.StatusCode)
	}

	var startResult map[string]any
	startData, _ := io.ReadAll(startResp.Body)
	if err := json.Unmarshal(startData, &startResult); err != nil {
		t.Fatalf("failed to parse start response: %v", err)
	}

	sessionID, _ := startResult["session_id"].(string)
	if sessionID == "" {
		t.Fatal("start response missing session_id")
	}

	// Now stop the session (1 hour later)
	stopBody := fmt.Sprintf(`{"session_id":"%s","timestamp":1700003600}`, sessionID)
	stopResp, err := http.Post(ts.URL+"/parking/stop", "application/json",
		strings.NewReader(stopBody))
	if err != nil {
		t.Fatalf("POST /parking/stop failed: %v", err)
	}
	defer stopResp.Body.Close()

	if stopResp.StatusCode != http.StatusOK {
		t.Fatalf("stop session: expected status 200, got %d", stopResp.StatusCode)
	}

	var stopResult map[string]any
	stopData, _ := io.ReadAll(stopResp.Body)
	if err := json.Unmarshal(stopData, &stopResult); err != nil {
		t.Fatalf("failed to parse stop response: %v", err)
	}

	// Verify duration_seconds = 3600 (1 hour)
	duration, _ := stopResult["duration_seconds"].(float64)
	if duration != 3600 {
		t.Errorf("expected duration_seconds 3600, got %v", duration)
	}

	// Verify total_amount = 2.50 (2.50/hr * 1hr)
	totalAmount, _ := stopResult["total_amount"].(float64)
	if totalAmount != 2.50 {
		t.Errorf("expected total_amount 2.50, got %v", totalAmount)
	}

	// Verify currency
	if currency, _ := stopResult["currency"].(string); currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %q", currency)
	}

	// Verify status is "stopped"
	if status, _ := stopResult["status"].(string); status != "stopped" {
		t.Errorf("expected status 'stopped', got %q", status)
	}
}

// ---------------------------------------------------------------------------
// TS-09-16: Parking Operator Session Status
// Requirement: 09-REQ-8.4
// ---------------------------------------------------------------------------

func TestSessionStatus(t *testing.T) {
	t.Helper()

	srv := New()
	ts := httptest.NewServer(srv)
	defer ts.Close()

	// Start a session
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	startResp, err := http.Post(ts.URL+"/parking/start", "application/json",
		strings.NewReader(startBody))
	if err != nil {
		t.Fatalf("POST /parking/start failed: %v", err)
	}
	defer startResp.Body.Close()

	if startResp.StatusCode != http.StatusOK {
		t.Fatalf("start session: expected status 200, got %d", startResp.StatusCode)
	}

	var startResult map[string]any
	startData, _ := io.ReadAll(startResp.Body)
	if err := json.Unmarshal(startData, &startResult); err != nil {
		t.Fatalf("failed to parse start response: %v", err)
	}

	sessionID, _ := startResult["session_id"].(string)
	if sessionID == "" {
		t.Fatal("start response missing session_id")
	}

	// Query status
	statusResp, err := http.Get(ts.URL + "/parking/status/" + sessionID)
	if err != nil {
		t.Fatalf("GET /parking/status failed: %v", err)
	}
	defer statusResp.Body.Close()

	if statusResp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200, got %d", statusResp.StatusCode)
	}

	var statusResult map[string]any
	statusData, _ := io.ReadAll(statusResp.Body)
	if err := json.Unmarshal(statusData, &statusResult); err != nil {
		t.Fatalf("failed to parse status response: %v", err)
	}

	if sid, _ := statusResult["session_id"].(string); sid != sessionID {
		t.Errorf("expected session_id %q, got %q", sessionID, sid)
	}
	if status, _ := statusResult["status"].(string); status != "active" {
		t.Errorf("expected status 'active', got %q", status)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E7: Parking Operator Stop Unknown Session
// Requirement: 09-REQ-8.E1
// ---------------------------------------------------------------------------

func TestStopUnknownSession(t *testing.T) {
	t.Helper()

	srv := New()
	ts := httptest.NewServer(srv)
	defer ts.Close()

	body := `{"session_id":"nonexistent","timestamp":1700000000}`
	resp, err := http.Post(ts.URL+"/parking/stop", "application/json",
		strings.NewReader(body))
	if err != nil {
		t.Fatalf("POST /parking/stop failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", resp.StatusCode)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E8: Parking Operator Status Unknown Session
// Requirement: 09-REQ-8.E2
// ---------------------------------------------------------------------------

func TestStatusUnknownSession(t *testing.T) {
	t.Helper()

	srv := New()
	ts := httptest.NewServer(srv)
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/parking/status/nonexistent")
	if err != nil {
		t.Fatalf("GET /parking/status/nonexistent failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", resp.StatusCode)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E9: Parking Operator Malformed Request
// Requirement: 09-REQ-8.E3
// ---------------------------------------------------------------------------

func TestMalformedRequest(t *testing.T) {
	t.Helper()

	srv := New()
	ts := httptest.NewServer(srv)
	defer ts.Close()

	resp, err := http.Post(ts.URL+"/parking/start", "text/plain",
		bytes.NewReader([]byte("not valid json")))
	if err != nil {
		t.Fatalf("POST /parking/start with malformed body failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", resp.StatusCode)
	}
}

// ---------------------------------------------------------------------------
// TS-09-P4 / TS-09-P5: Parking Operator Session Uniqueness
// Property 5 from design.md
// Requirement: 09-REQ-8.2, 09-REQ-8.5
// ---------------------------------------------------------------------------

func TestSessionUniqueness(t *testing.T) {
	t.Helper()

	srv := New()
	ts := httptest.NewServer(srv)
	defer ts.Close()

	ids := make(map[string]bool)
	for i := 0; i < 100; i++ {
		body := fmt.Sprintf(
			`{"vehicle_id":"V%d","zone_id":"z1","timestamp":%d}`, i, i+1,
		)
		resp, err := http.Post(ts.URL+"/parking/start", "application/json",
			strings.NewReader(body))
		if err != nil {
			t.Fatalf("POST /parking/start #%d failed: %v", i, err)
		}

		if resp.StatusCode != http.StatusOK {
			resp.Body.Close()
			t.Fatalf("POST /parking/start #%d: expected 200, got %d", i, resp.StatusCode)
		}

		var result map[string]any
		data, _ := io.ReadAll(resp.Body)
		resp.Body.Close()
		if err := json.Unmarshal(data, &result); err != nil {
			t.Fatalf("failed to parse response #%d: %v", i, err)
		}

		sessionID, _ := result["session_id"].(string)
		if !uuidRegex.MatchString(sessionID) {
			t.Errorf("session #%d: %q does not match UUID format", i, sessionID)
		}
		if ids[sessionID] {
			t.Errorf("session #%d: duplicate session_id %q", i, sessionID)
		}
		ids[sessionID] = true
	}

	if len(ids) != 100 {
		t.Errorf("expected 100 unique session IDs, got %d", len(ids))
	}
}

// ---------------------------------------------------------------------------
// TS-09-P3: Parking Operator Session Integrity
// Property 4 from design.md
// Requirement: 09-REQ-8.2, 09-REQ-8.3, 09-REQ-8.5
// ---------------------------------------------------------------------------

func TestSessionIntegrity(t *testing.T) {
	t.Helper()

	srv := New()
	ts := httptest.NewServer(srv)
	defer ts.Close()

	testCases := []struct {
		startTS          int64
		stopTS           int64
		expectedDuration int64
		expectedAmount   float64
	}{
		{1700000000, 1700003600, 3600, 2.50},     // 1 hour
		{1700000000, 1700007200, 7200, 5.00},     // 2 hours
		{1700000000, 1700001800, 1800, 1.25},     // 30 minutes
		{1700000000, 1700000060, 60, 2.50 / 60},  // 1 minute
	}

	for i, tc := range testCases {
		startBody := fmt.Sprintf(
			`{"vehicle_id":"V%d","zone_id":"z1","timestamp":%d}`, i, tc.startTS,
		)
		startResp, err := http.Post(ts.URL+"/parking/start", "application/json",
			strings.NewReader(startBody))
		if err != nil {
			t.Fatalf("case %d: POST /parking/start failed: %v", i, err)
		}

		if startResp.StatusCode != http.StatusOK {
			startResp.Body.Close()
			t.Fatalf("case %d: start expected 200, got %d", i, startResp.StatusCode)
		}

		var startResult map[string]any
		startData, _ := io.ReadAll(startResp.Body)
		startResp.Body.Close()
		if err := json.Unmarshal(startData, &startResult); err != nil {
			t.Fatalf("case %d: failed to parse start response: %v", i, err)
		}

		sessionID, _ := startResult["session_id"].(string)

		stopBody := fmt.Sprintf(`{"session_id":"%s","timestamp":%d}`, sessionID, tc.stopTS)
		stopResp, err := http.Post(ts.URL+"/parking/stop", "application/json",
			strings.NewReader(stopBody))
		if err != nil {
			t.Fatalf("case %d: POST /parking/stop failed: %v", i, err)
		}

		if stopResp.StatusCode != http.StatusOK {
			stopResp.Body.Close()
			t.Fatalf("case %d: stop expected 200, got %d", i, stopResp.StatusCode)
		}

		var stopResult map[string]any
		stopData, _ := io.ReadAll(stopResp.Body)
		stopResp.Body.Close()
		if err := json.Unmarshal(stopData, &stopResult); err != nil {
			t.Fatalf("case %d: failed to parse stop response: %v", i, err)
		}

		duration, _ := stopResult["duration_seconds"].(float64)
		if int64(duration) != tc.expectedDuration {
			t.Errorf("case %d: expected duration %d, got %v",
				i, tc.expectedDuration, duration)
		}

		totalAmount, _ := stopResult["total_amount"].(float64)
		diff := totalAmount - tc.expectedAmount
		if diff < -0.01 || diff > 0.01 {
			t.Errorf("case %d: expected total_amount %.4f, got %.4f",
				i, tc.expectedAmount, totalAmount)
		}
	}
}
