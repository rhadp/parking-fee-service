package store

import (
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Store provides thread-safe in-memory storage for command responses
// and manages command timeout timers.
type Store struct{}

// NewStore creates a new empty Store.
func NewStore() *Store {
	return &Store{} // stub
}

// StoreResponse stores a command response and cancels any pending timeout timer.
func (s *Store) StoreResponse(resp model.CommandResponse) {
	// stub
}

// GetResponse retrieves a stored command response by command ID.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	return nil, false // stub
}

// StartTimeout starts a timer that stores a timeout response if no real
// response arrives within the given duration.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	// stub
}
