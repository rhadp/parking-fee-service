package main

import (
	"fmt"
	"math"
	"sync"
	"time"

	"github.com/google/uuid"
)

const feePerHour = 2.50 // EUR per hour

// Session represents a parking session.
type Session struct {
	SessionID string    `json:"session_id"`
	VehicleID string    `json:"vehicle_id"`
	ZoneID    string    `json:"zone_id"`
	StartTime time.Time `json:"start_time"`
	Status    string    `json:"status"` // "active" or "completed"
}

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

// Create starts a new parking session and returns it.
func (s *SessionStore) Create(vehicleID, zoneID string) *Session {
	s.mu.Lock()
	defer s.mu.Unlock()

	session := &Session{
		SessionID: uuid.New().String(),
		VehicleID: vehicleID,
		ZoneID:    zoneID,
		StartTime: time.Now(),
		Status:    "active",
	}
	s.sessions[session.SessionID] = session
	return session
}

// Stop completes a parking session and returns the duration and fee.
// Returns an error if the session does not exist or is already completed.
func (s *SessionStore) Stop(sessionID string) (int64, float64, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	session, ok := s.sessions[sessionID]
	if !ok {
		return 0, 0, fmt.Errorf("session %q not found", sessionID)
	}
	if session.Status != "active" {
		return 0, 0, fmt.Errorf("session %q not found", sessionID)
	}

	duration := time.Since(session.StartTime)
	durationSeconds := int64(duration.Seconds())
	hours := duration.Hours()
	fee := math.Round(hours*feePerHour*100) / 100 // round to 2 decimal places

	session.Status = "completed"
	return durationSeconds, fee, nil
}

// List returns all sessions (active and completed).
func (s *SessionStore) List() []Session {
	s.mu.RLock()
	defer s.mu.RUnlock()

	result := make([]Session, 0, len(s.sessions))
	for _, session := range s.sessions {
		result = append(result, *session)
	}
	return result
}
