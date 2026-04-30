// Package store provides an in-memory command response store with
// mutex-based thread safety and timeout management.
package store

import (
	"sync"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Store holds command responses in memory, protected by a mutex.
type Store struct {
	mu        sync.Mutex
	responses map[string]model.CommandResponse
	timers    map[string]*time.Timer
}

// NewStore creates a new empty Store.
func NewStore() *Store {
	return &Store{
		responses: make(map[string]model.CommandResponse),
		timers:    make(map[string]*time.Timer),
	}
}

// StoreResponse saves a command response and cancels any pending timeout timer.
func (s *Store) StoreResponse(resp model.CommandResponse) {
	// TODO: implement
}

// GetResponse retrieves a command response by command ID.
// Returns the response and true if found, or nil and false otherwise.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	// TODO: implement
	return nil, false
}

// StartTimeout starts a timer that stores a timeout response after the
// given duration if no real response has been received.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	// TODO: implement
}
