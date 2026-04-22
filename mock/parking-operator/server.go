package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"sync"

	"github.com/google/uuid"
)

// Rate represents the pricing information for a parking session.
type Rate struct {
	RateType string  `json:"rate_type"`
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"`
}

// Session represents an in-memory parking session.
type Session struct {
	SessionID string  `json:"session_id"`
	VehicleID string  `json:"vehicle_id"`
	ZoneID    string  `json:"zone_id"`
	Status    string  `json:"status"`
	StartTime int64   `json:"start_time"`
	StopTime  int64   `json:"stop_time,omitempty"`
	Duration  int64   `json:"duration_seconds,omitempty"`
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

// startResponse is the JSON response for POST /parking/start.
type startResponse struct {
	SessionID string `json:"session_id"`
	Status    string `json:"status"`
	Rate      Rate   `json:"rate"`
}

// stopRequest is the JSON body for POST /parking/stop.
type stopRequest struct {
	SessionID string `json:"session_id"`
	Timestamp int64  `json:"timestamp"`
}

// stopResponse is the JSON response for POST /parking/stop.
type stopResponse struct {
	SessionID string  `json:"session_id"`
	Status    string  `json:"status"`
	Duration  int64   `json:"duration_seconds"`
	TotalAmt  float64 `json:"total_amount"`
	Currency  string  `json:"currency"`
}

// errorResponse is a JSON error body.
type errorResponse struct {
	Error string `json:"error"`
}

// defaultRate is the hardcoded parking rate: 2.50 EUR per hour.
var defaultRate = Rate{
	RateType: "per_hour",
	Amount:   2.50,
	Currency: "EUR",
}

// Server is the mock parking operator HTTP server.
// It manages in-memory parking sessions.
type Server struct {
	mu       sync.Mutex
	sessions map[string]*Session
}

// NewServer creates a new parking operator server.
func NewServer() *Server {
	return &Server{
		sessions: make(map[string]*Session),
	}
}

// Handler returns the HTTP handler for the parking operator server.
// Routes:
//   - POST /parking/start  — start a new parking session
//   - POST /parking/stop   — stop an active parking session
//   - GET  /parking/status/{session_id} — query session state
func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", s.handleStart)
	mux.HandleFunc("POST /parking/stop", s.handleStop)
	mux.HandleFunc("GET /parking/status/{session_id}", s.handleStatus)
	return mux
}

// handleStart creates a new parking session.
func (s *Server) handleStart(w http.ResponseWriter, r *http.Request) {
	var req startRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, errorResponse{
			Error: fmt.Sprintf("invalid request body: %v", err),
		})
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

	writeJSON(w, http.StatusOK, startResponse{
		SessionID: sessionID,
		Status:    "active",
		Rate:      defaultRate,
	})
}

// handleStop stops an existing parking session and calculates the bill.
func (s *Server) handleStop(w http.ResponseWriter, r *http.Request) {
	var req stopRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, errorResponse{
			Error: fmt.Sprintf("invalid request body: %v", err),
		})
		return
	}

	s.mu.Lock()
	session, ok := s.sessions[req.SessionID]
	if !ok {
		s.mu.Unlock()
		writeJSON(w, http.StatusNotFound, errorResponse{
			Error: fmt.Sprintf("session not found: %s", req.SessionID),
		})
		return
	}

	duration := req.Timestamp - session.StartTime
	durationHours := float64(duration) / 3600.0
	totalAmount := session.Rate.Amount * durationHours

	session.Status = "stopped"
	session.StopTime = req.Timestamp
	session.Duration = duration
	session.TotalAmt = totalAmount
	session.Currency = "EUR"
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, stopResponse{
		SessionID: req.SessionID,
		Status:    "stopped",
		Duration:  duration,
		TotalAmt:  totalAmount,
		Currency:  "EUR",
	})
}

// handleStatus returns the current state of a parking session.
func (s *Server) handleStatus(w http.ResponseWriter, r *http.Request) {
	sessionID := r.PathValue("session_id")

	s.mu.Lock()
	session, ok := s.sessions[sessionID]
	if !ok {
		s.mu.Unlock()
		writeJSON(w, http.StatusNotFound, errorResponse{
			Error: fmt.Sprintf("session not found: %s", sessionID),
		})
		return
	}
	// Copy session data while holding the lock.
	sessionCopy := *session
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, sessionCopy)
}

// writeJSON writes a JSON response with the given status code.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v)
}
