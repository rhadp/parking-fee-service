package store

import (
	"sync"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Store provides thread-safe in-memory storage for command responses
// and manages command timeout timers.
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

// StoreResponse stores a command response and cancels any pending timeout timer.
func (s *Store) StoreResponse(resp model.CommandResponse) {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.responses[resp.CommandID] = resp

	// Cancel the timeout timer if one exists for this command.
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

// StartTimeout starts a timer that stores a timeout response if no real
// response arrives within the given duration.
//
// The lock is held while creating and registering the timer so that
// StoreResponse can always find and cancel a pending timer. The timer
// callback also acquires the lock and checks whether a real response
// already exists before writing, guarding against the race where the
// timer goroutine is enqueued before StoreResponse calls Stop().
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	s.mu.Lock()
	timer := time.AfterFunc(duration, func() {
		s.mu.Lock()
		defer s.mu.Unlock()

		// Only store the timeout if no real response has arrived.
		// This guard is essential: even when StoreResponse calls
		// timer.Stop(), Stop can return false if the timer has already
		// fired (goroutine enqueued). In that case the goroutine will
		// acquire the lock after StoreResponse releases it; this check
		// prevents overwriting the real response with a timeout.
		if _, exists := s.responses[commandID]; !exists {
			s.responses[commandID] = model.CommandResponse{
				CommandID: commandID,
				Status:    "timeout",
			}
		}
		delete(s.timers, commandID)
	})
	s.timers[commandID] = timer
	s.mu.Unlock()
}
