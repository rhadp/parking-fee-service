package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"math"
	"math/rand"
	"net/http"
	"net/http/httptest"
	"testing"
)

// TS-09-5: POST /parking/start creates session and returns correct response.
func TestStartSession(t *testing.T) {
	store := NewSessionStore()
	handler := HandleStartParking(store)

	body := `{"vehicle_id": "VIN001", "zone_id": "zone-1", "timestamp": 1700000000}`
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
	if result.Rate.RateType != "per_hour" {
		t.Errorf("expected rate_type 'per_hour', got %q", result.Rate.RateType)
	}
	if result.Rate.Amount != 2.50 {
		t.Errorf("expected rate amount 2.50, got %f", result.Rate.Amount)
	}
	if result.Rate.Currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %q", result.Rate.Currency)
	}
}

// TS-09-6: POST /parking/stop calculates duration and total_amount.
func TestStopSession(t *testing.T) {
	store := NewSessionStore()

	// Start a session with known timestamp
	startResp := store.Start(StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1700000000,
	})

	stopHandler := HandleStopParking(store)
	stopBody := fmt.Sprintf(`{"session_id": %q, "timestamp": 1700003600}`, startResp.SessionID)
	stopReq := httptest.NewRequest(http.MethodPost, "/parking/stop", bytes.NewBufferString(stopBody))
	stopReq.Header.Set("Content-Type", "application/json")
	stopW := httptest.NewRecorder()
	stopHandler.ServeHTTP(stopW, stopReq)

	resp := stopW.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200, got %d", resp.StatusCode)
	}

	var result StopResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("failed to decode stop response: %v", err)
	}

	if result.SessionID != startResp.SessionID {
		t.Errorf("session_id mismatch: got %q, want %q", result.SessionID, startResp.SessionID)
	}
	if result.DurationSeconds != 3600 {
		t.Errorf("expected duration_seconds 3600, got %d", result.DurationSeconds)
	}
	if result.TotalAmount != 2.50 {
		t.Errorf("expected total_amount 2.50, got %f", result.TotalAmount)
	}
	if result.Status != "stopped" {
		t.Errorf("expected status 'stopped', got %q", result.Status)
	}
	if result.Currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %q", result.Currency)
	}
}

// TS-09-7: GET /parking/status/{session_id} returns session info.
func TestGetStatus(t *testing.T) {
	store := NewSessionStore()

	// Start a session
	startResp := store.Start(StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1700000000,
	})

	statusHandler := HandleParkingStatus(store)
	statusReq := httptest.NewRequest(http.MethodGet, "/parking/status/"+startResp.SessionID, nil)
	statusW := httptest.NewRecorder()
	statusHandler.ServeHTTP(statusW, statusReq)

	resp := statusW.Result()
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200, got %d", resp.StatusCode)
	}

	var session Session
	if err := json.NewDecoder(resp.Body).Decode(&session); err != nil {
		t.Fatalf("failed to decode status response: %v", err)
	}

	if session.SessionID != startResp.SessionID {
		t.Errorf("session_id mismatch: got %q, want %q", session.SessionID, startResp.SessionID)
	}
	if session.Status != "active" {
		t.Errorf("expected status 'active', got %q", session.Status)
	}
	if session.VehicleID != "VIN001" {
		t.Errorf("expected vehicle_id 'VIN001', got %q", session.VehicleID)
	}
	if session.ZoneID != "zone-1" {
		t.Errorf("expected zone_id 'zone-1', got %q", session.ZoneID)
	}
	if session.StartTime != 1700000000 {
		t.Errorf("expected start_time 1700000000, got %d", session.StartTime)
	}
}

// TS-09-E3: POST /parking/stop with unknown session_id returns 404.
func TestStopUnknownSession(t *testing.T) {
	store := NewSessionStore()
	handler := HandleStopParking(store)

	body := `{"session_id": "nonexistent", "timestamp": 1700000000}`
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

	if errResp.Error != "session not found" {
		t.Errorf("expected error 'session not found', got %q", errResp.Error)
	}
}

// TS-09-E4: GET /parking/status/{session_id} with unknown session_id returns 404.
func TestStatusUnknownSession(t *testing.T) {
	store := NewSessionStore()
	handler := HandleParkingStatus(store)

	req := httptest.NewRequest(http.MethodGet, "/parking/status/nonexistent", nil)
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

	if errResp.Error != "session not found" {
		t.Errorf("expected error 'session not found', got %q", errResp.Error)
	}
}

