// Package store provides in-memory data storage for the cloud-gateway service.
package store

import (
	"sync"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
)

// CommandStore provides thread-safe in-memory storage for commands.
// It implements FIFO eviction when the maximum size is exceeded.
type CommandStore struct {
	commands map[string]*model.Command
	order    []string // maintains insertion order for FIFO eviction
	maxSize  int
	mu       sync.RWMutex
}

// NewCommandStore creates a new CommandStore with the specified maximum size.
func NewCommandStore(maxSize int) *CommandStore {
	return &CommandStore{
		commands: make(map[string]*model.Command),
		order:    make([]string, 0, maxSize),
		maxSize:  maxSize,
	}
}

// Save stores a command. If the store is at capacity, the oldest command
// is evicted (FIFO eviction).
func (s *CommandStore) Save(cmd *model.Command) {
	s.mu.Lock()
	defer s.mu.Unlock()

	// Check if command already exists (update case)
	if _, exists := s.commands[cmd.CommandID]; exists {
		s.commands[cmd.CommandID] = cmd
		return
	}

	// Evict oldest command if at capacity
	if len(s.commands) >= s.maxSize && s.maxSize > 0 {
		s.evictOldest()
	}

	// Add new command
	s.commands[cmd.CommandID] = cmd
	s.order = append(s.order, cmd.CommandID)
}

// evictOldest removes the oldest command (first in order slice).
// Must be called with lock held.
func (s *CommandStore) evictOldest() {
	if len(s.order) == 0 {
		return
	}

	oldestID := s.order[0]
	s.order = s.order[1:]
	delete(s.commands, oldestID)
}

// Get retrieves a command by ID. Returns nil if not found.
func (s *CommandStore) Get(commandID string) *model.Command {
	s.mu.RLock()
	defer s.mu.RUnlock()

	cmd, exists := s.commands[commandID]
	if !exists {
		return nil
	}

	// Return a copy to prevent external modification
	cmdCopy := *cmd
	if cmd.CompletedAt != nil {
		completedAt := *cmd.CompletedAt
		cmdCopy.CompletedAt = &completedAt
	}
	// Copy the doors slice
	if cmd.Doors != nil {
		cmdCopy.Doors = make([]string, len(cmd.Doors))
		copy(cmdCopy.Doors, cmd.Doors)
	}
	return &cmdCopy
}

// Update modifies an existing command. If the command doesn't exist,
// it is not added to the store.
func (s *CommandStore) Update(cmd *model.Command) bool {
	s.mu.Lock()
	defer s.mu.Unlock()

	if _, exists := s.commands[cmd.CommandID]; !exists {
		return false
	}

	s.commands[cmd.CommandID] = cmd
	return true
}

// GetPendingCommands returns all commands with pending status.
// Used for timeout checking.
func (s *CommandStore) GetPendingCommands() []*model.Command {
	s.mu.RLock()
	defer s.mu.RUnlock()

	var pending []*model.Command
	for _, cmd := range s.commands {
		if cmd.Status == model.CommandStatusPending {
			// Return copies to prevent external modification
			cmdCopy := *cmd
			if cmd.CompletedAt != nil {
				completedAt := *cmd.CompletedAt
				cmdCopy.CompletedAt = &completedAt
			}
			if cmd.Doors != nil {
				cmdCopy.Doors = make([]string, len(cmd.Doors))
				copy(cmdCopy.Doors, cmd.Doors)
			}
			pending = append(pending, &cmdCopy)
		}
	}
	return pending
}

// Count returns the number of commands in the store.
func (s *CommandStore) Count() int {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return len(s.commands)
}

// Clear removes all commands from the store.
func (s *CommandStore) Clear() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.commands = make(map[string]*model.Command)
	s.order = make([]string, 0, s.maxSize)
}
