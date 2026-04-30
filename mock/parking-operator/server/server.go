// Package server implements the mock parking-operator HTTP server.
package server

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"sync"

	"github.com/google/uuid"
)

// Rate represents the pricing rate for a parking session.
type Rate struct {
	RateType string  `json:"rate_type"`
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"`
}

// Session represents a parking session stored in memory.
type Session struct {
	SessionID string  `json:"session_id"`
	VehicleID string  `json:"vehicle_id"`
	ZoneID    string  `json:"zone_id"`
	Status    string  `json:"status"`
	StartTime int64   `json:"start_time"`
	StopTime  int64   `json:"stop_time,omitempty"`
	Duration  uint64  `json:"duration_seconds,omitempty"`
	TotalAmt  float64 `json:"total_amount,omitempty"`
	Currency  string  `json:"currency,omitempty"`
	Rate      Rate    `json:"rate"`
}

// startRequest represents the JSON body for POST /parking/start.
type startRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// startResponse represents the JSON response for POST /parking/start.
type startResponse struct {
	SessionID string `json:"session_id"`
	Status    string `json:"status"`
	Rate      Rate   `json:"rate"`
}

// stopRequest represents the JSON body for POST /parking/stop.
type stopRequest struct {
	SessionID string `json:"session_id"`
	Timestamp int64  `json:"timestamp"`
}

// stopResponse represents the JSON response for POST /parking/stop.
type stopResponse struct {
	SessionID string  `json:"session_id"`
	Status    string  `json:"status"`
	Duration  uint64  `json:"duration_seconds"`
	TotalAmt  float64 `json:"total_amount"`
	Currency  string  `json:"currency"`
}

// errorResponse represents a JSON error response.
type errorResponse struct {
	Error string `json:"error"`
}

// defaultRate is the hardcoded rate for all parking sessions.
var defaultRate = Rate{
	RateType: "per_hour",
	Amount:   2.50,
	Currency: "EUR",
}

// store holds the in-memory session storage with mutex protection.
type store struct {
	mu       sync.Mutex
	sessions map[string]*Session
}

// newStore creates a new empty session store.
func newStore() *store {
	return &store{
		sessions: make(map[string]*Session),
	}
}

// New creates a new parking-operator HTTP handler with all routes registered.
func New() http.Handler {
	s := newStore()
	mux := http.NewServeMux()
	mux.HandleFunc("/parking/start", s.handleStart)
	mux.HandleFunc("/parking/stop", s.handleStop)
	mux.HandleFunc("/parking/status/", s.handleStatus)
	return mux
}

// handleStart handles POST /parking/start.
func (s *store) handleStart(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req startRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, fmt.Sprintf("invalid request body: %v", err))
		return
	}

	sessionID := uuid.New().String()
	session := &Session{
		SessionID: sessionID,
		VehicleID: req.VehicleID,
		ZoneID:    req.ZoneID,
		Status:    "active",
		StartTime: req.Timestamp,
		Rate:      defaultRate,
	}

	s.mu.Lock()
	s.sessions[sessionID] = session
	s.mu.Unlock()

	resp := startResponse{
		SessionID: sessionID,
		Status:    "active",
		Rate:      defaultRate,
	}

	writeJSON(w, http.StatusOK, resp)
}

// handleStop handles POST /parking/stop.
func (s *store) handleStop(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req stopRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, fmt.Sprintf("invalid request body: %v", err))
		return
	}

	s.mu.Lock()
	session, ok := s.sessions[req.SessionID]
	if !ok {
		s.mu.Unlock()
		writeError(w, http.StatusNotFound, fmt.Sprintf("session %q not found", req.SessionID))
		return
	}

	duration := uint64(req.Timestamp - session.StartTime)
	durationHours := float64(duration) / 3600.0
	totalAmount := defaultRate.Amount * durationHours

	session.Status = "stopped"
	session.StopTime = req.Timestamp
	session.Duration = duration
	session.TotalAmt = totalAmount
	session.Currency = "EUR"
	s.mu.Unlock()

	resp := stopResponse{
		SessionID: req.SessionID,
		Status:    "stopped",
		Duration:  duration,
		TotalAmt:  totalAmount,
		Currency:  "EUR",
	}

	writeJSON(w, http.StatusOK, resp)
}

// handleStatus handles GET /parking/status/{session_id}.
func (s *store) handleStatus(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	// Extract session_id from path: /parking/status/{session_id}
	path := strings.TrimPrefix(r.URL.Path, "/parking/status/")
	sessionID := path
	if sessionID == "" {
		writeError(w, http.StatusBadRequest, "missing session_id in path")
		return
	}

	s.mu.Lock()
	session, ok := s.sessions[sessionID]
	if !ok {
		s.mu.Unlock()
		writeError(w, http.StatusNotFound, fmt.Sprintf("session %q not found", sessionID))
		return
	}

	// Copy session data under lock to avoid races.
	cp := *session
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, cp)
}

// writeJSON marshals v to JSON and writes it to w with the given status code.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}

// writeError writes a JSON error response.
func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, errorResponse{Error: msg})
}
