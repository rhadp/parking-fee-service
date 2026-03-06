package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func setupHandler() *Handler {
	store := NewSessionStore()
	return NewHandler(store)
}

// TS-09-7: POST /parking/start with valid body creates a session.
func TestStartSession_Valid(t *testing.T) {
	h := setupHandler()
	body := `{"vehicle_id":"VIN001","zone_id":"muc-central","timestamp":1709640000}`
	req := httptest.NewRequest(http.MethodPost, "/parking/start", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()

	h.HandleStartParking(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d: %s", rec.Code, rec.Body.String())
	}

	ct := rec.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Fatalf("expected Content-Type application/json, got %s", ct)
	}

	var resp StartResponse
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if resp.SessionID == "" {
		t.Fatal("expected non-empty session_id")
	}
	if resp.Status != "active" {
		t.Fatalf("expected status 'active', got '%s'", resp.Status)
	}
}

// TS-09-8: POST /parking/stop with valid session_id completes the session.
func TestStopSession_Valid(t *testing.T) {
	h := setupHandler()

	// Start a session first.
	startBody := `{"vehicle_id":"VIN001","zone_id":"muc-central","timestamp":1709640000}`
	startReq := httptest.NewRequest(http.MethodPost, "/parking/start", strings.NewReader(startBody))
	startReq.Header.Set("Content-Type", "application/json")
	startRec := httptest.NewRecorder()
	h.HandleStartParking(startRec, startReq)

	if startRec.Code != http.StatusOK {
		t.Fatalf("start: expected status 200, got %d: %s", startRec.Code, startRec.Body.String())
	}

	var startResp StartResponse
	if err := json.NewDecoder(startRec.Body).Decode(&startResp); err != nil {
		t.Fatalf("start: failed to decode response: %v", err)
	}

	// Stop the session.
	stopBody := `{"session_id":"` + startResp.SessionID + `"}`
	stopReq := httptest.NewRequest(http.MethodPost, "/parking/stop", strings.NewReader(stopBody))
	stopReq.Header.Set("Content-Type", "application/json")
	stopRec := httptest.NewRecorder()
	h.HandleStopParking(stopRec, stopReq)

	if stopRec.Code != http.StatusOK {
		t.Fatalf("stop: expected status 200, got %d: %s", stopRec.Code, stopRec.Body.String())
	}

	var stopResp StopResponse
	if err := json.NewDecoder(stopRec.Body).Decode(&stopResp); err != nil {
		t.Fatalf("stop: failed to decode response: %v", err)
	}

	if stopResp.SessionID != startResp.SessionID {
		t.Fatalf("expected session_id '%s', got '%s'", startResp.SessionID, stopResp.SessionID)
	}
	if stopResp.DurationSeconds < 0 {
		t.Fatalf("expected non-negative duration, got %d", stopResp.DurationSeconds)
	}
	if stopResp.Fee < 0 {
		t.Fatalf("expected non-negative fee, got %f", stopResp.Fee)
	}
	if stopResp.Status != "completed" {
		t.Fatalf("expected status 'completed', got '%s'", stopResp.Status)
	}
}

