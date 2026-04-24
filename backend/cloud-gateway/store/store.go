package store

import (
	"sync"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Store provides thread-safe in-memory storage for command responses.
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

// StoreResponse stores a command response and cancels any pending timeout timer.
func (s *Store) StoreResponse(resp model.CommandResponse) {
}

// GetResponse retrieves a command response by command ID.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	return nil, false
}

// StartTimeout starts a timeout timer for the given command ID.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
}
