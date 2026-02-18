// Package main implements a mock PARKING_OPERATOR REST service.
//
// This service simulates a real parking operator backend for development and
// testing of the PARKING_OPERATOR_ADAPTOR. It exposes REST endpoints for
// starting/stopping parking sessions, querying session details, and returning
// rate information. Sessions and fees are computed in-memory.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"math"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"
)

// ─── Data Models ────────────────────────────────────────────────────────────

// RateConfig holds the operator's pricing configuration.
type RateConfig struct {
	ZoneID     string  `json:"zone_id"`
	RateType   string  `json:"rate_type"`   // "per_minute" or "flat"
	RateAmount float64 `json:"rate_amount"` // cost unit
	Currency   string  `json:"currency"`
}

// Session represents a parking session in the in-memory store.
type Session struct {
	SessionID       string  `json:"session_id"`
	VehicleID       string  `json:"vehicle_id"`
	ZoneID          string  `json:"zone_id"`
	StartTime       int64   `json:"start_time"`
	EndTime         *int64  `json:"end_time,omitempty"`
	Status          string  `json:"status"` // "active" or "completed"
	TotalFee        float64 `json:"total_fee,omitempty"`
	DurationSeconds int64   `json:"duration_seconds,omitempty"`
}

// StartRequest is the JSON body for POST /parking/start.
type StartRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// StartResponse is the JSON response for POST /parking/start.
type StartResponse struct {
	SessionID string       `json:"session_id"`
	Status    string       `json:"status"`
	Rate      RateResponse `json:"rate"`
}

// StopRequest is the JSON body for POST /parking/stop.
type StopRequest struct {
	SessionID string `json:"session_id"`
	Timestamp int64  `json:"timestamp"`
}

// StopResponse is the JSON response for POST /parking/stop.
type StopResponse struct {
	SessionID       string  `json:"session_id"`
	Status          string  `json:"status"`
	TotalFee        float64 `json:"total_fee"`
	DurationSeconds int64   `json:"duration_seconds"`
	Currency        string  `json:"currency"`
}

// RateResponse is the JSON response for GET /parking/rate.
type RateResponse struct {
	ZoneID     string  `json:"zone_id"`
	RateType   string  `json:"rate_type"`
	RateAmount float64 `json:"rate_amount"`
	Currency   string  `json:"currency"`
}

// SessionResponse is the JSON response for GET /parking/sessions/{id}.
type SessionResponse struct {
	SessionID       string   `json:"session_id"`
	VehicleID       string   `json:"vehicle_id"`
	ZoneID          string   `json:"zone_id"`
	StartTime       int64    `json:"start_time"`
	EndTime         *int64   `json:"end_time,omitempty"`
	Rate            RateInfo `json:"rate"`
	TotalFee        float64  `json:"total_fee"`
	DurationSeconds int64    `json:"duration_seconds"`
	Status          string   `json:"status"`
}

// RateInfo is embedded rate information in session responses.
type RateInfo struct {
	RateType   string  `json:"rate_type"`
	RateAmount float64 `json:"rate_amount"`
	Currency   string  `json:"currency"`
}

// ErrorResponse is returned for error conditions.
type ErrorResponse struct {
	Error string `json:"error"`
}

// ─── In-Memory Store ────────────────────────────────────────────────────────

// SessionStore holds all sessions in memory, protected by a mutex.
type SessionStore struct {
	mu       sync.RWMutex
	sessions map[string]*Session // keyed by session_id
	counter  int                 // monotonic session ID counter
}

// NewSessionStore creates a new empty session store.
func NewSessionStore() *SessionStore {
	return &SessionStore{
		sessions: make(map[string]*Session),
	}
}

// nextID generates the next session ID.
func (s *SessionStore) nextID() string {
	s.counter++
	return fmt.Sprintf("sess-%03d", s.counter)
}

// FindActiveByVehicle returns the active session for a vehicle, or nil.
func (s *SessionStore) FindActiveByVehicle(vehicleID string) *Session {
	for _, sess := range s.sessions {
		if sess.VehicleID == vehicleID && sess.Status == "active" {
			return sess
		}
	}
	return nil
}

// ─── Fee Calculation ────────────────────────────────────────────────────────

// CalculateFee computes the parking fee based on rate type and duration.
//
// For "per_minute": rate_amount × ceil(duration_minutes)
// For "flat": fixed rate_amount regardless of duration.
func CalculateFee(rateType string, rateAmount float64, durationSeconds int64) float64 {
	switch rateType {
	case "per_minute":
		minutes := math.Ceil(float64(durationSeconds) / 60.0)
		return rateAmount * minutes
	case "flat":
		return rateAmount
	default:
		return 0
	}
}

