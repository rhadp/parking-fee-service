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
func NewStore() *Store {
	return &Store{
		responses: make(map[string]model.CommandResponse),
		timers:    make(map[string]*time.Timer),
	}
}

// StoreResponse stores a CommandResponse keyed by its CommandID and cancels any
// pending timeout timer for that command.
func (s *Store) StoreResponse(resp model.CommandResponse) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.responses[resp.CommandID] = resp
	if t, ok := s.timers[resp.CommandID]; ok {
		t.Stop()
		delete(s.timers, resp.CommandID)
	}
}

// GetResponse returns the stored response for commandID.
// Returns (nil, false) if the commandID is not found.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	resp, ok := s.responses[commandID]
	if !ok {
		return nil, false
	}
	return &resp, true
}

// StartTimeout starts a timer for commandID. After duration elapses, if no
// response has been stored for that commandID, it stores a response with
// status "timeout".
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	s.mu.Lock()
	defer s.mu.Unlock()
	t := time.AfterFunc(duration, func() {
		s.mu.Lock()
		defer s.mu.Unlock()
		// Only write the timeout response if no real response has been stored yet.
		if _, exists := s.responses[commandID]; !exists {
			s.responses[commandID] = model.CommandResponse{
				CommandID: commandID,
				Status:    "timeout",
			}
		}
		delete(s.timers, commandID)
	})
	s.timers[commandID] = t
}
