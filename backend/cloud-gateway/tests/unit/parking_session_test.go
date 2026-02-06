package unit

import (
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
	"time"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/service"
)

func TestParkingSessionService_GetParkingSession_Success(t *testing.T) {
	// Create mock server
	mockSession := model.ParkingSession{
		SessionID:       "session-123",
		ZoneName:        "Demo Zone",
		HourlyRate:      2.50,
		Currency:        "EUR",
		DurationSeconds: 3600,
		CurrentCost:     2.50,
		Timestamp:       time.Now().UTC().Format(time.RFC3339),
	}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(mockSession)
	}))
	defer server.Close()

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	svc := service.NewParkingSessionService(server.URL, logger, "TEST_VIN")

	result := svc.GetParkingSession(context.Background(), "TEST_VIN")

	if result.Error != nil {
		t.Fatalf("unexpected error: %v", result.Error)
	}
	if result.Session == nil {
		t.Fatal("expected session, got nil")
	}
	if result.Session.SessionID != "session-123" {
		t.Errorf("expected session_id=session-123, got %s", result.Session.SessionID)
	}
	if result.Session.HourlyRate != 2.50 {
		t.Errorf("expected hourly_rate=2.50, got %f", result.Session.HourlyRate)
	}
}

func TestParkingSessionService_GetParkingSession_NoActiveSession(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(map[string]string{
			"error_code": "NO_ACTIVE_SESSION",
			"message":    "No active session",
		})
	}))
	defer server.Close()

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	svc := service.NewParkingSessionService(server.URL, logger, "TEST_VIN")

	result := svc.GetParkingSession(context.Background(), "TEST_VIN")

	if result.Error == nil {
		t.Fatal("expected error for no active session")
	}
	if result.Code != model.ErrNoActiveSession {
		t.Errorf("expected error code NO_ACTIVE_SESSION, got %s", result.Code)
	}
}

func TestParkingSessionService_GetParkingSession_VINValidation(t *testing.T) {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	svc := service.NewParkingSessionService("http://localhost:8081", logger, "CONFIGURED_VIN")

	// Try with wrong VIN
	result := svc.GetParkingSession(context.Background(), "WRONG_VIN")

	if result.Error == nil {
		t.Fatal("expected error for wrong VIN")
	}
	if result.Code != model.ErrVehicleNotFound {
		t.Errorf("expected error code VEHICLE_NOT_FOUND, got %s", result.Code)
	}
}

func TestParkingSessionService_GetParkingSession_Cache(t *testing.T) {
	callCount := 0
	mockSession := model.ParkingSession{
		SessionID: "session-123",
		ZoneName:  "Demo Zone",
	}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		callCount++
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(mockSession)
	}))
	defer server.Close()

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	svc := service.NewParkingSessionService(server.URL, logger, "TEST_VIN")

	// First call
	result1 := svc.GetParkingSession(context.Background(), "TEST_VIN")
	if result1.Error != nil {
		t.Fatalf("unexpected error: %v", result1.Error)
	}

	// Second call should use cache
	result2 := svc.GetParkingSession(context.Background(), "TEST_VIN")
	if result2.Error != nil {
		t.Fatalf("unexpected error: %v", result2.Error)
	}

	// Should only have made one HTTP call
	if callCount != 1 {
		t.Errorf("expected 1 HTTP call, got %d", callCount)
	}
}

func TestParkingSessionService_ClearCache(t *testing.T) {
	callCount := 0
	mockSession := model.ParkingSession{
		SessionID: "session-123",
	}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		callCount++
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(mockSession)
	}))
	defer server.Close()

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelError}))
	svc := service.NewParkingSessionService(server.URL, logger, "TEST_VIN")

	// First call
	svc.GetParkingSession(context.Background(), "TEST_VIN")

	// Clear cache
	svc.ClearCache()

	// Second call should make new HTTP request
	svc.GetParkingSession(context.Background(), "TEST_VIN")

	if callCount != 2 {
		t.Errorf("expected 2 HTTP calls after cache clear, got %d", callCount)
	}
}
