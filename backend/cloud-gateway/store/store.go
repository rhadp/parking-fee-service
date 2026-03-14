// Package store provides an in-memory command status store.
package store

import (
	"sync"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Store is a thread-safe in-memory store for command statuses.
type Store struct {
	mu       sync.Mutex
	commands map[string]*model.CommandStatus
}

// NewStore creates a new empty command store.
func NewStore() *Store {
	return &Store{
		commands: make(map[string]*model.CommandStatus),
	}
}

// Add stores a command status entry.
func (s *Store) Add(cmd model.CommandStatus) {
	s.mu.Lock()
	defer s.mu.Unlock()
	entry := cmd
	s.commands[cmd.CommandID] = &entry
}

// Get retrieves a command status by command ID.
func (s *Store) Get(commandID string) (*model.CommandStatus, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	cs, ok := s.commands[commandID]
	if !ok {
		return nil, false
	}
	// Return a copy to avoid data races on the caller side.
	copy := *cs
	return &copy, true
}

// UpdateFromResponse updates an existing command status from a command response.
// If the command ID is not found, the call is a no-op.
func (s *Store) UpdateFromResponse(resp model.CommandResponse) {
	s.mu.Lock()
	defer s.mu.Unlock()
	cs, ok := s.commands[resp.CommandID]
	if !ok {
		return
	}
	cs.Status = resp.Status
	cs.Reason = resp.Reason
}

// ExpireTimedOut marks any pending command older than timeout as "timeout".
func (s *Store) ExpireTimedOut(timeout time.Duration) {
	now := time.Now()
	s.mu.Lock()
	defer s.mu.Unlock()
	for _, cs := range s.commands {
		if cs.Status == "pending" && now.Sub(cs.CreatedAt) > timeout {
			cs.Status = "timeout"
		}
	}
}
