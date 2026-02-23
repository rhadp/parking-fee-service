// Package bridge provides the REST-to-MQTT bridge logic for the CLOUD_GATEWAY
// service. The Command Tracker manages pending commands and correlates MQTT
// responses with their originating REST requests.
package bridge

import (
	"log"
	"sync"
	"time"
)

// CommandResponse holds the response from a vehicle command, received via MQTT.
type CommandResponse struct {
	// CommandID is the unique identifier for the command.
	CommandID string `json:"command_id"`

	// Status is "success", "failed", or "timeout".
	Status string `json:"status"`

	// Reason provides optional detail for failures.
	Reason string `json:"reason,omitempty"`

	// Timestamp is the Unix epoch time of the response.
	Timestamp int64 `json:"timestamp,omitempty"`
}

// Tracker manages pending command requests and correlates them with MQTT
// responses using command_id. Each pending command has a channel that the REST
// handler blocks on (with timeout). When a matching MQTT response arrives,
// the tracker writes to the channel, unblocking the handler.
//
// Thread-safe: all operations are protected by a mutex.
type Tracker struct {
	mu       sync.Mutex
	pending  map[string]chan CommandResponse
	timeout  time.Duration
}

// NewTracker creates a new command tracker with the specified timeout duration.
// The timeout determines how long a pending command waits for an MQTT response
// before being resolved with a "timeout" status.
func NewTracker(timeout time.Duration) *Tracker {
	return &Tracker{
		pending: make(map[string]chan CommandResponse),
		timeout: timeout,
	}
}

// Register adds a pending command with the given ID and returns a channel that
// will receive the command response (or a timeout response). The caller should
// read exactly one value from the returned channel.
//
// A background goroutine enforces the timeout: if no response arrives within
// the configured timeout duration, a timeout response is sent on the channel
// and the pending entry is removed.
func (t *Tracker) Register(commandID string) <-chan CommandResponse {
	ch := make(chan CommandResponse, 1)

	t.mu.Lock()
	t.pending[commandID] = ch
	t.mu.Unlock()

	// Start timeout goroutine
	go func() {
		timer := time.NewTimer(t.timeout)
		defer timer.Stop()

		<-timer.C

		t.mu.Lock()
		// Only send timeout if still pending (not already resolved)
		if _, exists := t.pending[commandID]; exists {
			delete(t.pending, commandID)
			t.mu.Unlock()
			ch <- CommandResponse{
				CommandID: commandID,
				Status:    "timeout",
			}
		} else {
			t.mu.Unlock()
		}
	}()

	return ch
}

// Resolve delivers a response to a pending command identified by commandID.
// Returns true if the command was pending and the response was delivered,
// false if the commandID is unknown (already resolved, timed out, or never
// registered).
//
// If the commandID is unknown, a warning is logged and the response is
// discarded (per 03-REQ-3.E1). If the command was already resolved, the
// duplicate is ignored (per 03-REQ-3.E2).
func (t *Tracker) Resolve(commandID string, resp CommandResponse) bool {
	t.mu.Lock()
	ch, exists := t.pending[commandID]
	if exists {
		delete(t.pending, commandID)
	}
	t.mu.Unlock()

	if !exists {
		log.Printf("WARN: received response for unknown command_id %q (discarded)", commandID)
		return false
	}

	resp.CommandID = commandID
	ch <- resp
	return true
}

// HasPending returns true if a command with the given ID is currently pending.
func (t *Tracker) HasPending(commandID string) bool {
	t.mu.Lock()
	defer t.mu.Unlock()
	_, exists := t.pending[commandID]
	return exists
}

// PendingCount returns the number of currently pending commands.
func (t *Tracker) PendingCount() int {
	t.mu.Lock()
	defer t.mu.Unlock()
	return len(t.pending)
}
