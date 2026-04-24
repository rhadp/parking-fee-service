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
	s.mu.Lock()
	defer s.mu.Unlock()

	s.responses[resp.CommandID] = resp

	// Cancel any pending timeout timer for this command.
	if timer, ok := s.timers[resp.CommandID]; ok {
		timer.Stop()
		delete(s.timers, resp.CommandID)
	}
}

// GetResponse retrieves a command response by command ID.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()

	resp, ok := s.responses[commandID]
	if !ok {
		return nil, false
	}
	return &resp, true
}

// StartTimeout starts a timeout timer for the given command ID.
// When the timer fires, it stores a timeout response only if no real response
// has been stored yet. This uses time.AfterFunc with an existence check inside
// the locked callback to avoid a race where a timeout overwrites a real response.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	s.mu.Lock()
	defer s.mu.Unlock()

	// Cancel any existing timer for this command ID (handles resubmission).
	if existing, ok := s.timers[commandID]; ok {
		existing.Stop()
	}

	timer := time.AfterFunc(duration, func() {
		s.mu.Lock()
		defer s.mu.Unlock()

		// Only store the timeout if no real response has arrived.
		if _, exists := s.responses[commandID]; !exists {
			s.responses[commandID] = model.CommandResponse{
				CommandID: commandID,
				Status:    "timeout",
			}
		}
		delete(s.timers, commandID)
	})

	s.timers[commandID] = timer
}
