// server.go — HTTP handlers and session store for the mock parking operator.
package main

import (
	"crypto/rand"
	"encoding/json"
	"fmt"
	"net/http"
	"sync"
)

const (
	ratePerHour  = 2.50
	rateCurrency = "EUR"
	rateType     = "per_hour"
)

// Rate describes the parking fee rate.
type Rate struct {
	RateType string  `json:"rate_type"`
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"`
}

// Session holds the in-memory state for a single parking session.
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

// server holds in-memory session state.
type server struct {
	mu       sync.Mutex
	sessions map[string]*Session
}

func newServer() *server {
	return &server{sessions: make(map[string]*Session)}
}

// newUUID generates a random UUID v4 using crypto/rand.
func newUUID() string {
	b := make([]byte, 16)
	if _, err := rand.Read(b); err != nil {
		panic(fmt.Sprintf("newUUID: crypto/rand.Read failed: %v", err))
	}
	b[6] = (b[6] & 0x0f) | 0x40 // Version 4
	b[8] = (b[8] & 0x3f) | 0x80 // Variant 10
	return fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
		b[0:4], b[4:6], b[6:8], b[8:10], b[10:16])
}

// writeJSON writes v as JSON with the given status code.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(v) //nolint:errcheck
}

// writeError writes a JSON error body.
func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, map[string]string{"error": msg})
}

// handleStart handles POST /parking/start.
func (s *server) handleStart(w http.ResponseWriter, r *http.Request) {
	var req startRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "malformed request body: "+err.Error())
		return
	}

	rate := Rate{RateType: rateType, Amount: ratePerHour, Currency: rateCurrency}
	sess := &Session{
		SessionID: newUUID(),
		VehicleID: req.VehicleID,
		ZoneID:    req.ZoneID,
		Status:    "active",
		StartTime: req.Timestamp,
		Rate:      rate,
	}

	s.mu.Lock()
	s.sessions[sess.SessionID] = sess
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, sess)
}

// handleStop handles POST /parking/stop.
func (s *server) handleStop(w http.ResponseWriter, r *http.Request) {
	var req stopRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, http.StatusBadRequest, "malformed request body: "+err.Error())
		return
	}

	s.mu.Lock()
	sess, ok := s.sessions[req.SessionID]
	if !ok {
		s.mu.Unlock()
		writeError(w, http.StatusNotFound, "session not found: "+req.SessionID)
		return
	}
	duration := req.Timestamp - sess.StartTime
	totalAmt := ratePerHour * (float64(duration) / 3600.0)
	sess.Status = "stopped"
	sess.StopTime = req.Timestamp
	sess.Duration = duration
	sess.TotalAmt = totalAmt
	sess.Currency = rateCurrency
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, sess)
}

// handleStatus handles GET /parking/status/{session_id}.
func (s *server) handleStatus(w http.ResponseWriter, r *http.Request) {
	sessionID := r.PathValue("session_id")

	s.mu.Lock()
	sess, ok := s.sessions[sessionID]
	s.mu.Unlock()

	if !ok {
		writeError(w, http.StatusNotFound, "session not found: "+sessionID)
		return
	}

	writeJSON(w, http.StatusOK, sess)
}
