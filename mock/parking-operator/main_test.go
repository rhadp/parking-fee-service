package main

import (
	"bytes"
	"encoding/json"
	"math"
	"net/http"
	"net/http/httptest"
	"testing"
)

// ─── Fee Calculation Tests (Property 7: Fee Calculation Accuracy) ───────────

func TestCalculateFeePerMinute(t *testing.T) {
	tests := []struct {
		name            string
		rateAmount      float64
		durationSeconds int64
		expectedFee     float64
	}{
		{"zero duration", 0.05, 0, 0.0},
		{"exactly 1 minute", 0.05, 60, 0.05},
		{"30 seconds rounds up", 0.05, 30, 0.05},
		{"90 seconds is 2 minutes", 0.05, 90, 0.10},
		{"5 minutes exactly", 0.05, 300, 0.25},
		{"5 minutes 1 second is 6 minutes", 0.05, 301, 0.30},
		{"1 second rounds up to 1 minute", 0.05, 1, 0.05},
		{"high rate 10 minutes", 1.00, 600, 10.00},
		{"fractional seconds 61s → 2min", 0.05, 61, 0.10},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			fee := CalculateFee("per_minute", tc.rateAmount, tc.durationSeconds)
			if math.Abs(fee-tc.expectedFee) > 0.001 {
				t.Errorf("expected fee %.4f, got %.4f", tc.expectedFee, fee)
			}
		})
	}
}

func TestCalculateFeeFlat(t *testing.T) {
	tests := []struct {
		name            string
		rateAmount      float64
		durationSeconds int64
		expectedFee     float64
	}{
		{"zero duration", 5.00, 0, 5.00},
		{"1 minute", 5.00, 60, 5.00},
		{"1 hour", 5.00, 3600, 5.00},
		{"different rate", 10.50, 7200, 10.50},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			fee := CalculateFee("flat", tc.rateAmount, tc.durationSeconds)
			if math.Abs(fee-tc.expectedFee) > 0.001 {
				t.Errorf("expected fee %.4f, got %.4f", tc.expectedFee, fee)
			}
		})
	}
}

func TestCalculateFeeUnknownType(t *testing.T) {
	fee := CalculateFee("unknown", 1.00, 300)
	if fee != 0 {
		t.Errorf("expected 0 for unknown rate type, got %.4f", fee)
	}
}

func TestCurrentFee(t *testing.T) {
	// Active session started at t=1000, now at t=1300 (300 seconds = 5 minutes).
	fee := CurrentFee("per_minute", 0.05, 1000, 1300)
	expected := 0.25 // 5 minutes × 0.05
	if math.Abs(fee-expected) > 0.001 {
		t.Errorf("expected fee %.4f, got %.4f", expected, fee)
	}
}

func TestCurrentFeeNowBeforeStart(t *testing.T) {
	fee := CurrentFee("per_minute", 0.05, 1000, 999)
	if fee != 0 {
		t.Errorf("expected 0 when now <= start, got %.4f", fee)
	}
}

// ─── Helper ─────────────────────────────────────────────────────────────────

func defaultServer() *Server {
	return NewServer(RateConfig{
		ZoneID:     "zone-1",
		RateType:   "per_minute",
		RateAmount: 0.05,
		Currency:   "EUR",
	})
}

func flatRateServer() *Server {
	return NewServer(RateConfig{
		ZoneID:     "zone-2",
		RateType:   "flat",
		RateAmount: 5.00,
		Currency:   "USD",
	})
}

func postJSON(handler http.Handler, path string, body any) *httptest.ResponseRecorder {
	data, _ := json.Marshal(body)
	req := httptest.NewRequest(http.MethodPost, path, bytes.NewReader(data))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	handler.ServeHTTP(w, req)
	return w
}

func getJSON(handler http.Handler, path string) *httptest.ResponseRecorder {
	req := httptest.NewRequest(http.MethodGet, path, nil)
	w := httptest.NewRecorder()
	handler.ServeHTTP(w, req)
	return w
}

func decodeJSON[T any](t *testing.T, w *httptest.ResponseRecorder) T {
	t.Helper()
	var v T
	if err := json.NewDecoder(w.Body).Decode(&v); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	return v
}