// CurrentFee calculates the current fee for an active session based on
// elapsed time from start to now.
func CurrentFee(rateType string, rateAmount float64, startTime int64, now int64) float64 {
	if now <= startTime {
		return 0
	}
	durationSeconds := now - startTime
	return CalculateFee(rateType, rateAmount, durationSeconds)
}

// ─── HTTP Handlers ──────────────────────────────────────────────────────────

// Server holds the handler dependencies.
type Server struct {
	store *SessionStore
	rate  RateConfig
}

// NewServer creates a new Server with the given rate configuration.
func NewServer(rate RateConfig) *Server {
	return &Server{
		store: NewSessionStore(),
		rate:  rate,
	}
}

// Handler returns an http.Handler with all routes registered.
func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", s.handleStart)
	mux.HandleFunc("POST /parking/stop", s.handleStop)
	mux.HandleFunc("GET /parking/sessions/{id}", s.handleGetSession)
	mux.HandleFunc("GET /parking/rate", s.handleGetRate)
	return mux
}

// handleStart handles POST /parking/start.
func (s *Server) handleStart(w http.ResponseWriter, r *http.Request) {
	var req StartRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, ErrorResponse{Error: "invalid request body"})
		return
	}

	if req.VehicleID == "" || req.ZoneID == "" {
		writeJSON(w, http.StatusBadRequest, ErrorResponse{Error: "vehicle_id and zone_id are required"})
		return
	}

	s.store.mu.Lock()
	defer s.store.mu.Unlock()

	// Edge case: duplicate start for same vehicle → return existing session.
	if existing := s.store.FindActiveByVehicle(req.VehicleID); existing != nil {
		writeJSON(w, http.StatusOK, StartResponse{
			SessionID: existing.SessionID,
			Status:    existing.Status,
			Rate: RateResponse{
				ZoneID:     s.rate.ZoneID,
				RateType:   s.rate.RateType,
				RateAmount: s.rate.RateAmount,
				Currency:   s.rate.Currency,
			},
		})
		return
	}

	ts := req.Timestamp
	if ts == 0 {
		ts = time.Now().Unix()
	}

	sess := &Session{
		SessionID: s.store.nextID(),
		VehicleID: req.VehicleID,
		ZoneID:    req.ZoneID,
		StartTime: ts,
		Status:    "active",
	}
	s.store.sessions[sess.SessionID] = sess

	writeJSON(w, http.StatusOK, StartResponse{
		SessionID: sess.SessionID,
		Status:    sess.Status,
		Rate: RateResponse{
			ZoneID:     s.rate.ZoneID,
			RateType:   s.rate.RateType,
			RateAmount: s.rate.RateAmount,
			Currency:   s.rate.Currency,
		},
	})
}

// handleStop handles POST /parking/stop.
func (s *Server) handleStop(w http.ResponseWriter, r *http.Request) {
	var req StopRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeJSON(w, http.StatusBadRequest, ErrorResponse{Error: "invalid request body"})
		return
	}

	if req.SessionID == "" {
		writeJSON(w, http.StatusBadRequest, ErrorResponse{Error: "session_id is required"})
		return
	}

	s.store.mu.Lock()
	defer s.store.mu.Unlock()

	sess, ok := s.store.sessions[req.SessionID]
	if !ok {
		writeJSON(w, http.StatusNotFound, ErrorResponse{Error: "session not found"})
		return
	}

	if sess.Status == "completed" {
		// Already stopped — return the existing completed state.
		writeJSON(w, http.StatusOK, StopResponse{
			SessionID:       sess.SessionID,
			Status:          sess.Status,
			TotalFee:        sess.TotalFee,
			DurationSeconds: sess.DurationSeconds,
			Currency:        s.rate.Currency,
		})
		return
	}

	ts := req.Timestamp
	if ts == 0 {
		ts = time.Now().Unix()
	}

	durationSeconds := ts - sess.StartTime
	if durationSeconds < 0 {
		durationSeconds = 0
	}

	totalFee := CalculateFee(s.rate.RateType, s.rate.RateAmount, durationSeconds)

	sess.EndTime = &ts
	sess.Status = "completed"
	sess.TotalFee = totalFee
	sess.DurationSeconds = durationSeconds

	writeJSON(w, http.StatusOK, StopResponse{
		SessionID:       sess.SessionID,
		Status:          sess.Status,
		TotalFee:        sess.TotalFee,
		DurationSeconds: sess.DurationSeconds,
		Currency:        s.rate.Currency,
	})
}

