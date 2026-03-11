package main

import "time"

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
	// TODO: implement
}

// NewSessionStore creates a new empty SessionStore.
func NewSessionStore() *SessionStore {
	return &SessionStore{}
}
