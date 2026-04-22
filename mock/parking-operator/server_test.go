package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"regexp"
	"strings"
	"sync"
	"testing"
)

// uuidRegex matches UUID v4 format: 8-4-4-4-12 hex digits.
var uuidRegex = regexp.MustCompile(
	`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`,
)

// ---------------------------------------------------------------------------
// TS-09-14: Parking Operator Start Session
// Requirement: 09-REQ-8.2, 09-REQ-8.5
// ---------------------------------------------------------------------------

func TestStartSession(t *testing.T) {
	srv := NewServer()
	handler := srv.Handler()

	body := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	req := httptest.NewRequest("POST", "/parking/start", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to parse response: %v", err)
	}

	sessionID, ok := resp["session_id"].(string)
	if !ok || sessionID == "" {
		t.Fatal("expected non-empty session_id in response")
	}

	if !uuidRegex.MatchString(sessionID) {
		t.Errorf("expected session_id to be UUID format, got: %s", sessionID)
	}

	if resp["status"] != "active" {
		t.Errorf("expected status 'active', got: %v", resp["status"])
	}

	rate, ok := resp["rate"].(map[string]interface{})
	if !ok {
		t.Fatal("expected 'rate' object in response")
	}

	if rate["amount"] != 2.5 {
		t.Errorf("expected rate amount 2.50, got: %v", rate["amount"])
	}

	if rate["currency"] != "EUR" {
		t.Errorf("expected rate currency 'EUR', got: %v", rate["currency"])
	}
}

// ---------------------------------------------------------------------------
// TS-09-15: Parking Operator Stop Session
// Requirement: 09-REQ-8.3
// ---------------------------------------------------------------------------

func TestStopSession(t *testing.T) {
	srv := NewServer()
	handler := srv.Handler()

	// First start a session.
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	startReq := httptest.NewRequest("POST", "/parking/start", strings.NewReader(startBody))
	startReq.Header.Set("Content-Type", "application/json")
	startRec := httptest.NewRecorder()
	handler.ServeHTTP(startRec, startReq)

	if startRec.Code != http.StatusOK {
		t.Fatalf("start: expected 200, got %d", startRec.Code)
	}

	var startResp map[string]interface{}
	json.Unmarshal(startRec.Body.Bytes(), &startResp)
	sessionID := startResp["session_id"].(string)

	// Now stop the session (1 hour later).
	stopBody := fmt.Sprintf(`{"session_id":"%s","timestamp":1700003600}`, sessionID)
	stopReq := httptest.NewRequest("POST", "/parking/stop", strings.NewReader(stopBody))
	stopReq.Header.Set("Content-Type", "application/json")
	stopRec := httptest.NewRecorder()
	handler.ServeHTTP(stopRec, stopReq)

	if stopRec.Code != http.StatusOK {
		t.Fatalf("stop: expected 200, got %d: %s", stopRec.Code, stopRec.Body.String())
	}

	var stopResp map[string]interface{}
	json.Unmarshal(stopRec.Body.Bytes(), &stopResp)

	// duration_seconds should be 3600 (1 hour).
	if dur, ok := stopResp["duration_seconds"].(float64); !ok || int(dur) != 3600 {
		t.Errorf("expected duration_seconds 3600, got: %v", stopResp["duration_seconds"])
	}

	// total_amount should be 2.50 (2.50/hr * 1hr).
	if amt, ok := stopResp["total_amount"].(float64); !ok || amt != 2.50 {
		t.Errorf("expected total_amount 2.50, got: %v", stopResp["total_amount"])
	}

	if stopResp["currency"] != "EUR" {
		t.Errorf("expected currency 'EUR', got: %v", stopResp["currency"])
	}
}

// ---------------------------------------------------------------------------
// TS-09-16: Parking Operator Session Status
// Requirement: 09-REQ-8.4
// ---------------------------------------------------------------------------

