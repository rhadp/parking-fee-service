package main

import (
	"encoding/json"
	"net/http"
)

// HandleStartParking handles POST /parking/start.
func HandleStartParking(store *SessionStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req StartRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid JSON: "+err.Error())
			return
		}

		if req.VehicleID == "" || req.ZoneID == "" {
			writeError(w, http.StatusBadRequest, "missing required fields: vehicle_id and zone_id")
			return
		}

		session := store.Create(req.VehicleID, req.ZoneID)

		writeJSON(w, http.StatusOK, StartResponse{
			SessionID: session.SessionID,
			Status:    session.Status,
		})
	}
}

// HandleStopParking handles POST /parking/stop.
func HandleStopParking(store *SessionStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req StopRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeError(w, http.StatusBadRequest, "invalid JSON: "+err.Error())
			return
		}

		if req.SessionID == "" {
			writeError(w, http.StatusBadRequest, "missing required field: session_id")
			return
		}

		durationSeconds, fee, err := store.Stop(req.SessionID)
		if err != nil {
			writeError(w, http.StatusNotFound, err.Error())
			return
		}

		writeJSON(w, http.StatusOK, StopResponse{
			SessionID:       req.SessionID,
			DurationSeconds: durationSeconds,
			Fee:             fee,
			Status:          "completed",
		})
	}
}

// HandleParkingStatus handles GET /parking/status.
func HandleParkingStatus(store *SessionStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		sessions := store.List()
		writeJSON(w, http.StatusOK, sessions)
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
