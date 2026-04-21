package store

import (
	"sync"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Store holds command responses and timeout timers in memory.
type Store struct {
	mu        sync.Mutex
	responses map[string]model.CommandResponse
	timers    map[string]*time.Timer
}

// NewStore creates an empty Store.
func NewStore() *Store {
	return &Store{
		responses: make(map[string]model.CommandResponse),
		timers:    make(map[string]*time.Timer),
	}
}

// StoreResponse stores a command response and cancels any pending timeout timer.
func (s *Store) StoreResponse(resp model.CommandResponse) {
	// stub: not implemented
}

// GetResponse retrieves the response for a command ID.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	return nil, false
}

// StartTimeout schedules a timeout response after duration if no response has arrived.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	// stub: not implemented
}
