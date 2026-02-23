package handler

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/store"
)

// NewRouter creates and configures the HTTP router for the parking fee service.
// The router wires up health, operator lookup, and adapter metadata handlers
// with auth middleware on protected endpoints.
//
// Routes:
//   - GET /health                     — health check (no auth)
//   - GET /operators?lat=&lon=        — operator lookup by location (auth required)
//   - GET /operators/{id}/adapter     — adapter metadata retrieval (auth required)
func NewRouter(s *store.Store, authTokens []string, fuzzinessMeters float64) http.Handler {
	opHandler := &operatorsHandler{
		store:           s,
		fuzzinessMeters: fuzzinessMeters,
	}

	adpHandler := &adapterHandler{
		store: s,
	}

	mux := http.NewServeMux()

	// Health endpoint — no auth required.
	mux.HandleFunc("GET /health", handleHealth)

	// Protected endpoints — wrapped with auth middleware.
	mux.Handle("GET /operators/{id}/adapter", AuthMiddleware(adpHandler, authTokens))
	mux.Handle("GET /operators", AuthMiddleware(opHandler, authTokens))

	return mux
}