// ─── POST /parking/start Tests ──────────────────────────────────────────────

func TestStartSession(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	w := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	})

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", w.Code, w.Body.String())
	}

	resp := decodeJSON[StartResponse](t, w)
	if resp.SessionID == "" {
		t.Error("expected non-empty session_id")
	}
	if resp.Status != "active" {
		t.Errorf("expected status 'active', got %q", resp.Status)
	}
	if resp.Rate.RateType != "per_minute" {
		t.Errorf("expected rate_type 'per_minute', got %q", resp.Rate.RateType)
	}
	if math.Abs(resp.Rate.RateAmount-0.05) > 0.001 {
		t.Errorf("expected rate_amount 0.05, got %.4f", resp.Rate.RateAmount)
	}
	if resp.Rate.Currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %q", resp.Rate.Currency)
	}
	if resp.Rate.ZoneID != "zone-1" {
		t.Errorf("expected zone_id 'zone-1', got %q", resp.Rate.ZoneID)
	}
}

func TestStartSessionDuplicate(t *testing.T) {
	// 04-REQ-6.E2: duplicate start for same vehicle returns existing session.
	srv := defaultServer()
	handler := srv.Handler()

	req := StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	}

	// First start.
	w1 := postJSON(handler, "/parking/start", req)
	if w1.Code != http.StatusOK {
		t.Fatalf("first start failed: %d", w1.Code)
	}
	resp1 := decodeJSON[StartResponse](t, w1)

	// Second start — same vehicle, should return existing.
	w2 := postJSON(handler, "/parking/start", req)
	if w2.Code != http.StatusOK {
		t.Fatalf("second start failed: %d", w2.Code)
	}
	resp2 := decodeJSON[StartResponse](t, w2)

	if resp1.SessionID != resp2.SessionID {
		t.Errorf("expected same session_id for duplicate start, got %q and %q",
			resp1.SessionID, resp2.SessionID)
	}
}

func TestStartSessionInvalidBody(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	req := httptest.NewRequest(http.MethodPost, "/parking/start",
		bytes.NewReader([]byte("not json")))
	w := httptest.NewRecorder()
	handler.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}
}

func TestStartSessionMissingFields(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	w := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "",
		ZoneID:    "",
	})
	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}
}

func TestStartSessionDifferentVehicles(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	w1 := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	})
	resp1 := decodeJSON[StartResponse](t, w1)

	w2 := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "VIN002",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	})
	resp2 := decodeJSON[StartResponse](t, w2)

	if resp1.SessionID == resp2.SessionID {
		t.Error("expected different session IDs for different vehicles")
	}
}

// ─── POST /parking/stop Tests ───────────────────────────────────────────────

func TestStopSession(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	// Start a session.
	ws := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	})
	startResp := decodeJSON[StartResponse](t, ws)

	// Stop it 300 seconds later (5 minutes).
	w := postJSON(handler, "/parking/stop", StopRequest{
		SessionID: startResp.SessionID,
		Timestamp: 1708301100,
	})

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", w.Code, w.Body.String())
	}

	resp := decodeJSON[StopResponse](t, w)
	if resp.SessionID != startResp.SessionID {
		t.Errorf("expected session_id %q, got %q", startResp.SessionID, resp.SessionID)
	}
	if resp.Status != "completed" {
		t.Errorf("expected status 'completed', got %q", resp.Status)
	}
	if resp.DurationSeconds != 300 {
		t.Errorf("expected duration 300, got %d", resp.DurationSeconds)
	}
	// 5 minutes × 0.05 = 0.25
	expectedFee := 0.25
	if math.Abs(resp.TotalFee-expectedFee) > 0.001 {
		t.Errorf("expected fee %.4f, got %.4f", expectedFee, resp.TotalFee)
	}
	if resp.Currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %q", resp.Currency)
	}
}

