package main

import (
	"fmt"
	"math"
	"sync"

	"github.com/google/uuid"
)

// SessionStore manages parking sessions in memory.
type SessionStore struct {
	mu       sync.RWMutex
	sessions map[string]*Session
}

// NewSessionStore creates a new empty SessionStore.
func NewSessionStore() *SessionStore {
	return &SessionStore{
		sessions: make(map[string]*Session),
	}
}

// Start creates a new parking session using the provided request data.
// It generates a UUID session_id, stores the session, and returns a StartResponse.
func (s *SessionStore) Start(req StartRequest) StartResponse {
	s.mu.Lock()
	defer s.mu.Unlock()

	session := &Session{
		SessionID: uuid.New().String(),
		VehicleID: req.VehicleID,
		ZoneID:    req.ZoneID,
		Status:    "active",
		StartTime: req.Timestamp,
		Rate:      DefaultRate,
	}
	s.sessions[session.SessionID] = session

	return StartResponse{
		SessionID: session.SessionID,
		Status:    session.Status,
		Rate:      session.Rate,
	}
}

// Stop completes a parking session. It calculates duration_seconds from start
// to stop timestamp, computes total_amount as rate * duration_hours, and returns
// a StopResponse. Returns an error if the session does not exist.
func (s *SessionStore) Stop(req StopRequest) (StopResponse, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	session, ok := s.sessions[req.SessionID]
	if !ok {
		return StopResponse{}, fmt.Errorf("session not found")
	}
	if session.Status != "active" {
		return StopResponse{}, fmt.Errorf("session not found")
	}

	durationSeconds := req.Timestamp - session.StartTime
	if durationSeconds < 0 {
		durationSeconds = 0
	}
	durationHours := float64(durationSeconds) / 3600.0
	totalAmount := math.Round(session.Rate.Amount*durationHours*100) / 100

	session.Status = "stopped"
	session.StopTime = req.Timestamp

	return StopResponse{
		SessionID:       session.SessionID,
		Status:          "stopped",
		DurationSeconds: durationSeconds,
		TotalAmount:     totalAmount,
		Currency:        session.Rate.Currency,
	}, nil
}

// GetStatus returns the session with the given ID, or an error if not found.
func (s *SessionStore) GetStatus(sessionID string) (*Session, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	session, ok := s.sessions[sessionID]
	if !ok {
		return nil, fmt.Errorf("session not found")
	}
	// Return a copy
	copy := *session
	return &copy, nil
}
