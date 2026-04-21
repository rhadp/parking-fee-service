package main

import (
	"crypto/rand"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"sync"
)

// Rate represents the parking rate returned in a session start response.
type Rate struct {
	RateType string  `json:"rate_type"`
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"`
}

// Session represents an in-memory parking session.
type Session struct {
	SessionID       string  `json:"session_id"`
	VehicleID       string  `json:"vehicle_id"`
	ZoneID          string  `json:"zone_id"`
	Status          string  `json:"status"`
	StartTime       int64   `json:"start_time"`
	StopTime        int64   `json:"stop_time,omitempty"`
	DurationSeconds uint64  `json:"duration_seconds,omitempty"`
	TotalAmount     float64 `json:"total_amount,omitempty"`
	Currency        string  `json:"currency,omitempty"`
	Rate            Rate    `json:"rate"`
}

// StartRequest is the body of POST /parking/start.
type StartRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// StopRequest is the body of POST /parking/stop.
type StopRequest struct {
	SessionID string `json:"session_id"`
	Timestamp int64  `json:"timestamp"`
}

// hardcoded parking rate: 2.50 EUR per hour.
var defaultRate = Rate{
	RateType: "per_hour",
	Amount:   2.50,
	Currency: "EUR",
}

// generateUUID generates a random UUID v4.
func generateUUID() (string, error) {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		return "", err
	}
	// Set version 4 bits
	b[6] = (b[6] & 0x0f) | 0x40
	// Set variant bits
	b[8] = (b[8] & 0x3f) | 0x80
	return fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
		b[0:4], b[4:6], b[6:8], b[8:10], b[10:16]), nil
}

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(v); err != nil {
		// Best-effort; headers already sent
		fmt.Fprintf(w, `{"error":"encode failed"}`)
	}
}

// Server is the mock parking operator HTTP server.
type Server struct {
	mu       sync.Mutex
	sessions map[string]*Session
}

// NewServer creates a new Server with an empty session store.
func NewServer() *Server {
	return &Server{
		sessions: make(map[string]*Session),
	}
}

// ServeHTTP dispatches requests to the appropriate handler.
func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	switch {
	case r.Method == http.MethodPost && r.URL.Path == "/parking/start":
		s.handleStart(w, r)
	case r.Method == http.MethodPost && r.URL.Path == "/parking/stop":
		s.handleStop(w, r)
	case r.Method == http.MethodGet && strings.HasPrefix(r.URL.Path, "/parking/status/"):
		s.handleStatus(w, r)
	default:
		http.NotFound(w, r)
	}
}

// handleStart handles POST /parking/start.
// Creates a new session and returns session_id, status, and rate.
func (s *Server) handleStart(w http.ResponseWriter, r *http.Request) {
	var req StartRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "malformed request body"})
		return
	}

	sessionID, err := generateUUID()
	if err != nil {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "failed to generate session ID"})
		return
	}

	sess := &Session{
		SessionID: sessionID,
		VehicleID: req.VehicleID,
		ZoneID:    req.ZoneID,
		Status:    "active",
		StartTime: req.Timestamp,
		Rate:      defaultRate,
	}

	s.mu.Lock()
	s.sessions[sessionID] = sess
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, sess)
}

// handleStop handles POST /parking/stop.
// Stops the session, calculates duration and total amount, and returns the updated session.
func (s *Server) handleStop(w http.ResponseWriter, r *http.Request) {
	var req StopRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "malformed request body"})
		return
	}

	s.mu.Lock()
	sess, ok := s.sessions[req.SessionID]
	if !ok {
		s.mu.Unlock()
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "session not found"})
		return
	}

	// Calculate duration and total amount
	var duration int64
	if req.Timestamp > sess.StartTime {
		duration = req.Timestamp - sess.StartTime
	}
	totalAmount := sess.Rate.Amount * (float64(duration) / 3600.0)

	sess.Status = "stopped"
	sess.StopTime = req.Timestamp
	sess.DurationSeconds = uint64(duration)
	sess.TotalAmount = totalAmount
	sess.Currency = sess.Rate.Currency
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, sess)
}

// handleStatus handles GET /parking/status/{session_id}.
// Returns the current session state.
func (s *Server) handleStatus(w http.ResponseWriter, r *http.Request) {
	// Extract session_id from path: /parking/status/{session_id}
	sessionID := strings.TrimPrefix(r.URL.Path, "/parking/status/")
	if sessionID == "" {
		writeJSON(w, http.StatusBadRequest, map[string]string{"error": "missing session_id"})
		return
	}

	s.mu.Lock()
	sess, ok := s.sessions[sessionID]
	s.mu.Unlock()

	if !ok {
		writeJSON(w, http.StatusNotFound, map[string]string{"error": "session not found"})
		return
	}

	writeJSON(w, http.StatusOK, sess)
}
