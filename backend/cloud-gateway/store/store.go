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
	return &Store{
		responses: make(map[string]model.CommandResponse),
		timers:    make(map[string]*time.Timer),
	}
}

// StoreResponse saves a command response and cancels any existing timeout timer.
func (s *Store) StoreResponse(resp model.CommandResponse) {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.responses[resp.CommandID] = resp

	// Cancel any pending timeout timer for this command
	if timer, ok := s.timers[resp.CommandID]; ok {
		timer.Stop()
		delete(s.timers, resp.CommandID)
	}
}

// GetResponse retrieves a stored command response by command ID.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()

	resp, ok := s.responses[commandID]
	if !ok {
		return nil, false
	}
	return &resp, true
}

// StartTimeout starts a timer that will set the command status to "timeout"
// after the specified duration if no response has been received.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	s.mu.Lock()
	defer s.mu.Unlock()

	timer := time.AfterFunc(duration, func() {
		s.mu.Lock()
		defer s.mu.Unlock()

		// Only set timeout if no response has been stored yet
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
