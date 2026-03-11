package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// TS-09-7: POST /parking/start with valid body creates a session.
func TestStartSession_Valid(t *testing.T) {
	store := NewSessionStore()
	handler := HandleStartParking(store)

	body := `{"vehicle_id": "VIN001", "zone_id": "muc-central", "timestamp": 1709640000}`
	req := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200, got %d", resp.StatusCode)
	}

	ct := resp.Header.Get("Content-Type")
	if ct != "application/json" {
		t.Errorf("expected Content-Type application/json, got %q", ct)
	}

	var result StartResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if result.SessionID == "" {
		t.Error("expected non-empty session_id")
	}
	if result.Status != "active" {
		t.Errorf("expected status 'active', got %q", result.Status)
	}
}

// TS-09-8: POST /parking/stop with valid session completes it.
func TestStopSession_Valid(t *testing.T) {
	store := NewSessionStore()

	// Start a session first
	startHandler := HandleStartParking(store)
	startBody := `{"vehicle_id": "VIN001", "zone_id": "muc-central", "timestamp": 1709640000}`
	startReq := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString(startBody))
	startReq.Header.Set("Content-Type", "application/json")
	startW := httptest.NewRecorder()
	startHandler.ServeHTTP(startW, startReq)

	if startW.Result().StatusCode != http.StatusOK {
		t.Fatalf("failed to start session: status %d", startW.Result().StatusCode)
	}

	var startResult StartResponse
	if err := json.NewDecoder(startW.Body).Decode(&startResult); err != nil {
		t.Fatalf("failed to decode start response: %v", err)
	}

	// Stop the session
	stopHandler := HandleStopParking(store)
	stopBody, _ := json.Marshal(StopRequest{SessionID: startResult.SessionID})
	stopReq := httptest.NewRequest(http.MethodPost, "/parking/stop", bytes.NewBuffer(stopBody))
	stopReq.Header.Set("Content-Type", "application/json")
	stopW := httptest.NewRecorder()
	stopHandler.ServeHTTP(stopW, stopReq)

	resp := stopW.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200, got %d", resp.StatusCode)
	}

	var stopResult StopResponse
	if err := json.NewDecoder(resp.Body).Decode(&stopResult); err != nil {
		t.Fatalf("failed to decode stop response: %v", err)
	}

	if stopResult.SessionID != startResult.SessionID {
		t.Errorf("session_id mismatch: got %q, want %q", stopResult.SessionID, startResult.SessionID)
	}
	if stopResult.DurationSeconds < 0 {
		t.Error("duration_seconds should be non-negative")
	}
	if stopResult.Fee < 0 {
		t.Error("fee should be non-negative")
	}
	if stopResult.Status != "completed" {
		t.Errorf("expected status 'completed', got %q", stopResult.Status)
	}
}

// TS-09-9: GET /parking/status returns all sessions.
func TestGetStatus_ReturnsAllSessions(t *testing.T) {
	store := NewSessionStore()

	// Start a session
	startHandler := HandleStartParking(store)
	startBody := `{"vehicle_id": "VIN001", "zone_id": "muc-central", "timestamp": 1709640000}`
	startReq := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString(startBody))
	startReq.Header.Set("Content-Type", "application/json")
	startW := httptest.NewRecorder()
	startHandler.ServeHTTP(startW, startReq)

	if startW.Result().StatusCode != http.StatusOK {
		t.Fatalf("failed to start session: status %d", startW.Result().StatusCode)
	}

	var startResult StartResponse
	if err := json.NewDecoder(startW.Body).Decode(&startResult); err != nil {
		t.Fatalf("failed to decode start response: %v", err)
	}

	// Get status
	statusHandler := HandleParkingStatus(store)
	statusReq := httptest.NewRequest(http.MethodGet, "/parking/status", nil)
	statusW := httptest.NewRecorder()
	statusHandler.ServeHTTP(statusW, statusReq)

	resp := statusW.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200, got %d", resp.StatusCode)
	}

	var sessions []Session
	if err := json.NewDecoder(resp.Body).Decode(&sessions); err != nil {
		t.Fatalf("failed to decode status response: %v", err)
	}

	if len(sessions) < 1 {
		t.Fatal("expected at least one session in status response")
	}

	found := false
	for _, s := range sessions {
		if s.SessionID == startResult.SessionID {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("started session %q not found in status response", startResult.SessionID)
	}
}

// TS-09-E9: GET /parking/status returns empty array when no sessions exist.
func TestGetStatus_EmptyWhenNoSessions(t *testing.T) {
	store := NewSessionStore()
	handler := HandleParkingStatus(store)

	req := httptest.NewRequest(http.MethodGet, "/parking/status", nil)
	w := httptest.NewRecorder()
	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200, got %d", resp.StatusCode)
	}

	var sessions []Session
	if err := json.NewDecoder(resp.Body).Decode(&sessions); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if len(sessions) != 0 {
		t.Errorf("expected empty array, got %d sessions", len(sessions))
	}
}

