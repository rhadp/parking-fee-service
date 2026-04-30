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
	s.mu.Lock()
	defer s.mu.Unlock()
	s.responses[resp.CommandID] = resp
	if timer, ok := s.timers[resp.CommandID]; ok {
		timer.Stop()
		delete(s.timers, resp.CommandID)
	}
}

// GetResponse retrieves a command response by command ID.
// Returns the response and true if found, or nil and false otherwise.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	resp, ok := s.responses[commandID]
	if !ok {
		return nil, false
	}
	return &resp, true
}

// StartTimeout starts a timer that stores a timeout response after the
// given duration if no real response has been received.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	s.mu.Lock()
	defer s.mu.Unlock()
	timer := time.AfterFunc(duration, func() {
		s.mu.Lock()
		defer s.mu.Unlock()
		// Only store timeout if no real response has arrived
		if _, ok := s.responses[commandID]; !ok {
			s.responses[commandID] = model.CommandResponse{
				CommandID: commandID,
				Status:    "timeout",
			}
		}
		delete(s.timers, commandID)
	})
	s.timers[commandID] = timer
}
