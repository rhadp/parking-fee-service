// Package service provides business logic for the parking-fee-service.
package service

import (
	"math"
	"sync"
	"time"

	"github.com/google/uuid"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

// ParkingService provides business logic for mock parking operations.
type ParkingService struct {
	store      *store.SessionStore
	zoneStore  *store.ZoneStore
	hourlyRate float64
	mu         sync.RWMutex
}

// NewParkingService creates a new ParkingService.
func NewParkingService(sessionStore *store.SessionStore, zoneStore *store.ZoneStore, hourlyRate float64) *ParkingService {
	return &ParkingService{
		store:      sessionStore,
		zoneStore:  zoneStore,
		hourlyRate: hourlyRate,
	}
}

// StartSession creates a new parking session or returns existing active session for vehicle_id (idempotent).
// Returns (session, isExisting, error).
func (s *ParkingService) StartSession(req *model.StartSessionRequest) (*model.Session, bool, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	// Check for existing active session (idempotent)
	existing := s.store.GetActiveByVehicle(req.VehicleID)
	if existing != nil {
		return existing, true, nil
	}

	// Parse timestamp
	startTime, err := time.Parse(time.RFC3339, req.Timestamp)
	if err != nil {
		startTime = time.Now()
	}

	// Create new session
	session := &model.Session{
		SessionID:  uuid.New().String(),
		VehicleID:  req.VehicleID,
		ZoneID:     req.ZoneID,
		Latitude:   req.Latitude,
		Longitude:  req.Longitude,
		StartTime:  startTime,
		HourlyRate: s.hourlyRate,
		State:      model.SessionStateActive,
	}

	// Save to store
	if err := s.store.Save(session); err != nil {
		return nil, false, err
	}

	return session, false, nil
}

// StopSession ends an active parking session (idempotent).
// Returns previous stop result if session is already stopped.
func (s *ParkingService) StopSession(req *model.StopSessionRequest) (*model.Session, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	// Get session
	session := s.store.Get(req.SessionID)
	if session == nil {
		return nil, nil
	}

	// If already stopped, return previous result (idempotent)
	if session.State == model.SessionStateStopped {
		return session, nil
	}

	// Parse timestamp
	endTime, err := time.Parse(time.RFC3339, req.Timestamp)
	if err != nil {
		endTime = time.Now()
	}

	// Calculate duration and cost
	durationSeconds := int64(endTime.Sub(session.StartTime).Seconds())
	totalCost := s.CalculateCost(durationSeconds)
	paymentStatus := "success" // Mock payment always succeeds

	// Update session
	session.EndTime = &endTime
	session.State = model.SessionStateStopped
	session.TotalCost = &totalCost
	session.PaymentStatus = &paymentStatus

	if err := s.store.Update(session); err != nil {
		return nil, err
	}

	return session, nil
}

// GetSessionStatus returns current session status.
// Returns nil if session not found.
func (s *ParkingService) GetSessionStatus(sessionID string) *model.SessionStatus {
	s.mu.RLock()
	defer s.mu.RUnlock()

	session := s.store.Get(sessionID)
	if session == nil {
		return nil
	}

	var durationSeconds int64
	var currentCost float64

	if session.State == model.SessionStateActive {
		durationSeconds = int64(time.Since(session.StartTime).Seconds())
		currentCost = s.CalculateCost(durationSeconds)
	} else {
		if session.EndTime != nil {
			durationSeconds = int64(session.EndTime.Sub(session.StartTime).Seconds())
		}
		if session.TotalCost != nil {
			currentCost = *session.TotalCost
		}
	}

	return &model.SessionStatus{
		SessionID:       session.SessionID,
		State:           session.State,
		StartTime:       session.StartTime.Format(time.RFC3339),
		DurationSeconds: durationSeconds,
		CurrentCost:     currentCost,
		ZoneID:          session.ZoneID,
	}
}

// GetActiveSessionByVehicle returns active session for vehicle_id if exists.
func (s *ParkingService) GetActiveSessionByVehicle(vehicleID string) *model.Session {
	s.mu.RLock()
	defer s.mu.RUnlock()

	return s.store.GetActiveByVehicle(vehicleID)
}

// CalculateCost calculates cost based on duration and hourly rate.
// Formula: (duration_seconds / 3600) * hourly_rate, rounded to 2 decimals.
func (s *ParkingService) CalculateCost(durationSeconds int64) float64 {
	cost := (float64(durationSeconds) / 3600.0) * s.hourlyRate
	return math.Round(cost*100) / 100
}