// TS-09-E7: POST /parking/start with malformed JSON returns 400.
func TestStartSession_MalformedBody(t *testing.T) {
	store := NewSessionStore()
	handler := HandleStartParking(store)

	body := `{invalid json`
	req := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusBadRequest {
		t.Fatalf("expected status 400, got %d", resp.StatusCode)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(resp.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}

	if errResp.Error == "" {
		t.Error("expected non-empty error field")
	}
}

// TS-09-E8: POST /parking/stop with unknown session_id returns 404.
func TestStopSession_UnknownSession(t *testing.T) {
	store := NewSessionStore()
	handler := HandleStopParking(store)

	body := `{"session_id": "nonexistent-session"}`
	req := httptest.NewRequest(http.MethodPost, "/parking/stop", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	handler.ServeHTTP(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusNotFound {
		t.Fatalf("expected status 404, got %d", resp.StatusCode)
	}

	var errResp ErrorResponse
	if err := json.NewDecoder(resp.Body).Decode(&errResp); err != nil {
		t.Fatalf("failed to decode error response: %v", err)
	}

	if errResp.Error == "" {
		t.Error("expected non-empty error field")
	}
}

// TS-09-P8: Session store consistency -- start, stop, and status transitions.
func TestSessionStoreConsistency(t *testing.T) {
	store := NewSessionStore()
	startHandler := HandleStartParking(store)
	stopHandler := HandleStopParking(store)
	statusHandler := HandleParkingStatus(store)

	// 1. Start a session
	startBody := `{"vehicle_id": "VIN002", "zone_id": "muc-north", "timestamp": 1709640000}`
	startReq := httptest.NewRequest(http.MethodPost, "/parking/start", bytes.NewBufferString(startBody))
	startReq.Header.Set("Content-Type", "application/json")
	startW := httptest.NewRecorder()
	startHandler.ServeHTTP(startW, startReq)

	if startW.Result().StatusCode != http.StatusOK {
		t.Fatalf("start failed: status %d", startW.Result().StatusCode)
	}

	var startResult StartResponse
	if err := json.NewDecoder(startW.Body).Decode(&startResult); err != nil {
		t.Fatalf("failed to decode start response: %v", err)
	}

	// 2. Verify session appears as "active" in status
	statusReq := httptest.NewRequest(http.MethodGet, "/parking/status", nil)
	statusW := httptest.NewRecorder()
	statusHandler.ServeHTTP(statusW, statusReq)

	var sessions []Session
	if err := json.NewDecoder(statusW.Body).Decode(&sessions); err != nil {
		t.Fatalf("failed to decode status: %v", err)
	}

	found := false
	for _, s := range sessions {
		if s.SessionID == startResult.SessionID {
			if s.Status != "active" {
				t.Errorf("expected session status 'active', got %q", s.Status)
			}
			found = true
		}
	}
	if !found {
		t.Fatal("started session not found in status")
	}

	// 3. Stop the session
	stopBody, _ := json.Marshal(StopRequest{SessionID: startResult.SessionID})
	stopReq := httptest.NewRequest(http.MethodPost, "/parking/stop", bytes.NewBuffer(stopBody))
	stopReq.Header.Set("Content-Type", "application/json")
	stopW := httptest.NewRecorder()
	stopHandler.ServeHTTP(stopW, stopReq)

	if stopW.Result().StatusCode != http.StatusOK {
		t.Fatalf("stop failed: status %d", stopW.Result().StatusCode)
	}

	var stopResult StopResponse
	if err := json.NewDecoder(stopW.Body).Decode(&stopResult); err != nil {
		t.Fatalf("failed to decode stop response: %v", err)
	}

	if stopResult.DurationSeconds < 0 {
		t.Error("duration_seconds should be non-negative")
	}
	if stopResult.Fee < 0 {
		t.Error("fee should be non-negative")
	}

	// 4. Verify session appears as "completed" in status
	statusReq2 := httptest.NewRequest(http.MethodGet, "/parking/status", nil)
	statusW2 := httptest.NewRecorder()
	statusHandler.ServeHTTP(statusW2, statusReq2)

	var sessions2 []Session
	if err := json.NewDecoder(statusW2.Body).Decode(&sessions2); err != nil {
		t.Fatalf("failed to decode status: %v", err)
	}

	for _, s := range sessions2 {
		if s.SessionID == startResult.SessionID {
			if s.Status != "completed" {
				t.Errorf("expected session status 'completed', got %q", s.Status)
			}
		}
	}

	// 5. Stopping the same session again should return 404
	stopReq2 := httptest.NewRequest(http.MethodPost, "/parking/stop", bytes.NewBuffer(stopBody))
	stopReq2.Header.Set("Content-Type", "application/json")
	stopW2 := httptest.NewRecorder()
	stopHandler.ServeHTTP(stopW2, stopReq2)

	if stopW2.Result().StatusCode != http.StatusNotFound {
		t.Errorf("expected 404 for stopping completed session, got %d", stopW2.Result().StatusCode)
	}
}
