package property

import (
	"fmt"
	"testing"
	"time"

	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/store"
)

// TestCommandStoreFIFOEviction tests Property 12: Command Store FIFO Eviction.
// For any sequence of commands exceeding the maximum store size (100), the oldest
// commands SHALL be evicted first. After inserting N commands where N > max_size,
// only the most recent max_size commands SHALL be retrievable.
func TestCommandStoreFIFOEviction(t *testing.T) {
	// Feature: cloud-gateway, Property 12: Command Store FIFO Eviction
	// Validates: Requirements 12.1, 12.2

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	// Use a smaller max size for testing
	maxSize := 10

	properties.Property("oldest commands evicted first when store exceeds max size", prop.ForAll(
		func(numCommands int) bool {
			s := store.NewCommandStore(maxSize)

			var allIDs []string
			for i := 0; i < numCommands; i++ {
				cmd := &model.Command{
					CommandID:   fmt.Sprintf("cmd-%d", i),
					CommandType: model.CommandTypeLock,
					Status:      model.CommandStatusPending,
					CreatedAt:   time.Now(),
				}
				s.Save(cmd)
				allIDs = append(allIDs, cmd.CommandID)
			}

			// Determine which commands should still exist
			expectedStart := 0
			if numCommands > maxSize {
				expectedStart = numCommands - maxSize
			}

			// Check that only the most recent maxSize commands exist
			for i, id := range allIDs {
				cmd := s.Get(id)
				if i < expectedStart {
					// Should have been evicted
					if cmd != nil {
						t.Logf("Command %s at index %d should have been evicted but was found", id, i)
						return false
					}
				} else {
					// Should still exist
					if cmd == nil {
						t.Logf("Command %s at index %d should exist but was not found", id, i)
						return false
					}
				}
			}

			// Verify count
			expectedCount := numCommands
			if numCommands > maxSize {
				expectedCount = maxSize
			}
			if s.Count() != expectedCount {
				t.Logf("Expected count %d, got %d", expectedCount, s.Count())
				return false
			}

			return true
		},
		gen.IntRange(1, 30),
	))

	properties.Property("store never exceeds max size", prop.ForAll(
		func(numCommands int) bool {
			s := store.NewCommandStore(maxSize)

			for i := 0; i < numCommands; i++ {
				cmd := &model.Command{
					CommandID:   fmt.Sprintf("cmd-%d", i),
					CommandType: model.CommandTypeLock,
					Status:      model.CommandStatusPending,
					CreatedAt:   time.Now(),
				}
				s.Save(cmd)

				// Store should never exceed max size
				if s.Count() > maxSize {
					t.Logf("Store count %d exceeds max size %d after inserting command %d", s.Count(), maxSize, i)
					return false
				}
			}

			return true
		},
		gen.IntRange(1, 50),
	))

	properties.Property("commands are retrievable after save", prop.ForAll(
		func(numCommands int) bool {
			// Use larger store to avoid eviction
			s := store.NewCommandStore(100)

			for i := 0; i < numCommands; i++ {
				cmd := &model.Command{
					CommandID:   fmt.Sprintf("cmd-%d", i),
					CommandType: model.CommandTypeLock,
					Doors:       []string{model.DoorAll},
					Status:      model.CommandStatusPending,
					CreatedAt:   time.Now(),
				}
				s.Save(cmd)

				// Should be immediately retrievable
				retrieved := s.Get(cmd.CommandID)
				if retrieved == nil {
					t.Logf("Command %s not retrievable after save", cmd.CommandID)
					return false
				}
				if retrieved.CommandID != cmd.CommandID {
					t.Logf("Retrieved command ID %s doesn't match saved ID %s", retrieved.CommandID, cmd.CommandID)
					return false
				}
			}

			return true
		},
		gen.IntRange(1, 50),
	))

	properties.TestingRun(t)
}

// TestCommandStoreGetPendingCommands tests that GetPendingCommands returns
// only commands with pending status.
func TestCommandStoreGetPendingCommands(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	properties.Property("GetPendingCommands returns only pending commands", prop.ForAll(
		func(numPending, numSuccess, numFailed int) bool {
			s := store.NewCommandStore(100)

			// Add pending commands
			for i := 0; i < numPending; i++ {
				s.Save(&model.Command{
					CommandID: fmt.Sprintf("pending-%d", i),
					Status:    model.CommandStatusPending,
					CreatedAt: time.Now(),
				})
			}

			// Add success commands
			for i := 0; i < numSuccess; i++ {
				now := time.Now()
				s.Save(&model.Command{
					CommandID:   fmt.Sprintf("success-%d", i),
					Status:      model.CommandStatusSuccess,
					CreatedAt:   time.Now(),
					CompletedAt: &now,
				})
			}

			// Add failed commands
			for i := 0; i < numFailed; i++ {
				now := time.Now()
				s.Save(&model.Command{
					CommandID:   fmt.Sprintf("failed-%d", i),
					Status:      model.CommandStatusFailed,
					CreatedAt:   time.Now(),
					CompletedAt: &now,
				})
			}

			pending := s.GetPendingCommands()

			// Should have exactly numPending commands
			if len(pending) != numPending {
				t.Logf("Expected %d pending commands, got %d", numPending, len(pending))
				return false
			}

			// All returned commands should have pending status
			for _, cmd := range pending {
				if cmd.Status != model.CommandStatusPending {
					t.Logf("Command %s has status %s, expected pending", cmd.CommandID, cmd.Status)
					return false
				}
			}

			return true
		},
		gen.IntRange(0, 20),
		gen.IntRange(0, 20),
		gen.IntRange(0, 20),
	))

	properties.TestingRun(t)
}
