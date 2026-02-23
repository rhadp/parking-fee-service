package api

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestStatusHandler_CachedTelemetry(t *testing.T) {
	cache := NewTelemetryCache()
	cache.Update("VIN12345", TelemetryData{
		Locked:    true,
		Timestamp: 1708700000,
	})

	handler := NewStatusHandler(cache)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/VIN12345/status", nil)
	req.SetPathValue("vin", "VIN12345")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", rec.Code)
	}

	var resp map[string]interface{}
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("response is not valid JSON: %v", err)
	}

	if resp["vin"] != "VIN12345" {
		t.Errorf("expected vin 'VIN12345', got %v", resp["vin"])
	}
	if resp["locked"] != true {
		t.Errorf("expected locked true, got %v", resp["locked"])
	}
	if resp["timestamp"].(float64) != 1708700000 {
		t.Errorf("expected timestamp 1708700000, got %v", resp["timestamp"])
	}
}

func TestStatusHandler_NoTelemetry(t *testing.T) {
	cache := NewTelemetryCache()
	handler := NewStatusHandler(cache)

	req := httptest.NewRequest(http.MethodGet, "/vehicles/UNKNOWN_VIN/status", nil)
	req.SetPathValue("vin", "UNKNOWN_VIN")
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d; body: %s", rec.Code, rec.Body.String())
	}

	var resp map[string]interface{}
	json.NewDecoder(rec.Body).Decode(&resp)
	if _, ok := resp["error"]; !ok {
		t.Error("expected 'error' field in response")
	}
}

func TestStatusHandler_MissingVIN(t *testing.T) {
	cache := NewTelemetryCache()
	handler := NewStatusHandler(cache)

	req := httptest.NewRequest(http.MethodGet, "/vehicles//status", nil)
	// Don't set path value — simulating missing VIN
	rec := httptest.NewRecorder()

	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", rec.Code)
	}
}

func TestTelemetryCache_UpdateAndGet(t *testing.T) {
	cache := NewTelemetryCache()

	// Initially empty
	_, ok := cache.Get("VIN1")
	if ok {
		t.Error("expected no data for VIN1")
	}

	// Update
	cache.Update("VIN1", TelemetryData{Locked: true, Timestamp: 100})
	data, ok := cache.Get("VIN1")
	if !ok {
		t.Fatal("expected data for VIN1 after update")
	}
	if data.VIN != "VIN1" {
		t.Errorf("expected vin 'VIN1', got %q", data.VIN)
	}
	if !data.Locked {
		t.Error("expected locked to be true")
	}

	// Overwrite
	cache.Update("VIN1", TelemetryData{Locked: false, Timestamp: 200})
	data, _ = cache.Get("VIN1")
	if data.Locked {
		t.Error("expected locked to be false after update")
	}
	if data.Timestamp != 200 {
		t.Errorf("expected timestamp 200, got %d", data.Timestamp)
	}
}

func TestTelemetryCache_MultipleVINs(t *testing.T) {
	cache := NewTelemetryCache()
	cache.Update("VIN_A", TelemetryData{Locked: true, Timestamp: 100})
	cache.Update("VIN_B", TelemetryData{Locked: false, Timestamp: 200})

	dataA, ok := cache.Get("VIN_A")
	if !ok || !dataA.Locked {
		t.Error("VIN_A: expected locked=true")
	}

	dataB, ok := cache.Get("VIN_B")
	if !ok || dataB.Locked {
		t.Error("VIN_B: expected locked=false")
	}
}
