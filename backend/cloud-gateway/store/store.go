package store

import (
	"sync"
	"time"

	"parking-fee-service/backend/cloud-gateway/model"
)

// Store holds command responses in memory, protected by a mutex.
type Store struct {
	mu        sync.Mutex
	responses map[string]model.CommandResponse
	timers    map[string]*time.Timer
}

// NewStore creates a new empty Store.
func NewStore() *Store {
	panic("not implemented")
}

// StoreResponse saves a command response and cancels any existing timeout timer.
func (s *Store) StoreResponse(resp model.CommandResponse) {
	panic("not implemented")
}

// GetResponse retrieves a stored command response by command ID.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	panic("not implemented")
}

// StartTimeout starts a timer that will set the command status to "timeout"
// after the specified duration if no response has been received.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	panic("not implemented")
}
