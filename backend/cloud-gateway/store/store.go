package store

import (
	"sync"
	"time"

	"parking-fee-service/backend/cloud-gateway/model"
)

// Store holds command responses in memory with timeout management.
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

// StoreResponse stores a command response, cancelling any pending timeout timer.
// If a timer exists, it is stopped. Even if Stop() returns false (timer has already
// fired), the timer callback checks for an existing response under the mutex before
// writing, so it will not overwrite a legitimate response with "timeout".
func (s *Store) StoreResponse(resp model.CommandResponse) {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.responses[resp.CommandID] = resp

	// Cancel any pending timeout timer. Even if Stop() returns false (the timer
	// has already fired and is queued), the timer goroutine checks for an
	// existing response before overwriting, so the "timeout" will not win.
	if t, ok := s.timers[resp.CommandID]; ok {
		t.Stop()
		delete(s.timers, resp.CommandID)
	}
}

// GetResponse returns the stored response for commandID, or (nil, false) if not found.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()

	resp, ok := s.responses[commandID]
	if !ok {
		return nil, false
	}
	// Return a copy to avoid callers mutating stored state.
	copy := resp
	return &copy, true
}

// StartTimeout starts a timer that, after duration, stores a timeout response for
// commandID — but only if no response has already been stored. This guards against
// the race where Stop() returns false yet a real response was already written.
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	s.mu.Lock()
	defer s.mu.Unlock()

	t := time.AfterFunc(duration, func() {
		s.mu.Lock()
		defer s.mu.Unlock()

		// Only write the timeout if no real response has arrived yet.
		if _, exists := s.responses[commandID]; !exists {
			s.responses[commandID] = model.CommandResponse{
				CommandID: commandID,
				Status:    "timeout",
			}
		}
		// Clean up the timer entry regardless.
		delete(s.timers, commandID)
	})

	s.timers[commandID] = t
}
