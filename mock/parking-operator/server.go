// server.go — stub HTTP handlers for the mock parking operator.
// Task group 3 will implement the full session lifecycle.
package main

import (
	"encoding/json"
	"net/http"
)

// server holds in-memory session state.
// Stub: sessions map is unused until task group 3.
type server struct {
	sessions map[string]any
}

func newServer() *server {
	return &server{sessions: make(map[string]any)}
}

// handleStart handles POST /parking/start.
// Stub returns empty JSON — real implementation in task group 3.
func (s *server) handleStart(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]any{}) //nolint
}

// handleStop handles POST /parking/stop.
// Stub returns empty JSON — real implementation in task group 3.
func (s *server) handleStop(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]any{}) //nolint
}

// handleStatus handles GET /parking/status/{session_id}.
// Stub returns empty JSON — real implementation in task group 3.
func (s *server) handleStatus(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]any{}) //nolint
}
