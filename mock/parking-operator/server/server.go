// Package server implements the mock parking-operator HTTP server.
// It provides an in-memory session store and REST endpoints for
// starting, stopping, and querying parking sessions.
package server

import (
	"crypto/rand"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"strings"
	"sync"
)

// Rate represents the parking rate applied to a session.
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

// startRequest is the JSON body for POST /parking/start.
type startRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// stopRequest is the JSON body for POST /parking/stop.
type stopRequest struct {
	SessionID string `json:"session_id"`
	Timestamp int64  `json:"timestamp"`
}

// startResponse is the JSON body returned by POST /parking/start.
type startResponse struct {
	SessionID string `json:"session_id"`
	Status    string `json:"status"`
	Rate      Rate   `json:"rate"`
}

// stopResponse is the JSON body returned by POST /parking/stop.
type stopResponse struct {
	SessionID string  `json:"session_id"`
	Status    string  `json:"status"`
	Duration  uint64  `json:"duration_seconds"`
	TotalAmt  float64 `json:"total_amount"`
	Currency  string  `json:"currency"`
}

// errorResponse is the JSON body returned on error.
type errorResponse struct {
	Error string `json:"error"`
}

// defaultRate is the hardcoded parking rate.
var defaultRate = Rate{
	RateType: "per_hour",
	Amount:   2.50,
	Currency: "EUR",
}

// store holds sessions in memory with mutex protection.
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

// generateUUID produces a UUID v4 string using crypto/rand.
func generateUUID() (string, error) {
	var uuid [16]byte
	if _, err := io.ReadFull(rand.Reader, uuid[:]); err != nil {
		return "", fmt.Errorf("generating UUID: %w", err)
	}
	// Set version 4 and variant bits per RFC 4122.
	uuid[6] = (uuid[6] & 0x0f) | 0x40
	uuid[8] = (uuid[8] & 0x3f) | 0x80
	return fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
		uuid[0:4], uuid[4:6], uuid[6:8], uuid[8:10], uuid[10:16]), nil
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
		writeError(w, http.StatusBadRequest, "invalid request body: "+err.Error())
		return
	}

	sessionID, err := generateUUID()
	if err != nil {
		writeError(w, http.StatusInternalServerError, "failed to generate session ID")
		return
	}

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

	log.Printf("session started: id=%s vehicle=%s zone=%s", sessionID, req.VehicleID, req.ZoneID)

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
		writeError(w, http.StatusBadRequest, "invalid request body: "+err.Error())
		return
	}

	s.mu.Lock()
	session, ok := s.sessions[req.SessionID]
	if !ok {
		s.mu.Unlock()
		writeError(w, http.StatusNotFound, "session not found: "+req.SessionID)
		return
	}

	duration := uint64(req.Timestamp - session.StartTime)
	durationHours := float64(duration) / 3600.0
	totalAmount := session.Rate.Amount * durationHours

	session.Status = "stopped"
	session.StopTime = req.Timestamp
	session.Duration = duration
	session.TotalAmt = totalAmount
	session.Currency = session.Rate.Currency
	s.mu.Unlock()

	log.Printf("session stopped: id=%s duration=%ds total=%.2f %s",
		req.SessionID, duration, totalAmount, session.Rate.Currency)

	resp := stopResponse{
		SessionID: req.SessionID,
		Status:    "stopped",
		Duration:  duration,
		TotalAmt:  totalAmount,
		Currency:  session.Rate.Currency,
	}
	writeJSON(w, http.StatusOK, resp)
}

// handleStatus handles GET /parking/status/{session_id}.
func (s *store) handleStatus(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	// Extract session_id from the path: /parking/status/{session_id}
	sessionID := strings.TrimPrefix(r.URL.Path, "/parking/status/")
	if sessionID == "" {
		writeError(w, http.StatusBadRequest, "missing session_id in path")
		return
	}

	s.mu.Lock()
	session, ok := s.sessions[sessionID]
	if !ok {
		s.mu.Unlock()
		writeError(w, http.StatusNotFound, "session not found: "+sessionID)
		return
	}

	// Copy session data under the lock to avoid races.
	cp := *session
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, cp)
}

// writeJSON serializes v as JSON and writes it to the response.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(v); err != nil {
		log.Printf("failed to write JSON response: %v", err)
	}
}

// writeError writes a JSON error response.
func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, errorResponse{Error: msg})
}
