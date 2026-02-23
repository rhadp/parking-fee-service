package main

import (
	"sync"
	"time"
)

// Session represents a parking session in the in-memory store.
type Session struct {
	ID        string     `json:"session_id"`
	VehicleID string     `json:"vehicle_id"`
	ZoneID    string     `json:"zone_id"`
	StartTime time.Time  `json:"-"`
	StopTime  *time.Time `json:"-"`
	Active    bool       `json:"active"`
}

// Zone represents a parking zone with its rate information.
type Zone struct {
	ID          string  `json:"zone_id"`
	Name        string  `json:"zone_name"`
	RatePerHour float64 `json:"rate_per_hour"`
	Currency    string  `json:"currency"`
}

// SessionStore provides thread-safe in-memory storage for sessions and zones.
type SessionStore struct {
	mu       sync.RWMutex
	sessions map[string]*Session
	zones    map[string]*Zone
}

// NewSessionStore creates a new SessionStore with pre-configured zones.
func NewSessionStore() *SessionStore {
	s := &SessionStore{
		sessions: make(map[string]*Session),
		zones:    make(map[string]*Zone),
	}

	// Pre-configured zones per design.md
	s.zones["zone-munich-central"] = &Zone{
		ID:          "zone-munich-central",
		Name:        "Munich Central",
		RatePerHour: 2.50,
		Currency:    "EUR",
	}
	s.zones["zone-munich-west"] = &Zone{
		ID:          "zone-munich-west",
		Name:        "Munich West",
		RatePerHour: 1.50,
		Currency:    "EUR",
	}

	return s
}

// AddSession stores a new session.
func (s *SessionStore) AddSession(session *Session) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.sessions[session.ID] = session
}

// GetSession retrieves a session by ID. Returns nil if not found.
func (s *SessionStore) GetSession(id string) *Session {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.sessions[id]
}

// GetZone retrieves a zone by ID. Returns nil if not found.
func (s *SessionStore) GetZone(id string) *Zone {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.zones[id]
}
