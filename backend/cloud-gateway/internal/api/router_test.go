package api

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/bridge"
)

func TestRouter_HealthEndpoint(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	cache := NewTelemetryCache()
	router := NewRouter("demo-token", tracker, pub, cache)

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	rec := httptest.NewRecorder()

	router.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", rec.Code)
	}

	var resp map[string]interface{}
	json.NewDecoder(rec.Body).Decode(&resp)
	if resp["status"] != "ok" {
		t.Errorf("expected status 'ok', got %v", resp["status"])
	}
}

func TestRouter_HealthNoAuth(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	cache := NewTelemetryCache()
	router := NewRouter("demo-token", tracker, pub, cache)

	// Health should work without Authorization header
	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	rec := httptest.NewRecorder()

	router.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected 200 without auth, got %d", rec.Code)
	}
}

func TestRouter_CommandsRequiresAuth(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	cache := NewTelemetryCache()
	router := NewRouter("demo-token", tracker, pub, cache)

	body := `{"command_id":"test","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	rec := httptest.NewRecorder()

	router.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected 401 without auth, got %d; body: %s", rec.Code, rec.Body.String())
	}
}

func TestRouter_StatusRequiresAuth(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	cache := NewTelemetryCache()
	router := NewRouter("demo-token", tracker, pub, cache)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/status", nil)
	rec := httptest.NewRecorder()

	router.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected 401 without auth, got %d; body: %s", rec.Code, rec.Body.String())
	}
}

func TestRouter_CommandsWithAuth(t *testing.T) {
	tracker := bridge.NewTracker(200 * time.Millisecond)
	pub := &mockPublisher{}
	cache := NewTelemetryCache()
	router := NewRouter("demo-token", tracker, pub, cache)

	go func() {
		time.Sleep(50 * time.Millisecond)
		tracker.Resolve("router-test", bridge.CommandResponse{Status: "success"})
	}()

	body := `{"command_id":"router-test","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer demo-token")
	rec := httptest.NewRecorder()

	router.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected 200, got %d; body: %s", rec.Code, rec.Body.String())
	}
}

func TestRouter_StatusWithAuth(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	cache := NewTelemetryCache()
	cache.Update("VIN12345", TelemetryData{Locked: true, Timestamp: 100})
	router := NewRouter("demo-token", tracker, pub, cache)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/status", nil)
	req.Header.Set("Authorization", "Bearer demo-token")
	rec := httptest.NewRecorder()

	router.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected 200, got %d; body: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]interface{}
	json.NewDecoder(rec.Body).Decode(&resp)
	if resp["vin"] != "VIN12345" {
		t.Errorf("expected vin 'VIN12345', got %v", resp["vin"])
	}
}

func TestRouter_StatusNotFound(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	cache := NewTelemetryCache()
	router := NewRouter("demo-token", tracker, pub, cache)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/UNKNOWN/status", nil)
	req.Header.Set("Authorization", "Bearer demo-token")
	rec := httptest.NewRecorder()

	router.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d; body: %s", rec.Code, rec.Body.String())
	}
}

func TestRouter_WrongToken(t *testing.T) {
	tracker := bridge.NewTracker(time.Second)
	pub := &mockPublisher{}
	cache := NewTelemetryCache()
	router := NewRouter("demo-token", tracker, pub, cache)

	body := `{"command_id":"test","type":"lock","doors":["driver"]}`
	req := httptest.NewRequest(http.MethodPost, "/vehicles/VIN12345/commands", strings.NewReader(body))
	req.Header.Set("Authorization", "Bearer wrong-token")
	rec := httptest.NewRecorder()

	router.ServeHTTP(rec, req)

	if rec.Code != http.StatusUnauthorized {
		t.Errorf("expected 401, got %d", rec.Code)
	}
}