// TS-09-9: GET /parking/status returns all sessions including started ones.
func TestGetStatus_ReturnsAllSessions(t *testing.T) {
	h := setupHandler()

	// Start a session.
	startBody := `{"vehicle_id":"VIN001","zone_id":"muc-central","timestamp":1709640000}`
	startReq := httptest.NewRequest(http.MethodPost, "/parking/start", strings.NewReader(startBody))
	startReq.Header.Set("Content-Type", "application/json")
	startRec := httptest.NewRecorder()
	h.HandleStartParking(startRec, startReq)

	if startRec.Code != http.StatusOK {
		t.Fatalf("start: expected status 200, got %d", startRec.Code)
	}

	var startResp StartResponse
	if err := json.NewDecoder(startRec.Body).Decode(&startResp); err != nil {
		t.Fatalf("start: failed to decode: %v", err)
	}

	// Get status.
	statusReq := httptest.NewRequest(http.MethodGet, "/parking/status", nil)
	statusRec := httptest.NewRecorder()
	h.HandleParkingStatus(statusRec, statusReq)

	if statusRec.Code != http.StatusOK {
		t.Fatalf("status: expected 200, got %d: %s", statusRec.Code, statusRec.Body.String())
	}

	var sessions []Session
	if err := json.NewDecoder(statusRec.Body).Decode(&sessions); err != nil {
		t.Fatalf("status: failed to decode: %v", err)
	}

	if len(sessions) < 1 {
		t.Fatal("expected at least one session")
	}

	found := false
	for _, s := range sessions {
		if s.SessionID == startResp.SessionID {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("expected session %s in status list", startResp.SessionID)
	}
}

// TS-09-E9: GET /parking/status returns empty array when no sessions exist.
func TestGetStatus_EmptyWhenNoSessions(t *testing.T) {
	h := setupHandler()

	req := httptest.NewRequest(http.MethodGet, "/parking/status", nil)
	rec := httptest.NewRecorder()
	h.HandleParkingStatus(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", rec.Code, rec.Body.String())
	}

	body := strings.TrimSpace(rec.Body.String())
	if body != "[]" {
		t.Fatalf("expected empty JSON array '[]', got '%s'", body)
	}
}

// TS-09-E7: POST /parking/start with malformed JSON returns 400.
func TestStartSession_MalformedBody(t *testing.T) {
	h := setupHandler()

	req := httptest.NewRequest(http.MethodPost, "/parking/start", strings.NewReader("{invalid json"))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()

	h.HandleStartParking(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Fatalf("expected status 400, got %d: %s", rec.Code, rec.Body.String())
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResp.Error == "" {
		t.Fatal("expected non-empty error message")
	}
}

// TS-09-E8: POST /parking/stop with unknown session_id returns 404.
func TestStopSession_UnknownSession(t *testing.T) {
	h := setupHandler()

	body := `{"session_id":"nonexistent-session"}`
	req := httptest.NewRequest(http.MethodPost, "/parking/stop", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	rec := httptest.NewRecorder()

	h.HandleStopParking(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Fatalf("expected status 404, got %d: %s", rec.Code, rec.Body.String())
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(rec.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}
	if errResp.Error == "" {
		t.Fatal("expected non-empty error message")
	}
}

// TS-09-P8: Session store consistency -- start/stop/double-stop.
func TestSessionStoreConsistency(t *testing.T) {
	h := setupHandler()

	// Start a session.
	startBody := `{"vehicle_id":"VIN002","zone_id":"muc-west","timestamp":1709640000}`
	startReq := httptest.NewRequest(http.MethodPost, "/parking/start", strings.NewReader(startBody))
	startReq.Header.Set("Content-Type", "application/json")
	startRec := httptest.NewRecorder()
	h.HandleStartParking(startRec, startReq)

	if startRec.Code != http.StatusOK {
		t.Fatalf("start: expected 200, got %d", startRec.Code)
	}

	var startResp StartResponse
	if err := json.NewDecoder(startRec.Body).Decode(&startResp); err != nil {
		t.Fatalf("start: decode error: %v", err)
	}

	// Verify it shows as active in status.
	statusReq := httptest.NewRequest(http.MethodGet, "/parking/status", nil)
	statusRec := httptest.NewRecorder()
	h.HandleParkingStatus(statusRec, statusReq)

	if statusRec.Code != http.StatusOK {
		t.Fatalf("status: expected 200, got %d", statusRec.Code)
	}

	var sessions []Session
	if err := json.NewDecoder(statusRec.Body).Decode(&sessions); err != nil {
		t.Fatalf("status: decode error: %v", err)
	}

	found := false
	for _, s := range sessions {
		if s.SessionID == startResp.SessionID && s.Status == "active" {
			found = true
			break
		}
	}
	if !found {
		t.Fatal("expected started session with status 'active' in status list")
	}

	// Stop the session.
	stopBody := `{"session_id":"` + startResp.SessionID + `"}`
	stopReq := httptest.NewRequest(http.MethodPost, "/parking/stop", strings.NewReader(stopBody))
	stopReq.Header.Set("Content-Type", "application/json")
	stopRec := httptest.NewRecorder()
	h.HandleStopParking(stopRec, stopReq)

	if stopRec.Code != http.StatusOK {
		t.Fatalf("stop: expected 200, got %d", stopRec.Code)
	}

	// Verify it shows as completed in status.
	statusReq2 := httptest.NewRequest(http.MethodGet, "/parking/status", nil)
	statusRec2 := httptest.NewRecorder()
	h.HandleParkingStatus(statusRec2, statusReq2)

	var sessions2 []Session
	if err := json.NewDecoder(statusRec2.Body).Decode(&sessions2); err != nil {
		t.Fatalf("status2: decode error: %v", err)
	}

	found = false
	for _, s := range sessions2 {
		if s.SessionID == startResp.SessionID && s.Status == "completed" {
			found = true
			break
		}
	}
	if !found {
		t.Fatal("expected stopped session with status 'completed' in status list")
	}

	// Double-stop should return 404.
	stopReq2 := httptest.NewRequest(http.MethodPost, "/parking/stop", strings.NewReader(stopBody))
	stopReq2.Header.Set("Content-Type", "application/json")
	stopRec2 := httptest.NewRecorder()
	h.HandleStopParking(stopRec2, stopReq2)

	if stopRec2.Code != http.StatusNotFound {
		t.Fatalf("double-stop: expected 404, got %d", stopRec2.Code)
	}
}
