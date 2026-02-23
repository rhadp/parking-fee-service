package handler

import (
	"net/http"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/store"
)

// NewRouter creates and configures the HTTP router for the parking fee service.
// The router wires up health, operator lookup, and adapter metadata handlers
// with auth middleware on protected endpoints.
// Returns a stub 501 router until implemented.
func NewRouter(s *store.Store, authTokens []string, fuzzinessMeters float64) http.Handler {
	// TODO: implement routing and handlers (task group 4)
	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "Not Implemented", http.StatusNotImplemented)
	})
	return mux
}