func TestStopSessionFlatRate(t *testing.T) {
	srv := flatRateServer()
	handler := srv.Handler()

	ws := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-2",
		Timestamp: 1708300800,
	})
	startResp := decodeJSON[StartResponse](t, ws)

	w := postJSON(handler, "/parking/stop", StopRequest{
		SessionID: startResp.SessionID,
		Timestamp: 1708304400, // 1 hour later
	})

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	resp := decodeJSON[StopResponse](t, w)
	// Flat rate: always 5.00 regardless of duration.
	if math.Abs(resp.TotalFee-5.00) > 0.001 {
		t.Errorf("expected flat fee 5.00, got %.4f", resp.TotalFee)
	}
	if resp.Currency != "USD" {
		t.Errorf("expected currency 'USD', got %q", resp.Currency)
	}
}

func TestStopSessionUnknown(t *testing.T) {
	// 04-REQ-6.E1: unknown session_id → 404.
	srv := defaultServer()
	handler := srv.Handler()

	w := postJSON(handler, "/parking/stop", StopRequest{
		SessionID: "nonexistent",
		Timestamp: 1708300800,
	})

	if w.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", w.Code)
	}

	resp := decodeJSON[ErrorResponse](t, w)
	if resp.Error != "session not found" {
		t.Errorf("expected 'session not found', got %q", resp.Error)
	}
}

func TestStopSessionAlreadyStopped(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	// Start session.
	ws := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	})
	startResp := decodeJSON[StartResponse](t, ws)

	// Stop it.
	postJSON(handler, "/parking/stop", StopRequest{
		SessionID: startResp.SessionID,
		Timestamp: 1708301100,
	})

	// Stop again — should return completed state, not error.
	w := postJSON(handler, "/parking/stop", StopRequest{
		SessionID: startResp.SessionID,
		Timestamp: 1708302000,
	})

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", w.Code)
	}

	resp := decodeJSON[StopResponse](t, w)
	if resp.Status != "completed" {
		t.Errorf("expected status 'completed', got %q", resp.Status)
	}
}

func TestStopSessionInvalidBody(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	req := httptest.NewRequest(http.MethodPost, "/parking/stop",
		bytes.NewReader([]byte("bad json")))
	w := httptest.NewRecorder()
	handler.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}
}

func TestStopSessionMissingSessionID(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	w := postJSON(handler, "/parking/stop", StopRequest{})
	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}
}

// ─── GET /parking/sessions/{id} Tests ───────────────────────────────────────

func TestGetSessionActive(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	// Start a session.
	ws := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	})
	startResp := decodeJSON[StartResponse](t, ws)

	// Query it.
	w := getJSON(handler, "/parking/sessions/"+startResp.SessionID)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d: %s", w.Code, w.Body.String())
	}

	resp := decodeJSON[SessionResponse](t, w)
	if resp.SessionID != startResp.SessionID {
		t.Errorf("expected session_id %q, got %q", startResp.SessionID, resp.SessionID)
	}
	if resp.VehicleID != "VIN001" {
		t.Errorf("expected vehicle_id 'VIN001', got %q", resp.VehicleID)
	}
	if resp.ZoneID != "zone-1" {
		t.Errorf("expected zone_id 'zone-1', got %q", resp.ZoneID)
	}
	if resp.Status != "active" {
		t.Errorf("expected status 'active', got %q", resp.Status)
	}
	if resp.StartTime != 1708300800 {
		t.Errorf("expected start_time 1708300800, got %d", resp.StartTime)
	}
	if resp.Rate.RateType != "per_minute" {
		t.Errorf("expected rate_type 'per_minute', got %q", resp.Rate.RateType)
	}
}

func TestGetSessionCompleted(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	// Start and stop.
	ws := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	})
	startResp := decodeJSON[StartResponse](t, ws)

	postJSON(handler, "/parking/stop", StopRequest{
		SessionID: startResp.SessionID,
		Timestamp: 1708301100,
	})

	// Query it.
	w := getJSON(handler, "/parking/sessions/"+startResp.SessionID)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	resp := decodeJSON[SessionResponse](t, w)
	if resp.Status != "completed" {
		t.Errorf("expected status 'completed', got %q", resp.Status)
	}
	if resp.EndTime == nil {
		t.Error("expected end_time to be set")
	}
	if resp.DurationSeconds != 300 {
		t.Errorf("expected duration 300, got %d", resp.DurationSeconds)
	}
	expectedFee := 0.25
	if math.Abs(resp.TotalFee-expectedFee) > 0.001 {
		t.Errorf("expected fee %.4f, got %.4f", expectedFee, resp.TotalFee)
	}
}

