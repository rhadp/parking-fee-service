// Package store provides data storage implementations for the parking-fee-service.
package store

import (
	"database/sql"
	"sync"
	"time"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
)

// SessionStore provides SQLite-backed storage for parking sessions.
type SessionStore struct {
	db          *sql.DB
	mu          sync.RWMutex
	initialized bool
}

// NewSessionStore creates a new SessionStore with the given database connection.
func NewSessionStore(db *sql.DB) *SessionStore {
	return &SessionStore{
		db: db,
	}
}

// InitSchema creates the sessions table if it doesn't exist.
func (s *SessionStore) InitSchema() error {
	s.mu.Lock()
	defer s.mu.Unlock()

	query := `
		CREATE TABLE IF NOT EXISTS sessions (
			session_id TEXT PRIMARY KEY,
			vehicle_id TEXT NOT NULL,
			zone_id TEXT NOT NULL,
			latitude REAL NOT NULL,
			longitude REAL NOT NULL,
			start_time TEXT NOT NULL,
			end_time TEXT,
			hourly_rate REAL NOT NULL,
			state TEXT NOT NULL,
			total_cost REAL,
			payment_status TEXT
		);
		CREATE INDEX IF NOT EXISTS idx_sessions_vehicle_id ON sessions(vehicle_id);
		CREATE INDEX IF NOT EXISTS idx_sessions_state ON sessions(state);
	`

	_, err := s.db.Exec(query)
	if err != nil {
		return err
	}

	s.initialized = true
	return nil
}

// Save stores a new session in the database.
func (s *SessionStore) Save(session *model.Session) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	query := `
		INSERT INTO sessions (session_id, vehicle_id, zone_id, latitude, longitude, start_time, end_time, hourly_rate, state, total_cost, payment_status)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	`

	var endTime *string
	if session.EndTime != nil {
		t := session.EndTime.Format(time.RFC3339)
		endTime = &t
	}

	_, err := s.db.Exec(query,
		session.SessionID,
		session.VehicleID,
		session.ZoneID,
		session.Latitude,
		session.Longitude,
		session.StartTime.Format(time.RFC3339),
		endTime,
		session.HourlyRate,
		session.State,
		session.TotalCost,
		session.PaymentStatus,
	)
	return err
}

// Update modifies an existing session in the database.
func (s *SessionStore) Update(session *model.Session) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	query := `
		UPDATE sessions SET
			vehicle_id = ?,
			zone_id = ?,
			latitude = ?,
			longitude = ?,
			start_time = ?,
			end_time = ?,
			hourly_rate = ?,
			state = ?,
			total_cost = ?,
			payment_status = ?
		WHERE session_id = ?
	`

	var endTime *string
	if session.EndTime != nil {
		t := session.EndTime.Format(time.RFC3339)
		endTime = &t
	}

	_, err := s.db.Exec(query,
		session.VehicleID,
		session.ZoneID,
		session.Latitude,
		session.Longitude,
		session.StartTime.Format(time.RFC3339),
		endTime,
		session.HourlyRate,
		session.State,
		session.TotalCost,
		session.PaymentStatus,
		session.SessionID,
	)
	return err
}

// Get retrieves a session by ID.
// Returns nil if the session is not found.
func (s *SessionStore) Get(sessionID string) *model.Session {
	s.mu.RLock()
	defer s.mu.RUnlock()

	query := `
		SELECT session_id, vehicle_id, zone_id, latitude, longitude, start_time, end_time, hourly_rate, state, total_cost, payment_status
		FROM sessions WHERE session_id = ?
	`

	row := s.db.QueryRow(query, sessionID)
	return s.scanSession(row)
}

// GetActiveByVehicle retrieves the active session for a vehicle.
// Returns nil if no active session exists.
func (s *SessionStore) GetActiveByVehicle(vehicleID string) *model.Session {
	s.mu.RLock()
	defer s.mu.RUnlock()

	query := `
		SELECT session_id, vehicle_id, zone_id, latitude, longitude, start_time, end_time, hourly_rate, state, total_cost, payment_status
		FROM sessions WHERE vehicle_id = ? AND state = ?
	`

	row := s.db.QueryRow(query, vehicleID, model.SessionStateActive)
	return s.scanSession(row)
}

// scanSession scans a row into a Session struct.
func (s *SessionStore) scanSession(row *sql.Row) *model.Session {
	var session model.Session
	var startTimeStr string
	var endTimeStr *string

	err := row.Scan(
		&session.SessionID,
		&session.VehicleID,
		&session.ZoneID,
		&session.Latitude,
		&session.Longitude,
		&startTimeStr,
		&endTimeStr,
		&session.HourlyRate,
		&session.State,
		&session.TotalCost,
		&session.PaymentStatus,
	)
	if err != nil {
		return nil
	}

	session.StartTime, _ = time.Parse(time.RFC3339, startTimeStr)
	if endTimeStr != nil {
		t, _ := time.Parse(time.RFC3339, *endTimeStr)
		session.EndTime = &t
	}

	return &session
}

// IsInitialized returns true if the database schema has been initialized.
func (s *SessionStore) IsInitialized() bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.initialized
}

// Ping verifies the database connection is operational.
func (s *SessionStore) Ping() error {
	return s.db.Ping()
}
