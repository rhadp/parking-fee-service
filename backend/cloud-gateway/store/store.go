package store

import (
	"sync"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Store holds command responses and timeout timers in memory.
type Store struct {
	mu        sync.Mutex
	responses map[string]model.CommandResponse
	timers    map[string]*time.Timer
}

// NewStore creates an empty Store.
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
	if t, ok := s.timers[resp.CommandID]; ok {
		t.Stop()
		delete(s.timers, resp.CommandID)
	}
}

// GetResponse retrieves the response for a command ID.
func (s *Store) GetResponse(commandID string) (*model.CommandResponse, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	resp, ok := s.responses[commandID]
	if !ok {
		return nil, false
	}
	r := resp
	return &r, true
}

// StartTimeout schedules a timeout response after duration if no response has arrived.
// If a real response is stored before the timer fires, the timer is cancelled and
// the timeout response is never written. If the timer callback runs, it only writes
// if no real response has been stored yet (guards against the Stop()==false race).
func (s *Store) StartTimeout(commandID string, duration time.Duration) {
	s.mu.Lock()
	defer s.mu.Unlock()
	t := time.AfterFunc(duration, func() {
		s.mu.Lock()
		defer s.mu.Unlock()
		// Only write timeout if no real response has been stored yet.
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