func TestGetSessionNotFound(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	w := getJSON(handler, "/parking/sessions/nonexistent")
	if w.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", w.Code)
	}
}

// ─── GET /parking/rate Tests ────────────────────────────────────────────────

func TestGetRate(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	w := getJSON(handler, "/parking/rate")
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	resp := decodeJSON[RateResponse](t, w)
	if resp.ZoneID != "zone-1" {
		t.Errorf("expected zone_id 'zone-1', got %q", resp.ZoneID)
	}
	if resp.RateType != "per_minute" {
		t.Errorf("expected rate_type 'per_minute', got %q", resp.RateType)
	}
	if math.Abs(resp.RateAmount-0.05) > 0.001 {
		t.Errorf("expected rate_amount 0.05, got %.4f", resp.RateAmount)
	}
	if resp.Currency != "EUR" {
		t.Errorf("expected currency 'EUR', got %q", resp.Currency)
	}
}

func TestGetRateFlatConfig(t *testing.T) {
	srv := flatRateServer()
	handler := srv.Handler()

	w := getJSON(handler, "/parking/rate")
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	resp := decodeJSON[RateResponse](t, w)
	if resp.RateType != "flat" {
		t.Errorf("expected rate_type 'flat', got %q", resp.RateType)
	}
	if math.Abs(resp.RateAmount-5.00) > 0.001 {
		t.Errorf("expected rate_amount 5.00, got %.4f", resp.RateAmount)
	}
	if resp.Currency != "USD" {
		t.Errorf("expected currency 'USD', got %q", resp.Currency)
	}
}

// ─── Full Session Lifecycle Test ────────────────────────────────────────────

func TestFullSessionLifecycle(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	// 1. Check rate.
	rw := getJSON(handler, "/parking/rate")
	if rw.Code != http.StatusOK {
		t.Fatalf("rate: expected 200, got %d", rw.Code)
	}

	// 2. Start session.
	sw := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "DEMO0000000000001",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	})
	if sw.Code != http.StatusOK {
		t.Fatalf("start: expected 200, got %d", sw.Code)
	}
	startResp := decodeJSON[StartResponse](t, sw)

	// 3. Query session — should be active.
	gw := getJSON(handler, "/parking/sessions/"+startResp.SessionID)
	if gw.Code != http.StatusOK {
		t.Fatalf("get: expected 200, got %d", gw.Code)
	}
	getResp := decodeJSON[SessionResponse](t, gw)
	if getResp.Status != "active" {
		t.Errorf("expected active, got %q", getResp.Status)
	}

	// 4. Stop session after 10 minutes (600 seconds).
	pw := postJSON(handler, "/parking/stop", StopRequest{
		SessionID: startResp.SessionID,
		Timestamp: 1708301400,
	})
	if pw.Code != http.StatusOK {
		t.Fatalf("stop: expected 200, got %d", pw.Code)
	}
	stopResp := decodeJSON[StopResponse](t, pw)
	if stopResp.Status != "completed" {
		t.Errorf("expected completed, got %q", stopResp.Status)
	}
	if stopResp.DurationSeconds != 600 {
		t.Errorf("expected 600s, got %d", stopResp.DurationSeconds)
	}
	// 10 minutes × 0.05 = 0.50
	expectedFee := 0.50
	if math.Abs(stopResp.TotalFee-expectedFee) > 0.001 {
		t.Errorf("expected fee %.4f, got %.4f", expectedFee, stopResp.TotalFee)
	}

	// 5. Query session — should be completed.
	gw2 := getJSON(handler, "/parking/sessions/"+startResp.SessionID)
	getResp2 := decodeJSON[SessionResponse](t, gw2)
	if getResp2.Status != "completed" {
		t.Errorf("expected completed, got %q", getResp2.Status)
	}

	// 6. Start new session for same vehicle (previous completed).
	sw2 := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "DEMO0000000000001",
		ZoneID:    "zone-1",
		Timestamp: 1708302000,
	})
	if sw2.Code != http.StatusOK {
		t.Fatalf("second start: expected 200, got %d", sw2.Code)
	}
	startResp2 := decodeJSON[StartResponse](t, sw2)
	if startResp2.SessionID == startResp.SessionID {
		t.Error("expected new session_id for new session after previous completed")
	}
}

