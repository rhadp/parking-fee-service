package main

import (
	"encoding/json"
	"fmt"
	"math"
	"net/http"
	"strings"
	"time"

	"github.com/google/uuid"
)

// startSessionRequest is the JSON body for POST /parking/start.
type startSessionRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// startSessionResponse is the JSON response for POST /parking/start.
type startSessionResponse struct {
	SessionID string `json:"session_id"`
	Status    string `json:"status"`
}

// stopSessionRequest is the JSON body for POST /parking/stop.
type stopSessionRequest struct {
	SessionID string `json:"session_id"`
}

// stopSessionResponse is the JSON response for POST /parking/stop.
type stopSessionResponse struct {
	SessionID       string  `json:"session_id"`
	Fee             float64 `json:"fee"`
	DurationSeconds int64   `json:"duration_seconds"`
	Currency        string  `json:"currency"`
}

// sessionStatusResponse is the JSON response for GET /parking/{session_id}/status.
type sessionStatusResponse struct {
	SessionID  string  `json:"session_id"`
	Active     bool    `json:"active"`
	StartTime  int64   `json:"start_time"`
	CurrentFee float64 `json:"current_fee"`
	Currency   string  `json:"currency"`
}

// rateResponse is the JSON response for GET /rate/{zone_id}.
type rateResponse struct {
	RatePerHour float64 `json:"rate_per_hour"`
	Currency    string  `json:"currency"`
	ZoneName    string  `json:"zone_name"`
}

// errorResponse is the JSON body for error responses.
type errorResponse struct {
	Error string `json:"error"`
}

// Handler holds the session store and provides HTTP handler methods.
type Handler struct {
	store *SessionStore
}

// NewHandler creates a new Handler with a fresh session store.
func NewHandler() *Handler {
	return &Handler{
		store: NewSessionStore(),
	}
}

// HandleStartSession handles POST /parking/start.
// Creates a new parking session and returns session_id and status.
func (h *Handler) HandleStartSession(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req startSessionRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, errorResponse{Error: "invalid request body"})
		return
	}

	session := &Session{
		ID:        uuid.New().String(),
		VehicleID: req.VehicleID,
		ZoneID:    req.ZoneID,
		StartTime: time.Now(),
		Active:    true,
	}

	h.store.AddSession(session)

	writeJSON(w, http.StatusOK, startSessionResponse{
		SessionID: session.ID,
		Status:    "active",
	})
}

// HandleStopSession handles POST /parking/stop.
// Calculates fee based on elapsed time and zone rate, marks session as stopped.
func (h *Handler) HandleStopSession(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req stopSessionRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, errorResponse{Error: "invalid request body"})
		return
	}

	session := h.store.GetSession(req.SessionID)
	if session == nil {
		writeJSON(w, http.StatusNotFound, errorResponse{
			Error: fmt.Sprintf("session %q not found", req.SessionID),
		})
		return
	}

	now := time.Now()
	session.StopTime = &now
	session.Active = false

	duration := now.Sub(session.StartTime)
	durationSeconds := int64(math.Round(duration.Seconds()))

	// Fee calculation: rate_per_hour * (duration_seconds / 3600.0)
	zone := h.store.GetZone(session.ZoneID)
	var fee float64
	var currency string
	if zone != nil {
		fee = zone.RatePerHour * (float64(durationSeconds) / 3600.0)
		currency = zone.Currency
	} else {
		currency = "EUR"
	}

	writeJSON(w, http.StatusOK, stopSessionResponse{
		SessionID:       session.ID,
		Fee:             fee,
		DurationSeconds: durationSeconds,
		Currency:        currency,
	})
}

// HandleSessionStatus handles GET /parking/{session_id}/status.
// Returns the session's current status including active state, start time, and current fee.
func (h *Handler) HandleSessionStatus(w http.ResponseWriter, r *http.Request) {
	// Extract session_id from the URL path: /parking/{session_id}/status
	// Go 1.22 pattern: the mux will route /parking/{session_id}/status to this handler
	// But we need to extract the session_id from the path.
	sessionID := extractSessionID(r.URL.Path)
	if sessionID == "" {
		writeJSON(w, http.StatusBadRequest, errorResponse{Error: "missing session_id"})
		return
	}

	session := h.store.GetSession(sessionID)
	if session == nil {
		writeJSON(w, http.StatusNotFound, errorResponse{
			Error: fmt.Sprintf("session %q not found", sessionID),
		})
		return
	}

	// Calculate current fee
	var currentFee float64
	var elapsed time.Duration
	if session.Active {
		elapsed = time.Since(session.StartTime)
	} else if session.StopTime != nil {
		elapsed = session.StopTime.Sub(session.StartTime)
	}

	zone := h.store.GetZone(session.ZoneID)
	if zone != nil {
		durationSeconds := elapsed.Seconds()
		currentFee = zone.RatePerHour * (durationSeconds / 3600.0)
	}

	var currency string
	if zone != nil {
		currency = zone.Currency
	} else {
		currency = "EUR"
	}

	writeJSON(w, http.StatusOK, sessionStatusResponse{
		SessionID:  session.ID,
		Active:     session.Active,
		StartTime:  session.StartTime.Unix(),
		CurrentFee: currentFee,
		Currency:   currency,
	})
}

// HandleRate handles GET /rate/{zone_id}.
// Returns the parking rate for the given zone.
func (h *Handler) HandleRate(w http.ResponseWriter, r *http.Request) {
	// Extract zone_id from URL path: /rate/{zone_id}
	zoneID := extractZoneID(r.URL.Path)
	if zoneID == "" {
		writeJSON(w, http.StatusBadRequest, errorResponse{Error: "missing zone_id"})
		return
	}

	zone := h.store.GetZone(zoneID)
	if zone == nil {
		writeJSON(w, http.StatusNotFound, errorResponse{
			Error: fmt.Sprintf("zone %q not found", zoneID),
		})
		return
	}

	writeJSON(w, http.StatusOK, rateResponse{
		RatePerHour: zone.RatePerHour,
		Currency:    zone.Currency,
		ZoneName:    zone.Name,
	})
}

// HandleHealth handles GET /health.
func (h *Handler) HandleHealth(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
}

// writeJSON encodes v as JSON and writes it with the given status code.
func writeJSON(w http.ResponseWriter, status int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v) //nolint:errcheck
}

// extractSessionID extracts the session_id from a path like /parking/{session_id}/status.
func extractSessionID(path string) string {
	// Expected pattern: /parking/{session_id}/status
	parts := strings.Split(strings.Trim(path, "/"), "/")
	// parts: ["parking", "{session_id}", "status"]
	if len(parts) >= 3 && parts[0] == "parking" && parts[2] == "status" {
		return parts[1]
	}
	return ""
}

// extractZoneID extracts the zone_id from a path like /rate/{zone_id}.
func extractZoneID(path string) string {
	// Expected pattern: /rate/{zone_id}
	parts := strings.Split(strings.Trim(path, "/"), "/")
	// parts: ["rate", "{zone_id}"]
	if len(parts) >= 2 && parts[0] == "rate" {
		return parts[1]
	}
	return ""
}
