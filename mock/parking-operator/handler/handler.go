// Package handler provides HTTP handlers for the mock PARKING_OPERATOR server.
package handler

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/google/uuid"
	"github.com/rhadp/parking-fee-service/mock/parking-operator/store"
)

// writeJSON encodes v as JSON and writes it with the given status code.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error object.
func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, map[string]string{"error": msg})
}

// StartHandler handles POST /parking/start.
// Requirement: 09-REQ-2.2, 09-REQ-2.E3
func StartHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req store.StartRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		sessionID := uuid.NewString()
		resp := s.Start(sessionID, req)
		writeJSON(w, http.StatusOK, resp)
	}
}

// StopHandler handles POST /parking/stop.
// Requirement: 09-REQ-2.3, 09-REQ-2.E1, 09-REQ-2.E3
func StopHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req store.StopRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		resp, err := s.Stop(req)
		if err != nil {
			writeError(w, http.StatusNotFound, "session not found")
			return
		}

		writeJSON(w, http.StatusOK, resp)
	}
}

// ListSessionsHandler handles GET /parking/sessions.
// Returns all sessions for test observability.
func ListSessionsHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, _ *http.Request) {
		writeJSON(w, http.StatusOK, s.ListSessions())
	}
}

// StatusHandler handles GET /parking/status/{session_id}.
// Requirement: 09-REQ-2.4, 09-REQ-2.E2
func StatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Extract session_id from the URL path: /parking/status/{session_id}
		// Using Go 1.22 path value extraction when available; fall back to strings.TrimPrefix.
		sessionID := r.PathValue("session_id")
		if sessionID == "" {
			// Fallback: trim the fixed prefix
			sessionID = strings.TrimPrefix(r.URL.Path, "/parking/status/")
		}

		sess, err := s.GetStatus(sessionID)
		if err != nil {
			writeError(w, http.StatusNotFound, "session not found")
			return
		}

		writeJSON(w, http.StatusOK, sess)
	}
}
