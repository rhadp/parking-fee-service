package store

import (
	"sync"
	"time"

	"parking-fee-service/backend/cloud-gateway/model"
)

// Store holds command responses in memory with timeout management.
type Store struct {
	mu        sync.Mutex
	responses map[string]model.CommandResponse
	timers    map[string]*time.Timer
}

// NewStore creates an initialized Store.
func NewStore() *Store {
	return &Store{
		responses: make(map[string]model.CommandResponse),
		timers:    make(map[string]*time.Timer),
	}
}

// StoreResponse stores a command response, cancelling any pending timeout timer.
func (s *Store) StoreResponse(resp model.CommandResponse) {
}

// GetResponse returns the stored response for commandID, or (nil, false) if not found.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	return nil, false
}

// StartTimeout starts a timer that stores a timeout response for commandID after duration.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
}
