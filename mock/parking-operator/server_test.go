package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"regexp"
	"strings"
	"testing"
)

var uuidRegex = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)

// TS-09-14: POST /parking/start creates session with UUID and rate.
func TestStartSession(t *testing.T) {
	srv := NewServer()
	ts := httptest.NewServer(srv.Handler())
	defer ts.Close()

	body := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	resp, err := http.Post(ts.URL+"/parking/start", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected 200, got %d", resp.StatusCode)
	}
	var session Session
	if err := json.NewDecoder(resp.Body).Decode(&session); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if session.Status != "active" {
		t.Errorf("expected status 'active', got %q", session.Status)
	}
	if session.Rate.Amount != 2.50 {
		t.Errorf("expected rate.amount 2.50, got %f", session.Rate.Amount)
	}
	if session.Rate.Currency != "EUR" {
		t.Errorf("expected rate.currency 'EUR', got %q", session.Rate.Currency)
	}
	if session.Rate.RateType != "per_hour" {
		t.Errorf("expected rate.rate_type 'per_hour', got %q", session.Rate.RateType)
	}
	if !uuidRegex.MatchString(session.SessionID) {
		t.Errorf("expected UUID-format session_id, got %q", session.SessionID)
	}
}

// TS-09-15: POST /parking/stop returns duration and total_amount.
func TestStopSession(t *testing.T) {
	srv := NewServer()
	ts := httptest.NewServer(srv.Handler())
	defer ts.Close()

	// Start a session first
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	startResp, err := http.Post(ts.URL+"/parking/start", "application/json", strings.NewReader(startBody))
	if err != nil {
		t.Fatal(err)
	}
	defer startResp.Body.Close()
	if startResp.StatusCode != http.StatusOK {
		t.Fatalf("start: expected 200, got %d", startResp.StatusCode)
	}
	var startSession Session
	if err := json.NewDecoder(startResp.Body).Decode(&startSession); err != nil {
		t.Fatalf("failed to decode start response: %v", err)
	}

	// Stop the session 1 hour later
	stopBody, _ := json.Marshal(map[string]any{
		"session_id": startSession.SessionID,
		"timestamp":  1700003600, // 1700000000 + 3600 = 1 hour
	})
	stopResp, err := http.Post(ts.URL+"/parking/stop", "application/json", bytes.NewReader(stopBody))
	if err != nil {
		t.Fatal(err)
	}
	defer stopResp.Body.Close()
	if stopResp.StatusCode != http.StatusOK {
		t.Fatalf("stop: expected 200, got %d", stopResp.StatusCode)
	}
	var stopped Session
	if err := json.NewDecoder(stopResp.Body).Decode(&stopped); err != nil {
		t.Fatalf("failed to decode stop response: %v", err)
	}
	if stopped.Status != "stopped" {
		t.Errorf("expected status 'stopped', got %q", stopped.Status)
	}
	if stopped.Duration != 3600 {
		t.Errorf("expected duration_seconds 3600, got %d", stopped.Duration)
	}
	// total_amount = 2.50 * (3600/3600) = 2.50
	if stopped.TotalAmt < 2.49 || stopped.TotalAmt > 2.51 {
		t.Errorf("expected total_amount ~2.50, got %f", stopped.TotalAmt)
	}
	if stopped.Currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %q", stopped.Currency)
	}
}

// TS-09-16: GET /parking/status/{session_id} returns session state.
func TestSessionStatus(t *testing.T) {
	srv := NewServer()
	ts := httptest.NewServer(srv.Handler())
	defer ts.Close()

	// Start a session
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	startResp, err := http.Post(ts.URL+"/parking/start", "application/json", strings.NewReader(startBody))
	if err != nil {
		t.Fatal(err)
	}
	defer startResp.Body.Close()
	if startResp.StatusCode != http.StatusOK {
		t.Fatalf("start: expected 200, got %d", startResp.StatusCode)
	}
	var startSession Session
	if err := json.NewDecoder(startResp.Body).Decode(&startSession); err != nil {
		t.Fatalf("failed to decode start response: %v", err)
	}

	// Get status
	statusResp, err := http.Get(ts.URL + "/parking/status/" + startSession.SessionID)
	if err != nil {
		t.Fatal(err)
	}
	defer statusResp.Body.Close()
	if statusResp.StatusCode != http.StatusOK {
		t.Fatalf("status: expected 200, got %d", statusResp.StatusCode)
	}
	var session Session
	if err := json.NewDecoder(statusResp.Body).Decode(&session); err != nil {
		t.Fatalf("failed to decode status response: %v", err)
	}
	if session.SessionID != startSession.SessionID {
		t.Errorf("expected session_id %q, got %q", startSession.SessionID, session.SessionID)
	}
	if session.Status != "active" {
		t.Errorf("expected status 'active', got %q", session.Status)
	}
}

// TS-09-E7: POST /parking/stop with unknown session_id returns 404.
func TestStopUnknownSession(t *testing.T) {
	srv := NewServer()
	ts := httptest.NewServer(srv.Handler())
	defer ts.Close()

	body := `{"session_id":"nonexistent","timestamp":1700000000}`
	resp, err := http.Post(ts.URL+"/parking/stop", "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected 404 for unknown session, got %d", resp.StatusCode)
	}
}