// ─── Session Store Tests ────────────────────────────────────────────────────

func TestSessionStoreNextID(t *testing.T) {
	store := NewSessionStore()
	id1 := store.nextID()
	id2 := store.nextID()
	if id1 == id2 {
		t.Error("expected unique IDs")
	}
	if id1 != "sess-001" {
		t.Errorf("expected 'sess-001', got %q", id1)
	}
	if id2 != "sess-002" {
		t.Errorf("expected 'sess-002', got %q", id2)
	}
}

func TestSessionStoreFindActiveByVehicle(t *testing.T) {
	store := NewSessionStore()
	store.sessions["s1"] = &Session{SessionID: "s1", VehicleID: "V1", Status: "active"}
	store.sessions["s2"] = &Session{SessionID: "s2", VehicleID: "V2", Status: "completed"}

	found := store.FindActiveByVehicle("V1")
	if found == nil || found.SessionID != "s1" {
		t.Error("expected to find active session for V1")
	}

	found = store.FindActiveByVehicle("V2")
	if found != nil {
		t.Error("expected no active session for V2 (completed)")
	}

	found = store.FindActiveByVehicle("V3")
	if found != nil {
		t.Error("expected nil for unknown vehicle")
	}
}

// ─── Utility Tests ──────────────────────────────────────────────────────────

func TestEnvOrDefault(t *testing.T) {
	t.Setenv("TEST_PO_VAR", "custom")
	if v := envOrDefault("TEST_PO_VAR", "default"); v != "custom" {
		t.Errorf("expected 'custom', got %q", v)
	}

	if v := envOrDefault("TEST_PO_UNSET", "fallback"); v != "fallback" {
		t.Errorf("expected 'fallback', got %q", v)
	}
}

func TestParseFloatOrDefault(t *testing.T) {
	tests := []struct {
		input    string
		def      float64
		expected float64
	}{
		{"1.5", 0.0, 1.5},
		{"", 0.05, 0.05},
		{"bad", 0.05, 0.05},
		{"0", 0.05, 0.0},
	}
	for _, tc := range tests {
		result := parseFloatOrDefault(tc.input, tc.def)
		if math.Abs(result-tc.expected) > 0.001 {
			t.Errorf("parseFloatOrDefault(%q, %.2f) = %.4f, expected %.4f",
				tc.input, tc.def, result, tc.expected)
		}
	}
}

// ─── Content-Type Tests ─────────────────────────────────────────────────────

func TestResponseContentType(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	w := getJSON(handler, "/parking/rate")
	ct := w.Header().Get("Content-Type")
	if ct != "application/json" {
		t.Errorf("expected Content-Type 'application/json', got %q", ct)
	}
}

// ─── Edge: Zero-Duration Stop ───────────────────────────────────────────────

func TestStopSessionZeroDuration(t *testing.T) {
	srv := defaultServer()
	handler := srv.Handler()

	ws := postJSON(handler, "/parking/start", StartRequest{
		VehicleID: "VIN001",
		ZoneID:    "zone-1",
		Timestamp: 1708300800,
	})
	startResp := decodeJSON[StartResponse](t, ws)

	// Stop at same timestamp.
	w := postJSON(handler, "/parking/stop", StopRequest{
		SessionID: startResp.SessionID,
		Timestamp: 1708300800,
	})

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	resp := decodeJSON[StopResponse](t, w)
	if resp.DurationSeconds != 0 {
		t.Errorf("expected 0 duration, got %d", resp.DurationSeconds)
	}
	if resp.TotalFee != 0 {
		t.Errorf("expected 0 fee for 0 duration, got %.4f", resp.TotalFee)
	}
}
