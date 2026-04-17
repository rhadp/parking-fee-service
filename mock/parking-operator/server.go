package main

import (
	"crypto/rand"
	"encoding/json"
	"fmt"
	"net/http"
	"sync"
)

// Rate represents a parking rate schema.
type Rate struct {
	RateType string  `json:"rate_type"`
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"`
}

// Session represents an active or stopped parking session.
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

// startRequest is the body for POST /parking/start.
type startRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// stopRequest is the body for POST /parking/stop.
type stopRequest struct {
	SessionID string `json:"session_id"`
	Timestamp int64  `json:"timestamp"`
}

// Server holds in-memory parking session state.
type Server struct {
	mu       sync.Mutex
	sessions map[string]*Session
}

// NewServer creates a new Server with an empty session store.
func NewServer() *Server {
	return &Server{sessions: make(map[string]*Session)}
}

// defaultRate is the hardcoded parking rate returned for every session.
var defaultRate = Rate{
	RateType: "per_hour",
	Amount:   2.50,
	Currency: "EUR",
}

// newUUID generates a random UUID v4 string using crypto/rand.
func newUUID() string {
	var b [16]byte
	_, _ = rand.Read(b[:])
	b[6] = (b[6] & 0x0f) | 0x40 // version 4
	b[8] = (b[8] & 0x3f) | 0x80 // variant bits (10xx xxxx)
	return fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
		b[0:4], b[4:6], b[6:8], b[8:10], b[10:16])
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

func writeError(w http.ResponseWriter, status int, msg string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]string{"error": msg})
}

// handleStart handles POST /parking/start.
// Accepts {"vehicle_id", "zone_id", "timestamp"} and returns a new session
// with a UUID session_id, status "active", and the hardcoded rate.
func (s *Server) handleStart(w http.ResponseWriter, r *http.Request) {
	var req startRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body: "+err.Error())
		return
	}

	session := &Session{
		SessionID: newUUID(),
		VehicleID: req.VehicleID,
		ZoneID:    req.ZoneID,
		Status:    "active",
		StartTime: req.Timestamp,
		Rate:      defaultRate,
	}

	s.mu.Lock()
	s.sessions[session.SessionID] = session
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, session)
}

// handleStop handles POST /parking/stop.
// Accepts {"session_id", "timestamp"}, stops the session, computes duration
// and total_amount, and returns the updated session. Returns 404 if the
// session_id is unknown.
func (s *Server) handleStop(w http.ResponseWriter, r *http.Request) {
	var req stopRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "invalid request body: "+err.Error())
		return
	}

	s.mu.Lock()
	session, ok := s.sessions[req.SessionID]
	if ok {
		duration := uint64(req.Timestamp - session.StartTime)
		session.Status = "stopped"
		session.StopTime = req.Timestamp
		session.Duration = duration
		session.TotalAmt = 2.50 * (float64(duration) / 3600.0)
		session.Currency = "EUR"
	}
	s.mu.Unlock()

	if !ok {
		writeError(w, http.StatusNotFound, "session not found: "+req.SessionID)
		return
	}

	writeJSON(w, http.StatusOK, session)
}

// handleStatus handles GET /parking/status/{session_id}.
// Returns the current session state or 404 if not found.
func (s *Server) handleStatus(w http.ResponseWriter, r *http.Request) {
	sessionID := r.PathValue("session_id")

	s.mu.Lock()
	session, ok := s.sessions[sessionID]
	s.mu.Unlock()

	if !ok {
		writeError(w, http.StatusNotFound, "session not found: "+sessionID)
		return
	}

	writeJSON(w, http.StatusOK, session)
}

// Handler returns an http.Handler for all parking operator endpoints.
func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", s.handleStart)
	mux.HandleFunc("POST /parking/stop", s.handleStop)
	mux.HandleFunc("GET /parking/status/{session_id}", s.handleStatus)
	return mux
}