// handleGetSession handles GET /parking/sessions/{id}.
func (s *Server) handleGetSession(w http.ResponseWriter, r *http.Request) {
	id := r.PathValue("id")
	if id == "" {
		writeJSON(w, http.StatusBadRequest, ErrorResponse{Error: "session id is required"})
		return
	}

	s.store.mu.RLock()
	defer s.store.mu.RUnlock()

	sess, ok := s.store.sessions[id]
	if !ok {
		writeJSON(w, http.StatusNotFound, ErrorResponse{Error: "session not found"})
		return
	}

	resp := SessionResponse{
		SessionID: sess.SessionID,
		VehicleID: sess.VehicleID,
		ZoneID:    sess.ZoneID,
		StartTime: sess.StartTime,
		EndTime:   sess.EndTime,
		Rate: RateInfo{
			RateType:   s.rate.RateType,
			RateAmount: s.rate.RateAmount,
			Currency:   s.rate.Currency,
		},
		Status: sess.Status,
	}

	if sess.Status == "active" {
		// Calculate current fee based on elapsed time.
		now := time.Now().Unix()
		elapsed := now - sess.StartTime
		if elapsed < 0 {
			elapsed = 0
		}
		resp.TotalFee = CalculateFee(s.rate.RateType, s.rate.RateAmount, elapsed)
		resp.DurationSeconds = elapsed
	} else {
		resp.TotalFee = sess.TotalFee
		resp.DurationSeconds = sess.DurationSeconds
	}

	writeJSON(w, http.StatusOK, resp)
}

// handleGetRate handles GET /parking/rate.
func (s *Server) handleGetRate(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, RateResponse{
		ZoneID:     s.rate.ZoneID,
		RateType:   s.rate.RateType,
		RateAmount: s.rate.RateAmount,
		Currency:   s.rate.Currency,
	})
}

// ─── Utility ────────────────────────────────────────────────────────────────

// writeJSON encodes v as JSON and writes it to w with the given status code.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(v); err != nil {
		log.Printf("failed to encode response: %v", err)
	}
}

// ─── Main ───────────────────────────────────────────────────────────────────

func main() {
	listenAddr := flag.String("listen-addr", envOrDefault("LISTEN_ADDR", ":8082"),
		"Address to listen on (host:port)")
	rateType := flag.String("rate-type", envOrDefault("RATE_TYPE", "per_minute"),
		"Rate type: per_minute or flat")
	rateAmount := flag.Float64("rate-amount", parseFloatOrDefault(envOrDefault("RATE_AMOUNT", ""), 0.05),
		"Rate amount per unit")
	currency := flag.String("currency", envOrDefault("CURRENCY", "EUR"),
		"Currency code")
	zoneID := flag.String("zone-id", envOrDefault("ZONE_ID", "zone-1"),
		"Zone identifier")
	flag.Parse()

	// Validate rate type.
	*rateType = strings.ToLower(*rateType)
	if *rateType != "per_minute" && *rateType != "flat" {
		fmt.Fprintf(os.Stderr, "error: invalid rate-type %q, must be per_minute or flat\n", *rateType)
		os.Exit(1)
	}

	rate := RateConfig{
		ZoneID:     *zoneID,
		RateType:   *rateType,
		RateAmount: *rateAmount,
		Currency:   *currency,
	}

	srv := NewServer(rate)
	handler := srv.Handler()

	log.Printf("Mock PARKING_OPERATOR starting on %s", *listenAddr)
	log.Printf("Rate config: zone=%s type=%s amount=%.4f currency=%s",
		rate.ZoneID, rate.RateType, rate.RateAmount, rate.Currency)

	if err := http.ListenAndServe(*listenAddr, handler); err != nil {
		log.Fatalf("server error: %v", err)
	}
}

// envOrDefault returns the value of an environment variable, or the default.
func envOrDefault(key, defaultVal string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return defaultVal
}

// parseFloatOrDefault parses a float64 from s, returning defaultVal on error.
func parseFloatOrDefault(s string, defaultVal float64) float64 {
	if s == "" {
		return defaultVal
	}
	var f float64
	if _, err := fmt.Sscanf(s, "%f", &f); err != nil {
		return defaultVal
	}
	return f
}
