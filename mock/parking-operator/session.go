package main

import (
	"fmt"
	"sync"
	"time"

	"github.com/google/uuid"
)

// hourlyRate is the hardcoded parking fee rate in EUR per hour.
const hourlyRate = 2.50

// SessionStore manages parking sessions in memory.
type SessionStore struct {
	mu       sync.Mutex
	sessions map[string]*Session
}

// NewSessionStore creates a new empty session store.
func NewSessionStore() *SessionStore {
	return &SessionStore{
		sessions: make(map[string]*Session),
	}
}

// Create starts a new parking session and returns the session ID.
func (s *SessionStore) Create(vehicleID, zoneID string) *Session {
	s.mu.Lock()
	defer s.mu.Unlock()

	id := uuid.New().String()
	session := &Session{
		SessionID: id,
		VehicleID: vehicleID,
		ZoneID:    zoneID,
		StartTime: time.Now(),
		Status:    "active",
	}
	s.sessions[id] = session
	return session
}

// Stop completes an active parking session and returns the duration and fee.
// Returns an error if the session does not exist or is not active.
func (s *SessionStore) Stop(sessionID string) (*StopResponse, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	session, ok := s.sessions[sessionID]
	if !ok {
		return nil, fmt.Errorf("session not found: %s", sessionID)
	}
	if session.Status != "active" {
		return nil, fmt.Errorf("session not found: %s", sessionID)
	}

	duration := time.Since(session.StartTime)
	durationSeconds := int64(duration.Seconds())
	fee := duration.Hours() * hourlyRate

	session.Status = "completed"

	return &StopResponse{
		SessionID:       sessionID,
		DurationSeconds: durationSeconds,
		Fee:             fee,
		Status:          "completed",
	}, nil
}

// List returns all sessions (active and completed).
func (s *SessionStore) List() []Session {
	s.mu.Lock()
	defer s.mu.Unlock()

	result := make([]Session, 0, len(s.sessions))
	for _, session := range s.sessions {
		result = append(result, *session)
	}
	return result
}