// TS-09-E5: Invalid JSON body returns 400.
func TestInvalidJSON(t *testing.T) {
	store := NewSessionStore()

	tests := []struct {
		name    string
		handler http.HandlerFunc
		path    string
	}{
		{"start", HandleStartParking(store), "/parking/start"},
		{"stop", HandleStopParking(store), "/parking/stop"},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest(http.MethodPost, tc.path, bytes.NewBufferString("not json"))
			req.Header.Set("Content-Type", "application/json")
			w := httptest.NewRecorder()

			tc.handler.ServeHTTP(w, req)

			resp := w.Result()
			if resp.StatusCode != http.StatusBadRequest {
				t.Fatalf("expected status 400, got %d", resp.StatusCode)
			}

			var errResp ErrorResponse
			if err := json.NewDecoder(resp.Body).Decode(&errResp); err != nil {
				t.Fatalf("failed to decode error response: %v", err)
			}

			if errResp.Error != "invalid request body" {
				t.Errorf("expected error 'invalid request body', got %q", errResp.Error)
			}
		})
	}
}

// TS-09-24: parking-operator defaults to port 8080.
func TestConfigDefault(t *testing.T) {
	t.Setenv("PORT", "")
	port := GetPort()
	if port != "8080" {
		t.Errorf("expected default port '8080', got %q", port)
	}
}

// TestConfigOverride verifies PORT env var overrides default.
func TestConfigOverride(t *testing.T) {
	t.Setenv("PORT", "9090")
	port := GetPort()
	if port != "9090" {
		t.Errorf("expected port '9090', got %q", port)
	}
}

// TS-09-P2: Property test — Session Lifecycle.
// For any start/stop sequence, duration and total_amount are correctly computed.
func TestPropertySessionLifecycle(t *testing.T) {
	rng := rand.New(rand.NewSource(42))

	for i := 0; i < 100; i++ {
		store := NewSessionStore()

		startTS := int64(rng.Intn(2000000000))
		duration := int64(rng.Intn(86400)) + 1 // 1 to 86400 seconds
		stopTS := startTS + duration

		startResp := store.Start(StartRequest{
			VehicleID: "VIN-PROP",
			ZoneID:    "zone-prop",
			Timestamp: startTS,
		})

		stopResp, err := store.Stop(StopRequest{
			SessionID: startResp.SessionID,
			Timestamp: stopTS,
		})
		if err != nil {
			t.Fatalf("iteration %d: unexpected error: %v", i, err)
		}

		if stopResp.DurationSeconds != duration {
			t.Errorf("iteration %d: expected duration %d, got %d", i, duration, stopResp.DurationSeconds)
		}

		expectedAmount := math.Round(DefaultRate.Amount*float64(duration)/3600.0*100) / 100
		if math.Abs(stopResp.TotalAmount-expectedAmount) > 0.01 {
			t.Errorf("iteration %d: expected total_amount %.2f, got %.2f", i, expectedAmount, stopResp.TotalAmount)
		}

		if stopResp.Status != "stopped" {
			t.Errorf("iteration %d: expected status 'stopped', got %q", i, stopResp.Status)
		}

		if stopResp.Currency != "EUR" {
			t.Errorf("iteration %d: expected currency 'EUR', got %q", i, stopResp.Currency)
		}
	}
}

// TestSessionStoreConsistency validates start → status → stop → status transitions.
func TestSessionStoreConsistency(t *testing.T) {
	store := NewSessionStore()

	// 1. Start a session
	startResp := store.Start(StartRequest{
		VehicleID: "VIN002",
		ZoneID:    "muc-north",
		Timestamp: 1700000000,
	})

	// 2. Verify session is "active"
	session, err := store.GetStatus(startResp.SessionID)
	if err != nil {
		t.Fatalf("failed to get status: %v", err)
	}
	if session.Status != "active" {
		t.Errorf("expected status 'active', got %q", session.Status)
	}

	// 3. Stop the session
	stopResp, err := store.Stop(StopRequest{
		SessionID: startResp.SessionID,
		Timestamp: 1700003600,
	})
	if err != nil {
		t.Fatalf("failed to stop session: %v", err)
	}
	if stopResp.DurationSeconds != 3600 {
		t.Errorf("expected duration 3600, got %d", stopResp.DurationSeconds)
	}
	if stopResp.TotalAmount != 2.50 {
		t.Errorf("expected total_amount 2.50, got %.2f", stopResp.TotalAmount)
	}

	// 4. Verify session is now "stopped"
	session2, err := store.GetStatus(startResp.SessionID)
	if err != nil {
		t.Fatalf("failed to get status after stop: %v", err)
	}
	if session2.Status != "stopped" {
		t.Errorf("expected status 'stopped', got %q", session2.Status)
	}

	// 5. Stopping again should fail
	_, err = store.Stop(StopRequest{
		SessionID: startResp.SessionID,
		Timestamp: 1700007200,
	})
	if err == nil {
		t.Error("expected error when stopping already stopped session")
	}
}

// TestStartSessionHandler_ViaHTTPHandler verifies the full HTTP handler path for start.
func TestStartSessionHandler_ViaHTTPHandler(t *testing.T) {
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
	if result.Rate.Amount != 2.50 {
		t.Errorf("expected rate amount 2.50, got %f", result.Rate.Amount)
	}
}