func TestSessionStatus(t *testing.T) {
	srv := NewServer()
	handler := srv.Handler()

	// Start a session.
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	startReq := httptest.NewRequest("POST", "/parking/start", strings.NewReader(startBody))
	startReq.Header.Set("Content-Type", "application/json")
	startRec := httptest.NewRecorder()
	handler.ServeHTTP(startRec, startReq)

	if startRec.Code != http.StatusOK {
		t.Fatalf("start: expected 200, got %d", startRec.Code)
	}

	var startResp map[string]interface{}
	json.Unmarshal(startRec.Body.Bytes(), &startResp)
	sessionID := startResp["session_id"].(string)

	// Query status.
	statusReq := httptest.NewRequest("GET", "/parking/status/"+sessionID, nil)
	statusRec := httptest.NewRecorder()
	handler.ServeHTTP(statusRec, statusReq)

	if statusRec.Code != http.StatusOK {
		t.Fatalf("status: expected 200, got %d: %s", statusRec.Code, statusRec.Body.String())
	}

	var statusResp map[string]interface{}
	json.Unmarshal(statusRec.Body.Bytes(), &statusResp)

	if statusResp["session_id"] != sessionID {
		t.Errorf("expected session_id '%s', got: %v", sessionID, statusResp["session_id"])
	}

	if statusResp["status"] != "active" {
		t.Errorf("expected status 'active', got: %v", statusResp["status"])
	}
}

// ---------------------------------------------------------------------------
// TS-09-E7: Parking Operator Stop Unknown Session
// Requirement: 09-REQ-8.E1
// ---------------------------------------------------------------------------

func TestStopUnknownSession(t *testing.T) {
	srv := NewServer()
	handler := srv.Handler()

	body := `{"session_id":"nonexistent","timestamp":1700000000}`
	req := httptest.NewRequest("POST", "/parking/stop", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("expected 404 for unknown session, got %d", rec.Code)
	}

	// Response body should contain a JSON error message.
	var errResp map[string]interface{}
	if err := json.Unmarshal(rec.Body.Bytes(), &errResp); err != nil {
		t.Errorf("expected JSON error body, got: %s", rec.Body.String())
	}
}

// ---------------------------------------------------------------------------
// TS-09-E8: Parking Operator Status Unknown Session
// Requirement: 09-REQ-8.E2
// ---------------------------------------------------------------------------

func TestStatusUnknownSession(t *testing.T) {
	srv := NewServer()
	handler := srv.Handler()

	req := httptest.NewRequest("GET", "/parking/status/nonexistent", nil)
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("expected 404 for unknown session, got %d", rec.Code)
	}

	// Response body should contain a JSON error message.
	var errResp map[string]interface{}
	if err := json.Unmarshal(rec.Body.Bytes(), &errResp); err != nil {
		t.Errorf("expected JSON error body, got: %s", rec.Body.String())
	}
}

// ---------------------------------------------------------------------------
// TS-09-E9: Parking Operator Malformed Request
// Requirement: 09-REQ-8.E3
// ---------------------------------------------------------------------------

func TestMalformedRequest(t *testing.T) {
	srv := NewServer()
	handler := srv.Handler()

	req := httptest.NewRequest("POST", "/parking/start", strings.NewReader("not valid json"))
	req.Header.Set("Content-Type", "text/plain")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected 400 for malformed request, got %d", rec.Code)
	}
}

// ---------------------------------------------------------------------------
// TS-09-P3: Session Integrity Property (timestamps and billing)
// Property 4 from design.md
// Requirement: 09-REQ-8.2, 09-REQ-8.3, 09-REQ-8.5
// ---------------------------------------------------------------------------

