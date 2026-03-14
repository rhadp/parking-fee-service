// Package store provides an in-memory command status store.
package store

import (
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// Store is a thread-safe in-memory store for command statuses.
type Store struct{}

// NewStore creates a new empty command store.
func NewStore() *Store {
	return nil
}

// Add stores a command status entry.
func (s *Store) Add(cmd model.CommandStatus) {
}

// Get retrieves a command status by command ID.
func (s *Store) Get(commandID string) (*model.CommandStatus, bool) {
	return nil, false
}

// UpdateFromResponse updates an existing command status from a command response.
func (s *Store) UpdateFromResponse(resp model.CommandResponse) {
}

// ExpireTimedOut marks any pending command older than timeout as "timeout".
func (s *Store) ExpireTimedOut(timeout time.Duration) {
}
