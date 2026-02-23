// Package handler implements HTTP handlers for the parking fee service.
package handler

import (
	"encoding/json"
	"net/http"
)

// handleHealth responds to GET /health with {"status": "ok"}.
// This endpoint does not require authentication.
func handleHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
}