func TestSessionIntegrityProperty(t *testing.T) {
	// Test with various start/stop timestamp combinations.
	cases := []struct {
		startTS  int64
		duration int64 // seconds
	}{
		{1700000000, 3600},   // 1 hour
		{1700000000, 7200},   // 2 hours
		{1700000000, 1800},   // 30 min
		{1700000000, 60},     // 1 min
		{1700000000, 1},      // 1 second
		{1700000000, 36000},  // 10 hours
		{1600000000, 86400},  // 1 day
		{1700000000, 900},    // 15 min
		{1700000000, 5400},   // 1.5 hours
		{1700000000, 10800},  // 3 hours
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("duration_%ds", tc.duration), func(t *testing.T) {
			srv := NewServer()
			handler := srv.Handler()

			startBody := fmt.Sprintf(
				`{"vehicle_id":"V1","zone_id":"z1","timestamp":%d}`, tc.startTS)
			startReq := httptest.NewRequest("POST", "/parking/start",
				strings.NewReader(startBody))
			startReq.Header.Set("Content-Type", "application/json")
			startRec := httptest.NewRecorder()
			handler.ServeHTTP(startRec, startReq)

			if startRec.Code != http.StatusOK {
				t.Fatalf("start: expected 200, got %d", startRec.Code)
			}

			var startResp map[string]interface{}
			json.Unmarshal(startRec.Body.Bytes(), &startResp)
			sessionID := startResp["session_id"].(string)

			stopTS := tc.startTS + tc.duration
			stopBody := fmt.Sprintf(
				`{"session_id":"%s","timestamp":%d}`, sessionID, stopTS)
			stopReq := httptest.NewRequest("POST", "/parking/stop",
				strings.NewReader(stopBody))
			stopReq.Header.Set("Content-Type", "application/json")
			stopRec := httptest.NewRecorder()
			handler.ServeHTTP(stopRec, stopReq)

			if stopRec.Code != http.StatusOK {
				t.Fatalf("stop: expected 200, got %d", stopRec.Code)
			}

			var stopResp map[string]interface{}
			json.Unmarshal(stopRec.Body.Bytes(), &stopResp)

			gotDuration := int64(stopResp["duration_seconds"].(float64))
			if gotDuration != tc.duration {
				t.Errorf("expected duration %d, got %d", tc.duration, gotDuration)
			}

			expectedAmount := 2.50 * (float64(tc.duration) / 3600.0)
			gotAmount := stopResp["total_amount"].(float64)
			if diff := gotAmount - expectedAmount; diff > 0.01 || diff < -0.01 {
				t.Errorf("expected total_amount ~%.4f, got %.4f",
					expectedAmount, gotAmount)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-09-P4: Session ID Uniqueness (sequential)
// Property 5 from design.md
// Requirement: 09-REQ-8.2, 09-REQ-8.5
// ---------------------------------------------------------------------------

func TestSessionIDUniqueness(t *testing.T) {
	srv := NewServer()
	handler := srv.Handler()

	ids := make(map[string]bool)

	for i := 0; i < 100; i++ {
		body := fmt.Sprintf(
			`{"vehicle_id":"V%d","zone_id":"z1","timestamp":%d}`, i, int64(i))
		req := httptest.NewRequest("POST", "/parking/start",
			strings.NewReader(body))
		req.Header.Set("Content-Type", "application/json")
		rec := httptest.NewRecorder()
		handler.ServeHTTP(rec, req)

		if rec.Code != http.StatusOK {
			t.Fatalf("request %d: expected 200, got %d", i, rec.Code)
		}

		var resp map[string]interface{}
		json.Unmarshal(rec.Body.Bytes(), &resp)
		sessionID := resp["session_id"].(string)

		if !uuidRegex.MatchString(sessionID) {
			t.Errorf("request %d: session_id not UUID format: %s", i, sessionID)
		}

		if ids[sessionID] {
			t.Errorf("request %d: duplicate session_id: %s", i, sessionID)
		}
		ids[sessionID] = true
	}
}

// ---------------------------------------------------------------------------
// TS-09-P5: Concurrent Session Uniqueness
// Property 5 from design.md
// Requirement: 09-REQ-8.2, 09-REQ-8.5
// ---------------------------------------------------------------------------

func TestConcurrentSessionUniqueness(t *testing.T) {
	srv := NewServer()
	handler := srv.Handler()

	const n = 50
	results := make([]string, n)
	errors := make([]error, n)

	var wg sync.WaitGroup
	for i := 0; i < n; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			body := fmt.Sprintf(
				`{"vehicle_id":"V%d","zone_id":"z1","timestamp":%d}`, idx, int64(idx))
			req := httptest.NewRequest("POST", "/parking/start",
				strings.NewReader(body))
			req.Header.Set("Content-Type", "application/json")
			rec := httptest.NewRecorder()
			handler.ServeHTTP(rec, req)

			if rec.Code != http.StatusOK {
				errors[idx] = fmt.Errorf("request %d: got %d", idx, rec.Code)
				return
			}

			var resp map[string]interface{}
			if err := json.Unmarshal(rec.Body.Bytes(), &resp); err != nil {
				errors[idx] = fmt.Errorf("request %d: json error: %v", idx, err)
				return
			}
			sid, ok := resp["session_id"].(string)
			if !ok {
				errors[idx] = fmt.Errorf("request %d: no session_id", idx)
				return
			}
			results[idx] = sid
		}(i)
	}
	wg.Wait()

	for i, err := range errors {
		if err != nil {
			t.Errorf("goroutine %d: %v", i, err)
		}
	}

	ids := make(map[string]bool)
	for i, id := range results {
		if id == "" {
			continue // error already reported
		}
		if !uuidRegex.MatchString(id) {
			t.Errorf("request %d: not UUID format: %s", i, id)
		}
		if ids[id] {
			t.Errorf("duplicate session_id: %s", id)
		}
		ids[id] = true
	}

	if len(ids) != n {
		t.Errorf("expected %d unique session_ids, got %d", n, len(ids))
	}
}
