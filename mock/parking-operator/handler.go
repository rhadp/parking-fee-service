package main

import (
	"encoding/json"
	"net/http"
)

// Handler holds the HTTP handlers for the parking operator REST API.
type Handler struct {
	store *SessionStore
}

// NewHandler creates a Handler with the given session store.
func NewHandler(store *SessionStore) *Handler {
	return &Handler{store: store}
}

// HandleStartParking handles POST /parking/start.
func (h *Handler) HandleStartParking(w http.ResponseWriter, r *http.Request) {
	var req StartRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "invalid JSON: "+err.Error())
		return
	}

	if req.VehicleID == "" || req.ZoneID == "" {
		writeError(w, http.StatusBadRequest, "missing required fields: vehicle_id and zone_id")
		return
	}

	session := h.store.Create(req.VehicleID, req.ZoneID)

	writeJSON(w, http.StatusOK, StartResponse{
		SessionID: session.SessionID,
		Status:    session.Status,
	})
}

// HandleStopParking handles POST /parking/stop.
func (h *Handler) HandleStopParking(w http.ResponseWriter, r *http.Request) {
	var req StopRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "invalid JSON: "+err.Error())
		return
	}

	if req.SessionID == "" {
		writeError(w, http.StatusBadRequest, "missing required field: session_id")
		return
	}

	resp, err := h.store.Stop(req.SessionID)
	if err != nil {
		writeError(w, http.StatusNotFound, err.Error())
		return
	}

	writeJSON(w, http.StatusOK, resp)
}

// HandleParkingStatus handles GET /parking/status.
func (h *Handler) HandleParkingStatus(w http.ResponseWriter, _ *http.Request) {
	sessions := h.store.List()
	writeJSON(w, http.StatusOK, sessions)
}

// writeJSON encodes a value as JSON and writes it to the response.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error response.
func writeError(w http.ResponseWriter, status int, message string) {
	writeJSON(w, status, ErrorResponse{Error: message})
}
