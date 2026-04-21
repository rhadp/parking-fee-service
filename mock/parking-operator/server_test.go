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

// uuidRegex matches the UUID format returned as session_id.
var uuidRegex = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)

// postJSON posts a JSON body to the given URL path on the test server.
func postJSON(t *testing.T, ts *httptest.Server, path, body string) *http.Response {
	t.Helper()
	resp, err := http.Post(ts.URL+path, "application/json", strings.NewReader(body))
	if err != nil {
		t.Fatalf("POST %s: %v", path, err)
	}
	return resp
}

// getRequest sends a GET request to the given URL path on the test server.
func getRequest(t *testing.T, ts *httptest.Server, path string) *http.Response {
	t.Helper()
	resp, err := http.Get(ts.URL + path)
	if err != nil {
		t.Fatalf("GET %s: %v", path, err)
	}
	return resp
}

// decodeBody decodes the JSON response body into a map.
func decodeBody(t *testing.T, resp *http.Response) map[string]interface{} {
	t.Helper()
	var m map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&m); err != nil {
		t.Fatalf("decode response body: %v", err)
	}
	resp.Body.Close()
	return m
}

// TS-09-14: POST /parking/start returns session with UUID session_id, status "active",
// and rate {rate_type: "per_hour", amount: 2.50, currency: "EUR"}.
func TestStartSession(t *testing.T) {
	s := NewServer()
	ts := httptest.NewServer(s)
	defer ts.Close()

	body := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	resp := postJSON(t, ts, "/parking/start", body)

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected HTTP 200, got %d", resp.StatusCode)
	}

	m := decodeBody(t, resp)

	sessionID, ok := m["session_id"].(string)
	if !ok || sessionID == "" {
		t.Errorf("expected non-empty session_id string, got %v", m["session_id"])
	} else if !uuidRegex.MatchString(sessionID) {
		t.Errorf("expected UUID-format session_id, got %q", sessionID)
	}

	if status, _ := m["status"].(string); status != "active" {
		t.Errorf("expected status=active, got %q", status)
	}

	rate, ok := m["rate"].(map[string]interface{})
	if !ok {
		t.Errorf("expected rate object, got %v", m["rate"])
	} else {
		if amount, _ := rate["amount"].(float64); amount != 2.50 {
			t.Errorf("expected rate.amount=2.50, got %v", rate["amount"])
		}
		if currency, _ := rate["currency"].(string); currency != "EUR" {
			t.Errorf("expected rate.currency=EUR, got %q", currency)
		}
	}
}

// TS-09-15: POST /parking/stop returns duration_seconds and total_amount.
func TestStopSession(t *testing.T) {
	s := NewServer()
	ts := httptest.NewServer(s)
	defer ts.Close()

	// First start a session
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	startResp := postJSON(t, ts, "/parking/start", startBody)
	startData := decodeBody(t, startResp)

	sessionID, _ := startData["session_id"].(string)
	if sessionID == "" {
		t.Fatal("start did not return a session_id")
	}

	// Stop the session 1 hour later
	stopBody := `{"session_id":"` + sessionID + `","timestamp":1700003600}`
	stopResp := postJSON(t, ts, "/parking/stop", stopBody)

	if stopResp.StatusCode != http.StatusOK {
		t.Errorf("expected HTTP 200, got %d", stopResp.StatusCode)
	}

	stopData := decodeBody(t, stopResp)

	// duration_seconds should be 3600 (1 hour)
	duration, ok := stopData["duration_seconds"].(float64)
	if !ok {
		t.Errorf("expected duration_seconds field, got %v", stopData["duration_seconds"])
	} else if uint64(duration) != 3600 {
		t.Errorf("expected duration_seconds=3600, got %v", duration)
	}

	// total_amount should be 2.50 (2.50/hr * 1hr)
	totalAmount, ok := stopData["total_amount"].(float64)
	if !ok {
		t.Errorf("expected total_amount field, got %v", stopData["total_amount"])
	} else if abs64(totalAmount-2.50) > 0.01 {
		t.Errorf("expected total_amount≈2.50, got %v", totalAmount)
	}

	if currency, _ := stopData["currency"].(string); currency != "EUR" {
		t.Errorf("expected currency=EUR, got %q", currency)
	}

	if status, _ := stopData["status"].(string); status != "stopped" {
		t.Errorf("expected status=stopped, got %q", status)
	}
}

// TS-09-16: GET /parking/status/{session_id} returns session state.
func TestSessionStatus(t *testing.T) {
	s := NewServer()
	ts := httptest.NewServer(s)
	defer ts.Close()

	// Start a session first
	startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":1700000000}`
	startResp := postJSON(t, ts, "/parking/start", startBody)
	startData := decodeBody(t, startResp)

	sessionID, _ := startData["session_id"].(string)
	if sessionID == "" {
		t.Fatal("start did not return a session_id")
	}

	// Query status
	statusResp := getRequest(t, ts, "/parking/status/"+sessionID)

	if statusResp.StatusCode != http.StatusOK {
		t.Errorf("expected HTTP 200, got %d", statusResp.StatusCode)
	}

	statusData := decodeBody(t, statusResp)

	if id, _ := statusData["session_id"].(string); id != sessionID {
		t.Errorf("expected session_id=%q, got %q", sessionID, id)
	}

	if status, _ := statusData["status"].(string); status != "active" {
		t.Errorf("expected status=active, got %q", status)
	}
}

