// Package server implements the mock parking-operator HTTP server.
// It provides in-memory session management with start/stop/status endpoints.
package server

import (
	"crypto/rand"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"sync"
)

const (
	rateAmount   = 2.50
	rateCurrency = "EUR"
)

// session holds the state for an active or stopped parking session.
type session struct {
	SessionID  string  `json:"session_id"`
	VehicleID  string  `json:"vehicle_id"`
	ZoneID     string  `json:"zone_id"`
	StartTS    int64   `json:"start_timestamp"`
	StopTS     int64   `json:"stop_timestamp,omitempty"`
	Status     string  `json:"status"`
	TotalAmt   float64 `json:"total_amount,omitempty"`
	DurationSec int64  `json:"duration_seconds,omitempty"`
}

// Server is the mock parking-operator HTTP server.
type Server struct {
	mu       sync.Mutex
	sessions map[string]*session
	mux      *http.ServeMux
}

// New creates a new parking-operator server with all routes registered.
func New() *Server {
	s := &Server{
		sessions: make(map[string]*session),
		mux:      http.NewServeMux(),
	}
	s.mux.HandleFunc("POST /parking/start", s.handleStart)
	s.mux.HandleFunc("POST /parking/stop", s.handleStop)
	s.mux.HandleFunc("GET /parking/status/", s.handleStatus)
	return s
}

// ServeHTTP implements the http.Handler interface.
func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	s.mux.ServeHTTP(w, r)
}

func newUUID() string {
	var b [16]byte
	_, _ = rand.Read(b[:])
	b[6] = (b[6] & 0x0f) | 0x40 // version 4
	b[8] = (b[8] & 0x3f) | 0x80 // variant RFC 4122
	return fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
		b[0:4], b[4:6], b[6:8], b[8:10], b[10:16])
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// handleStart handles POST /parking/start.
func (s *Server) handleStart(w http.ResponseWriter, r *http.Request) {
	var req struct {
		VehicleID string `json:"vehicle_id"`
		ZoneID    string `json:"zone_id"`
		Timestamp int64  `json:"timestamp"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "bad request: "+err.Error(), http.StatusBadRequest)
		return
	}
	if req.VehicleID == "" || req.ZoneID == "" || req.Timestamp == 0 {
		http.Error(w, "bad request: missing required fields", http.StatusBadRequest)
		return
	}

	id := newUUID()
	sess := &session{
		SessionID: id,
		VehicleID: req.VehicleID,
		ZoneID:    req.ZoneID,
		StartTS:   req.Timestamp,
		Status:    "active",
	}
	s.mu.Lock()
	s.sessions[id] = sess
	s.mu.Unlock()

	writeJSON(w, http.StatusOK, map[string]any{
		"session_id": id,
		"status":     "active",
		"rate": map[string]any{
			"rate_type": "per_hour",
			"amount":    rateAmount,
			"currency":  rateCurrency,
		},
	})
}

// handleStop handles POST /parking/stop.
func (s *Server) handleStop(w http.ResponseWriter, r *http.Request) {
	var req struct {
		SessionID string `json:"session_id"`
		Timestamp int64  `json:"timestamp"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "bad request: "+err.Error(), http.StatusBadRequest)
		return
	}

	s.mu.Lock()
	sess, ok := s.sessions[req.SessionID]
	if ok {
		sess.Status = "stopped"
		sess.StopTS = req.Timestamp
	}
	s.mu.Unlock()

	if !ok {
		http.Error(w, "session not found", http.StatusNotFound)
		return
	}

	durationSec := max(req.Timestamp-sess.StartTS, 0)
	totalAmount := rateAmount * float64(durationSec) / 3600.0

	writeJSON(w, http.StatusOK, map[string]any{
		"session_id":       req.SessionID,
		"status":           "stopped",
		"duration_seconds": float64(durationSec),
		"total_amount":     totalAmount,
		"currency":         rateCurrency,
	})
}

// handleStatus handles GET /parking/status/{session_id}.
func (s *Server) handleStatus(w http.ResponseWriter, r *http.Request) {
	// Extract session_id from path: /parking/status/{id}
	sessionID := strings.TrimPrefix(r.URL.Path, "/parking/status/")
	if sessionID == "" {
		http.Error(w, "missing session_id", http.StatusBadRequest)
		return
	}

	s.mu.Lock()
	sess, ok := s.sessions[sessionID]
	s.mu.Unlock()

	if !ok {
		http.Error(w, "session not found", http.StatusNotFound)
		return
	}

	writeJSON(w, http.StatusOK, map[string]any{
		"session_id": sess.SessionID,
		"vehicle_id": sess.VehicleID,
		"zone_id":    sess.ZoneID,
		"status":     sess.Status,
	})
}
