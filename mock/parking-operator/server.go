package main

import (
	"encoding/json"
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
// Stub: always returns empty JSON (tests will fail checking specific fields).
func (s *Server) handleStart(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	if r.Header.Get("Content-Type") != "application/json" {
		// non-JSON content type — could be malformed request
		// stub: still returns 200 (tests checking 400 will fail)
	}
	var req StartRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		// stub: returns 200 even on parse error (tests checking 400 will fail)
		w.Write([]byte("{}")) //nolint
		return
	}
	// stub: returns empty JSON (tests checking specific fields will fail)
	w.Write([]byte("{}")) //nolint
}

// handleStop handles POST /parking/stop.
// Stub: always returns empty JSON (tests will fail checking specific fields).
func (s *Server) handleStop(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	var req StopRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.Write([]byte("{}")) //nolint
		return
	}
	// stub: returns 200 even for unknown sessions (tests checking 404 will fail)
	w.Write([]byte("{}")) //nolint
}

// handleStatus handles GET /parking/status/{session_id}.
// Stub: always returns empty JSON (tests will fail checking specific fields).
func (s *Server) handleStatus(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	// stub: returns 200 even for unknown sessions (tests checking 404 will fail)
	w.Write([]byte("{}")) //nolint
}
