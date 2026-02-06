// Package unit contains unit tests for the parking-fee-service.
package unit

import (
	"database/sql"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	_ "modernc.org/sqlite"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/handler"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

// TestHealthEndpoint_ReturnsHealthy tests that health returns 200 with required fields.
func TestHealthEndpoint_ReturnsHealthy(t *testing.T) {
	healthHandler := handler.NewHealthHandler(nil, nil)

	req := httptest.NewRequest("GET", "/health", nil)
	rec := httptest.NewRecorder()
	healthHandler.HandleHealth(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected status 200, got %d", rec.Code)
	}

	var response model.HealthResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &response); err != nil {
		t.Fatalf("failed to unmarshal response: %v", err)
	}

	if response.Status != "healthy" {
		t.Errorf("expected status 'healthy', got %q", response.Status)
	}

	if response.Service != "parking-fee-service" {
		t.Errorf("expected service 'parking-fee-service', got %q", response.Service)
	}

	if response.Timestamp == "" {
		t.Error("expected timestamp to be non-empty")
	}
}

// TestReadyEndpoint_ReturnsReadyWhenDatabaseInitialized tests ready returns 200 when database initialized.
func TestReadyEndpoint_ReturnsReadyWhenDatabaseInitialized(t *testing.T) {
	db, err := sql.Open("sqlite", ":memory:")
	if err != nil {
		t.Fatalf("failed to open test database: %v", err)
	}
	defer db.Close()

	sessionStore := store.NewSessionStore(db)
	if err := sessionStore.InitSchema(); err != nil {
		t.Fatalf("failed to init schema: %v", err)
	}

	healthHandler := handler.NewHealthHandler(sessionStore, nil)

	req := httptest.NewRequest("GET", "/ready", nil)
	rec := httptest.NewRecorder()
	healthHandler.HandleReady(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("expected status 200, got %d", rec.Code)
	}

	var response model.ReadyResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &response); err != nil {
		t.Fatalf("failed to unmarshal response: %v", err)
	}

	if response.Status != "ready" {
		t.Errorf("expected status 'ready', got %q", response.Status)
	}
}

// TestReadyEndpoint_Returns503WhenDatabaseUnavailable tests ready returns 503 when database unavailable.
func TestReadyEndpoint_Returns503WhenDatabaseUnavailable(t *testing.T) {
	// Test with nil session store
	healthHandler := handler.NewHealthHandler(nil, nil)

	req := httptest.NewRequest("GET", "/ready", nil)
	rec := httptest.NewRecorder()
	healthHandler.HandleReady(rec, req)

	if rec.Code != http.StatusServiceUnavailable {
		t.Errorf("expected status 503, got %d", rec.Code)
	}

	var response model.ReadyResponse
	if err := json.Unmarshal(rec.Body.Bytes(), &response); err != nil {
		t.Fatalf("failed to unmarshal response: %v", err)
	}

	if response.Status != "not ready" {
		t.Errorf("expected status 'not ready', got %q", response.Status)
	}

	if response.Reason == "" {
		t.Error("expected reason to be non-empty")
	}
}
