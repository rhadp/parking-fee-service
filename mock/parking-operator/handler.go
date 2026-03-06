package main

import "net/http"

// Handler holds the HTTP handlers for the parking operator REST API.
type Handler struct {
	store *SessionStore
}

// NewHandler creates a Handler with the given session store.
func NewHandler(store *SessionStore) *Handler {
	return &Handler{store: store}
}

// HandleStartParking handles POST /parking/start.
// Stub: returns 501 Not Implemented.
func (h *Handler) HandleStartParking(w http.ResponseWriter, r *http.Request) {
	http.Error(w, `{"error":"not implemented"}`, http.StatusNotImplemented)
}

// HandleStopParking handles POST /parking/stop.
// Stub: returns 501 Not Implemented.
func (h *Handler) HandleStopParking(w http.ResponseWriter, r *http.Request) {
	http.Error(w, `{"error":"not implemented"}`, http.StatusNotImplemented)
}

// HandleParkingStatus handles GET /parking/status.
// Stub: returns 501 Not Implemented.
func (h *Handler) HandleParkingStatus(w http.ResponseWriter, r *http.Request) {
	http.Error(w, `{"error":"not implemented"}`, http.StatusNotImplemented)
}
