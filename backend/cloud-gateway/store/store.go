// Package store provides in-memory command response storage with timeout management.
package store

import (
	"sync"
	"time"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/model"
)

// Store holds command responses in memory with associated timeout timers.
// All public operations are thread-safe via an internal mutex.
type Store struct {
	mu        sync.Mutex
	responses map[string]model.CommandResponse
	timers    map[string]*time.Timer
}

// NewStore creates a new, empty Store.
// STUB: returns a Store with nil maps (operations will panic or no-op).
func NewStore() *Store {
	return &Store{}
}

// StoreResponse stores a CommandResponse keyed by its CommandID and cancels any
// pending timeout timer for that command.
// STUB: no-op.
func (s *Store) StoreResponse(resp model.CommandResponse) {
	// not implemented
}

// GetResponse returns the stored response for commandID.
// Returns (nil, false) if the commandID is not found.
// STUB: always returns (nil, false).
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	return nil, false
}

// StartTimeout starts a timer for commandID. After duration elapses, if no
// response has been stored for that commandID, it stores a response with
// status "timeout".
// STUB: no-op.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	// not implemented
}
