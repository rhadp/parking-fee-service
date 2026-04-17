// Unit tests for the parking-operator HTTP handlers.
//
// RED PHASE: stub handlers return empty JSON so every assertion about
// session_id, status, rate, duration, etc. will fail until task group 3
// implements the real session lifecycle.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"regexp"
	"testing"
)

// uuidRE matches canonical UUID v4 format.
var uuidRE = regexp.MustCompile(`(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)

func newTestServer() *server {
	return newServer()
}

// ── TS-09-14: POST /parking/start ─────────────────────────────────────────

// TestStartSession verifies that POST /parking/start returns HTTP 200 with a
// UUID session_id, status "active", and rate {amount:2.50, currency:"EUR"}.
func TestStartSession(t *testing.T) {
	s := newTestServer()
	body := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	req := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	rr := httptest.NewRecorder()

	s.handleStart(rr, req)

	if rr.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr.Code)
	}

	var resp map[string]any
	if err := json.NewDecoder(rr.Body).Decode(&resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}

	sessionID, _ := resp["session_id"].(string)
	if !uuidRE.MatchString(sessionID) {
		t.Fatalf("expected UUID session_id, got: %q", sessionID)
	}
	if resp["status"] != "active" {
		t.Fatalf("expected status=active, got: %v", resp["status"])
	}

	rate, ok := resp["rate"].(map[string]any)
	if !ok {
		t.Fatal("expected rate object in response")
	}
	if rate["amount"] != 2.5 {
		t.Fatalf("expected rate.amount=2.5, got: %v", rate["amount"])
	}
	if rate["currency"] != "EUR" {
		t.Fatalf("expected rate.currency=EUR, got: %v", rate["currency"])
	}
	if rate["rate_type"] != "per_hour" {
		t.Fatalf("expected rate.rate_type=per_hour, got: %v", rate["rate_type"])
	}
}

// ── TS-09-P5: session uniqueness ──────────────────────────────────────────

// TestSessionIDUniqueness verifies that repeated POST /parking/start calls
// each return a distinct UUID session_id (TS-09-P5).
func TestSessionIDUniqueness(t *testing.T) {
	s := newTestServer()
	seen := make(map[string]bool)

	for i := range 20 {
		body := `{"vehicle_id":"V","zone_id":"z","timestamp":1700000000}`
		req := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString(body))
		req.Header.Set("Content-Type", "application/json")
		rr := httptest.NewRecorder()
		s.handleStart(rr, req)

		var resp map[string]any
		json.NewDecoder(rr.Body).Decode(&resp) //nolint
		id, _ := resp["session_id"].(string)

		if !uuidRE.MatchString(id) {
			t.Fatalf("iteration %d: expected UUID, got: %q", i, id)
		}
		if seen[id] {
			t.Fatalf("iteration %d: duplicate session_id %q", i, id)
		}
		seen[id] = true
	}
}

// ── TS-09-15: POST /parking/stop ──────────────────────────────────────────

// TestStopSession verifies that POST /parking/stop returns HTTP 200 with
// correct duration_seconds and total_amount (rate * duration_hours).
func TestStopSession(t *testing.T) {
	s := newTestServer()

	// Create a session first.
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	req := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString(startBody))
	req.Header.Set("Content-Type", "application/json")
	rr := httptest.NewRecorder()
	s.handleStart(rr, req)

	var startResp map[string]any
	json.NewDecoder(rr.Body).Decode(&startResp) //nolint
	sessionID, _ := startResp["session_id"].(string)
	if sessionID == "" {
		t.Skip("TestStopSession skipped: TestStartSession must pass first")
	}

	// Stop the session 1 hour later.
	stopBody, _ := json.Marshal(map[string]any{
		"session_id": sessionID,
		"timestamp":  1700003600, // start + 3600s
	})
	req2 := httptest.NewRequest(http.MethodPost, "/parking/stop", bytes.NewBuffer(stopBody))
	req2.Header.Set("Content-Type", "application/json")
	rr2 := httptest.NewRecorder()
	s.handleStop(rr2, req2)

	if rr2.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr2.Code)
	}

	var stopResp map[string]any
	json.NewDecoder(rr2.Body).Decode(&stopResp) //nolint

	if stopResp["status"] != "stopped" {
		t.Fatalf("expected status=stopped, got: %v", stopResp["status"])
	}
	// duration_seconds must be 3600
	dur, ok := stopResp["duration_seconds"].(float64)
	if !ok || dur != 3600 {
		t.Fatalf("expected duration_seconds=3600, got: %v", stopResp["duration_seconds"])
	}
	// total_amount = 2.50 EUR/hr * 1 hr = 2.50
	amt, ok := stopResp["total_amount"].(float64)
	if !ok || amt < 2.49 || amt > 2.51 {
		t.Fatalf("expected total_amount≈2.50, got: %v", stopResp["total_amount"])
	}
	if stopResp["currency"] != "EUR" {
		t.Fatalf("expected currency=EUR, got: %v", stopResp["currency"])
	}
}

// ── TS-09-16: GET /parking/status/{session_id} ────────────────────────────

// TestSessionStatus verifies that GET /parking/status/{id} returns the
// current session state with the correct session_id and status.
func TestSessionStatus(t *testing.T) {
	s := newTestServer()

	// Create a session.
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	req := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString(startBody))
	req.Header.Set("Content-Type", "application/json")
	rr := httptest.NewRecorder()
	s.handleStart(rr, req)

	var startResp map[string]any
	json.NewDecoder(rr.Body).Decode(&startResp) //nolint
	sessionID, _ := startResp["session_id"].(string)
	if sessionID == "" {
		t.Skip("TestSessionStatus skipped: TestStartSession must pass first")
	}

	// Query status.
	req2 := httptest.NewRequest(http.MethodGet, "/parking/status/"+sessionID, nil)
	req2.SetPathValue("session_id", sessionID)
	rr2 := httptest.NewRecorder()
	s.handleStatus(rr2, req2)

	if rr2.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", rr2.Code)
	}

	var statusResp map[string]any
	json.NewDecoder(rr2.Body).Decode(&statusResp) //nolint

	if statusResp["session_id"] != sessionID {
		t.Fatalf("expected session_id=%q, got: %v", sessionID, statusResp["session_id"])
	}
	if statusResp["status"] != "active" {
		t.Fatalf("expected status=active, got: %v", statusResp["status"])
	}
}

// ── TS-09-E7: POST /parking/stop — unknown session ────────────────────────

// TestStopUnknownSession verifies that stopping an unknown session returns 404.
func TestStopUnknownSession(t *testing.T) {
	s := newTestServer()
	body := `{"session_id":"nonexistent-session","timestamp":1700000000}`
	req := httptest.NewRequest(http.MethodPost, "/parking/stop", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	rr := httptest.NewRecorder()

	s.handleStop(rr, req)

	if rr.Code != http.StatusNotFound {
		t.Fatalf("expected 404 for unknown session_id, got %d", rr.Code)
	}
}

// ── TS-09-E8: GET /parking/status — unknown session ───────────────────────

// TestStatusUnknownSession verifies that querying an unknown session returns 404.
func TestStatusUnknownSession(t *testing.T) {
	s := newTestServer()
	req := httptest.NewRequest(http.MethodGet, "/parking/status/nonexistent", nil)
	req.SetPathValue("session_id", "nonexistent")
	rr := httptest.NewRecorder()

	s.handleStatus(rr, req)

	if rr.Code != http.StatusNotFound {
		t.Fatalf("expected 404 for unknown session_id, got %d", rr.Code)
	}
}

// ── TS-09-E9: POST /parking/start — malformed body ───────────────────────

// TestMalformedRequestStart verifies that a malformed POST body to /parking/start
// returns 400.
func TestMalformedRequestStart(t *testing.T) {
	s := newTestServer()
	req := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString("not valid json"))
	req.Header.Set("Content-Type", "text/plain")
	rr := httptest.NewRecorder()

	s.handleStart(rr, req)

	if rr.Code != http.StatusBadRequest {
		t.Fatalf("expected 400 for malformed body, got %d", rr.Code)
	}
}

// TestMalformedRequestStop verifies that a malformed POST body to /parking/stop
// returns 400.
func TestMalformedRequestStop(t *testing.T) {
	s := newTestServer()
	req := httptest.NewRequest(http.MethodPost, "/parking/stop", bytes.NewBufferString("not valid json"))
	req.Header.Set("Content-Type", "text/plain")
	rr := httptest.NewRecorder()

	s.handleStop(rr, req)

	if rr.Code != http.StatusBadRequest {
		t.Fatalf("expected 400 for malformed body, got %d", rr.Code)
	}
}

// ── TS-09-P3: Session Integrity Property ──────────────────────────────────

// TestSessionIntegrityProperty verifies that for any start-stop pair,
// duration_seconds == stop_timestamp - start_timestamp and
// total_amount ≈ 2.50 * duration_hours.
//
// Property: Property 4 from design.md
// Test Spec: TS-09-P3
// Requirements: 09-REQ-8.2, 09-REQ-8.3, 09-REQ-8.5
func TestSessionIntegrityProperty(t *testing.T) {
	cases := []struct {
		startTS   int64
		durationS int64
	}{
		{1700000000, 3600},  // 1 hour
		{1700000000, 1800},  // 30 minutes
		{1700000000, 7200},  // 2 hours
		{1700000000, 60},    // 1 minute
		{1700000000, 86400}, // 24 hours
		{1699999999, 3601},  // non-round timestamps
		{1000000000, 12345}, // arbitrary
		{1700000000, 1},     // 1 second
		{1700000000, 100},   // ~1.67 minutes
		{1700000000, 10800}, // 3 hours
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("start=%d_dur=%d", tc.startTS, tc.durationS), func(t *testing.T) {
			s := newTestServer()
			stopTS := tc.startTS + tc.durationS

			// Create session.
			startBody, _ := json.Marshal(map[string]any{
				"vehicle_id": "VIN001",
				"zone_id":    "zone-1",
				"timestamp":  tc.startTS,
			})
			startReq := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBuffer(startBody))
			startReq.Header.Set("Content-Type", "application/json")
			startRR := httptest.NewRecorder()
			s.handleStart(startRR, startReq)
			if startRR.Code != http.StatusOK {
				t.Fatalf("handleStart: expected 200, got %d", startRR.Code)
			}

			var startResp map[string]any
			json.NewDecoder(startRR.Body).Decode(&startResp) //nolint
			sessionID, _ := startResp["session_id"].(string)
			if sessionID == "" {
				t.Fatal("handleStart: no session_id in response")
			}

			// Stop session.
			stopBody, _ := json.Marshal(map[string]any{
				"session_id": sessionID,
				"timestamp":  stopTS,
			})
			stopReq := httptest.NewRequest(http.MethodPost, "/parking/stop", bytes.NewBuffer(stopBody))
			stopReq.Header.Set("Content-Type", "application/json")
			stopRR := httptest.NewRecorder()
			s.handleStop(stopRR, stopReq)
			if stopRR.Code != http.StatusOK {
				t.Fatalf("handleStop: expected 200, got %d", stopRR.Code)
			}

			var stopResp map[string]any
			json.NewDecoder(stopRR.Body).Decode(&stopResp) //nolint

			// Verify duration.
			dur, _ := stopResp["duration_seconds"].(float64)
			if int64(dur) != tc.durationS {
				t.Errorf("duration_seconds: expected %d, got %v", tc.durationS, dur)
			}

			// Verify total_amount = 2.50 * duration_hours (within 1 cent).
			amt, _ := stopResp["total_amount"].(float64)
			expected := 2.50 * float64(tc.durationS) / 3600.0
			if amt < expected-0.01 || amt > expected+0.01 {
				t.Errorf("total_amount: expected ≈%.4f, got %v", expected, amt)
			}

			// Verify currency.
			if stopResp["currency"] != "EUR" {
				t.Errorf("currency: expected EUR, got %v", stopResp["currency"])
			}
		})
	}
}
