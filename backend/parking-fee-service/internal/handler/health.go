// Package handler provides HTTP handlers for the parking-fee-service.
package handler

import (
	"log/slog"
	"net/http"
	"time"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/middleware"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

// HealthHandler handles health and readiness check requests.
type HealthHandler struct {
	sessionStore *store.SessionStore
	logger       *slog.Logger
}

// NewHealthHandler creates a new HealthHandler.
func NewHealthHandler(sessionStore *store.SessionStore, logger *slog.Logger) *HealthHandler {
	return &HealthHandler{
		sessionStore: sessionStore,
		logger:       logger,
	}
}

// HandleHealth handles GET /health
// Returns status "healthy", service name, and timestamp.
func (h *HealthHandler) HandleHealth(w http.ResponseWriter, r *http.Request) {
	response := model.HealthResponse{
		Status:    "healthy",
		Service:   "parking-fee-service",
		Timestamp: time.Now().Format(time.RFC3339),
	}

	middleware.WriteJSON(w, http.StatusOK, response)
}

// HandleReady handles GET /ready
// Verifies SQLite database connection is operational.
func (h *HealthHandler) HandleReady(w http.ResponseWriter, r *http.Request) {
	// Check database connection
	if h.sessionStore == nil {
		response := model.ReadyResponse{
			Status: "not ready",
			Reason: "session store not initialized",
		}
		middleware.WriteJSON(w, http.StatusServiceUnavailable, response)
		return
	}

	if err := h.sessionStore.Ping(); err != nil {
		response := model.ReadyResponse{
			Status: "not ready",
			Reason: "database connection failed",
		}
		middleware.WriteJSON(w, http.StatusServiceUnavailable, response)
		return
	}

	response := model.ReadyResponse{
		Status: "ready",
	}

	middleware.WriteJSON(w, http.StatusOK, response)
}
