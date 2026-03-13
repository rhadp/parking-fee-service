package main

import (
	"encoding/json"
	"net/http"
	"strings"
)

// HandleStartParking handles POST /parking/start.
func HandleStartParking(store *SessionStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req StartRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.VehicleID == "" || req.ZoneID == "" {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		resp := store.Start(req)
		writeJSON(w, http.StatusOK, resp)
	}
}

// HandleStopParking handles POST /parking/stop.
func HandleStopParking(store *SessionStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req StopRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		if req.SessionID == "" {
			writeError(w, http.StatusBadRequest, "invalid request body")
			return
		}

		resp, err := store.Stop(req)
		if err != nil {
			writeError(w, http.StatusNotFound, "session not found")
			return
		}

		writeJSON(w, http.StatusOK, resp)
	}
}

// HandleParkingStatus handles GET /parking/status/{session_id}.
func HandleParkingStatus(store *SessionStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Extract session_id from URL path: /parking/status/{session_id}
		path := strings.TrimPrefix(r.URL.Path, "/parking/status/")
		sessionID := strings.TrimRight(path, "/")
		if sessionID == "" {
			writeError(w, http.StatusBadRequest, "missing session_id in path")
			return
		}

		session, err := store.GetStatus(sessionID)
		if err != nil {
			writeError(w, http.StatusNotFound, "session not found")
			return
		}

		writeJSON(w, http.StatusOK, session)
	}
}

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error response.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, ErrorResponse{Error: message})
}