// TS-09-E7: POST /parking/stop with unknown session_id returns HTTP 404.
func TestStopUnknownSession(t *testing.T) {
	s := NewServer()
	ts := httptest.NewServer(s)
	defer ts.Close()

	body := `{"session_id":"nonexistent-session-id","timestamp":1700000000}`
	resp := postJSON(t, ts, "/parking/stop", body)

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected HTTP 404 for unknown session, got %d", resp.StatusCode)
	}
}

// TS-09-E8: GET /parking/status/{session_id} with unknown session_id returns HTTP 404.
func TestStatusUnknownSession(t *testing.T) {
	s := NewServer()
	ts := httptest.NewServer(s)
	defer ts.Close()

	resp := getRequest(t, ts, "/parking/status/nonexistent")

	if resp.StatusCode != http.StatusNotFound {
		t.Errorf("expected HTTP 404 for unknown session, got %d", resp.StatusCode)
	}
}

// TS-09-E9: POST /parking/start with malformed JSON body returns HTTP 400.
func TestMalformedRequest(t *testing.T) {
	s := NewServer()
	ts := httptest.NewServer(s)
	defer ts.Close()

	// Send non-JSON body
	req, err := http.NewRequest(http.MethodPost, ts.URL+"/parking/start", bytes.NewBufferString("not valid json"))
	if err != nil {
		t.Fatalf("create request: %v", err)
	}
	req.Header.Set("Content-Type", "text/plain")

	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		t.Fatalf("POST /parking/start: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusBadRequest {
		t.Errorf("expected HTTP 400 for malformed body, got %d", resp.StatusCode)
	}
}

// TS-09-P3: Property test — duration_seconds == stop - start and total_amount == 2.50 * duration_hours.
func TestSessionIntegrityProperty(t *testing.T) {
	s := NewServer()
	ts := httptest.NewServer(s)
	defer ts.Close()

	testCases := []struct {
		startTS  int64
		duration int64 // seconds
	}{
		{1700000000, 3600},  // 1 hour
		{1700000000, 7200},  // 2 hours
		{1700000000, 1800},  // 30 minutes
		{1700000000, 900},   // 15 minutes
		{1700000000, 36000}, // 10 hours
		{1699000000, 3600},
		{1701000000, 5400},  // 1.5 hours
		{1702000000, 120},   // 2 minutes
		{1703000000, 86400}, // 24 hours
		{1704000000, 3661},  // 1 hour 1 minute 1 second
	}

	for _, tc := range testCases {
		startBody := `{"vehicle_id":"VIN001","zone_id":"zone-1","timestamp":` + itoa(tc.startTS) + `}`
		startResp := postJSON(t, ts, "/parking/start", startBody)
		startData := decodeBody(t, startResp)

		sessionID, _ := startData["session_id"].(string)
		if sessionID == "" {
			t.Fatalf("start did not return session_id (startTS=%d)", tc.startTS)
		}

		stopTS := tc.startTS + tc.duration
		stopBody := `{"session_id":"` + sessionID + `","timestamp":` + itoa(stopTS) + `}`
		stopResp := postJSON(t, ts, "/parking/stop", stopBody)
		stopData := decodeBody(t, stopResp)

		gotDuration, _ := stopData["duration_seconds"].(float64)
		if uint64(gotDuration) != uint64(tc.duration) {
			t.Errorf("duration mismatch: want %d, got %v (startTS=%d)", tc.duration, gotDuration, tc.startTS)
		}

		expectedAmount := 2.50 * (float64(tc.duration) / 3600.0)
		gotAmount, _ := stopData["total_amount"].(float64)
		if abs64(gotAmount-expectedAmount) > 0.01 {
			t.Errorf("total_amount mismatch: want %.4f, got %v (duration=%d)", expectedAmount, gotAmount, tc.duration)
		}
	}
}

// TS-09-P4/P5: Property test — all session_ids are unique UUID format.
func TestSessionIDUniqueness(t *testing.T) {
	s := NewServer()
	ts := httptest.NewServer(s)
	defer ts.Close()

	seen := make(map[string]bool)
	for i := 0; i < 20; i++ {
		body := `{"vehicle_id":"V` + itoa(int64(i)) + `","zone_id":"zone-1","timestamp":` + itoa(int64(i)) + `}`
		resp := postJSON(t, ts, "/parking/start", body)

		if resp.StatusCode != http.StatusOK {
			t.Fatalf("start[%d] returned HTTP %d", i, resp.StatusCode)
		}

		data := decodeBody(t, resp)
		sessionID, _ := data["session_id"].(string)

		if sessionID == "" {
			t.Errorf("iteration %d: empty session_id", i)
			continue
		}
		if !uuidRegex.MatchString(sessionID) {
			t.Errorf("iteration %d: session_id %q is not UUID format", i, sessionID)
		}
		if seen[sessionID] {
			t.Errorf("iteration %d: duplicate session_id %q", i, sessionID)
		}
		seen[sessionID] = true
	}

	if len(seen) != 20 {
		t.Errorf("expected 20 unique session IDs, got %d", len(seen))
	}
}

// abs64 returns the absolute value of a float64.
func abs64(x float64) float64 {
	if x < 0 {
		return -x
	}
	return x
}

// itoa converts an int64 to a string (avoids importing strconv).
func itoa(n int64) string {
	if n == 0 {
		return "0"
	}
	negative := n < 0
	if negative {
		n = -n
	}
	var buf [20]byte
	pos := len(buf)
	for n > 0 {
		pos--
		buf[pos] = byte('0' + n%10)
		n /= 10
	}
	if negative {
		pos--
		buf[pos] = '-'
	}
	return string(buf[pos:])
}
