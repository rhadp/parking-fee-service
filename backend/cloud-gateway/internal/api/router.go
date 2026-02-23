package api

import (
	"encoding/json"
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/internal/bridge"
)

// NewRouter creates the HTTP router for the CLOUD_GATEWAY REST API.
// It wires up all routes:
//   - GET /health (no auth required)
//   - POST /vehicles/{vin}/commands (auth required)
//   - GET /vehicles/{vin}/status (auth required)
//
// Protected routes are wrapped with the bearer token auth middleware.
func NewRouter(authToken string, tracker *bridge.Tracker, publisher CommandPublisher, cache *TelemetryCache) http.Handler {
	mux := http.NewServeMux()

	// Health endpoint — no authentication required (03-REQ-1.3)
	mux.HandleFunc("GET /health", handleHealth)

	// Protected endpoints — wrapped with auth middleware (03-REQ-1.4)
	cmdHandler := NewCommandHandler(tracker, publisher)
	statusHandler := NewStatusHandler(cache)

	protectedMux := http.NewServeMux()
	protectedMux.Handle("POST /vehicles/{vin}/commands", cmdHandler)
	protectedMux.Handle("GET /vehicles/{vin}/status", statusHandler)

	mux.Handle("/vehicles/", AuthMiddleware(authToken, protectedMux))

	return mux
}

// handleHealth returns {"status": "ok"} with HTTP 200 (03-REQ-1.3).
func handleHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
}