// TS-09-E8: GET /parking/status with unknown session_id returns 404.
func TestStatusUnknownSession(t *testing.T) {
	srv := NewServer()
	ts := httptest.NewServer(srv.Handler())
	defer ts.Close()

	resp, err := http.Get(ts.URL + "/parking/status/nonexistent")
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected 404 for unknown session, got %d", resp.StatusCode)
	}
}

// TS-09-E9: POST /parking/start with malformed body returns 400.
func TestMalformedRequest(t *testing.T) {
	srv := NewServer()
	ts := httptest.NewServer(srv.Handler())
	defer ts.Close()

	resp, err := http.Post(ts.URL+"/parking/start", "text/plain", strings.NewReader("not valid json"))
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("expected 400 for malformed request, got %d", resp.StatusCode)
	}
}

// TS-09-E9: POST /parking/stop with malformed body returns 400.
func TestMalformedStopRequest(t *testing.T) {
	srv := NewServer()
	ts := httptest.NewServer(srv.Handler())
	defer ts.Close()

	resp, err := http.Post(ts.URL+"/parking/stop", "text/plain", strings.NewReader("not valid json"))
	if err != nil {
		t.Fatal(err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("expected 400 for malformed stop request, got %d", resp.StatusCode)
	}
}

// TS-09-P3: Session integrity property: duration and total_amount are calculated correctly.
func TestSessionIntegrityProperty(t *testing.T) {
	cases := []struct {
		startTS  int64
		stopTS   int64
		wantDur  uint64
		wantAmt  float64
	}{
		{1700000000, 1700003600, 3600, 2.50},
		{1700000000, 1700007200, 7200, 5.00},
		{1700000000, 1700001800, 1800, 1.25},
		{1700000000, 1700000001, 1, 2.50 / 3600.0},
	}
	for _, tc := range cases {
		srv := NewServer()
		ts := httptest.NewServer(srv.Handler())

		startBody, _ := json.Marshal(map[string]any{
			"vehicle_id": "V1", "zone_id": "z1", "timestamp": tc.startTS,
		})
		startResp, err := http.Post(ts.URL+"/parking/start", "application/json", bytes.NewReader(startBody))
		if err != nil {
			ts.Close()
			t.Fatalf("start failed: %v", err)
		}
		if startResp.StatusCode != http.StatusOK {
			ts.Close()
			t.Fatalf("start: expected 200, got %d", startResp.StatusCode)
		}
		var sess Session
		if err := json.NewDecoder(startResp.Body).Decode(&sess); err != nil {
			ts.Close()
			t.Fatalf("decode start: %v", err)
		}
		startResp.Body.Close()

		stopBody, _ := json.Marshal(map[string]any{
			"session_id": sess.SessionID,
			"timestamp":  tc.stopTS,
		})
		stopResp, err := http.Post(ts.URL+"/parking/stop", "application/json", bytes.NewReader(stopBody))
		if err != nil {
			ts.Close()
			t.Fatalf("stop failed: %v", err)
		}
		if stopResp.StatusCode != http.StatusOK {
			ts.Close()
			t.Fatalf("stop: expected 200, got %d, start=%d stop=%d", stopResp.StatusCode, tc.startTS, tc.stopTS)
		}
		var stopped Session
		if err := json.NewDecoder(stopResp.Body).Decode(&stopped); err != nil {
			ts.Close()
			t.Fatalf("decode stop: %v", err)
		}
		stopResp.Body.Close()
		ts.Close()

		if stopped.Duration != tc.wantDur {
			t.Errorf("duration: want %d, got %d (start=%d stop=%d)", tc.wantDur, stopped.Duration, tc.startTS, tc.stopTS)
		}
		diff := stopped.TotalAmt - tc.wantAmt
		if diff < -0.01 || diff > 0.01 {
			t.Errorf("total_amount: want ~%f, got %f (start=%d stop=%d)", tc.wantAmt, stopped.TotalAmt, tc.startTS, tc.stopTS)
		}
	}
}

// TS-09-P5: Session ID uniqueness: 10 start requests produce unique UUID session_ids.
func TestSessionIDUniqueness(t *testing.T) {
	srv := NewServer()
	ts := httptest.NewServer(srv.Handler())
	defer ts.Close()

	ids := make(map[string]bool)
	for i := range 10 {
		body, _ := json.Marshal(map[string]any{
			"vehicle_id": "V1", "zone_id": "z1", "timestamp": int64(i + 1),
		})
		resp, err := http.Post(ts.URL+"/parking/start", "application/json", bytes.NewReader(body))
		if err != nil {
			t.Fatalf("request %d failed: %v", i, err)
		}
		if resp.StatusCode != http.StatusOK {
			resp.Body.Close()
			t.Fatalf("request %d: expected 200, got %d", i, resp.StatusCode)
		}
		var sess Session
		if err := json.NewDecoder(resp.Body).Decode(&sess); err != nil {
			resp.Body.Close()
			t.Fatalf("decode %d: %v", i, err)
		}
		resp.Body.Close()

		if !uuidRegex.MatchString(sess.SessionID) {
			t.Errorf("request %d: session_id %q is not UUID format", i, sess.SessionID)
		}
		if ids[sess.SessionID] {
			t.Errorf("request %d: duplicate session_id %q", i, sess.SessionID)
		}
		ids[sess.SessionID] = true
	}
}
