package bridge

import (
	"sync"
	"testing"
	"time"
)

// TestTracker_Resolve verifies that a pending command is resolved when Resolve
// is called with a matching command_id.
// TS-03-10 (03-REQ-2.5)
func TestTracker_Resolve(t *testing.T) {
	tracker := NewTracker(5 * time.Second)

	ch := tracker.Register("cmd-001")

	go func() {
		tracker.Resolve("cmd-001", CommandResponse{Status: "success"})
	}()

	select {
	case resp := <-ch:
		if resp.Status != "success" {
			t.Errorf("expected status 'success', got %q", resp.Status)
		}
		if resp.CommandID != "cmd-001" {
			t.Errorf("expected command_id 'cmd-001', got %q", resp.CommandID)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for resolve")
	}

	if tracker.HasPending("cmd-001") {
		t.Error("expected cmd-001 to no longer be pending after resolve")
	}
}

// TestTracker_MatchByID verifies that resolving one command does not affect
// another pending command.
// TS-03-12 (03-REQ-3.2)
func TestTracker_MatchByID(t *testing.T) {
	tracker := NewTracker(5 * time.Second)

	chA := tracker.Register("cmd-A")
	_ = tracker.Register("cmd-B")

	// Resolve only cmd-B
	resolved := tracker.Resolve("cmd-B", CommandResponse{Status: "success"})
	if !resolved {
		t.Error("expected Resolve to return true for pending cmd-B")
	}

	// cmd-A should still be pending
	if !tracker.HasPending("cmd-A") {
		t.Error("expected cmd-A to still be pending")
	}

	// cmd-B should no longer be pending
	if tracker.HasPending("cmd-B") {
		t.Error("expected cmd-B to no longer be pending")
	}

	// Clean up cmd-A
	go func() {
		tracker.Resolve("cmd-A", CommandResponse{Status: "success"})
	}()
	<-chA
}

// TestTracker_UnknownID verifies that resolving a non-existent command_id
// returns false and does not panic.
// TS-03-E6 (03-REQ-3.E1)
func TestTracker_UnknownID(t *testing.T) {
	tracker := NewTracker(5 * time.Second)

	resolved := tracker.Resolve("ghost-cmd", CommandResponse{Status: "success"})
	if resolved {
		t.Error("expected Resolve to return false for unknown command_id")
	}

	// Verify no panic and tracker is still functional
	if tracker.PendingCount() != 0 {
		t.Errorf("expected 0 pending, got %d", tracker.PendingCount())
	}
}

// TestTracker_Duplicate verifies that only the first resolve for a command_id
// succeeds; subsequent resolves are ignored.
// TS-03-E7 (03-REQ-3.E2)
func TestTracker_Duplicate(t *testing.T) {
	tracker := NewTracker(5 * time.Second)

	ch := tracker.Register("cmd-dup")

	// First resolve
	resolved1 := tracker.Resolve("cmd-dup", CommandResponse{Status: "success"})
	if !resolved1 {
		t.Error("expected first Resolve to return true")
	}

	// Read the response
	select {
	case resp := <-ch:
		if resp.Status != "success" {
			t.Errorf("expected status 'success', got %q", resp.Status)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for first resolve")
	}

	// Second resolve should be ignored
	resolved2 := tracker.Resolve("cmd-dup", CommandResponse{Status: "failed"})
	if resolved2 {
		t.Error("expected second Resolve to return false (duplicate)")
	}
}

// TestTracker_Isolation verifies that commands for different VINs are tracked
// independently.
// TS-03-23 (03-REQ-5.2)
func TestTracker_Isolation(t *testing.T) {
	tracker := NewTracker(5 * time.Second)

	chA := tracker.Register("cmd-A")
	chB := tracker.Register("cmd-B")

	// Resolve cmd-A
	tracker.Resolve("cmd-A", CommandResponse{Status: "success"})

	select {
	case resp := <-chA:
		if resp.Status != "success" {
			t.Errorf("cmd-A: expected status 'success', got %q", resp.Status)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for cmd-A resolve")
	}

	// cmd-B should still be pending
	if !tracker.HasPending("cmd-B") {
		t.Error("expected cmd-B to still be pending")
	}

	// Clean up
	go func() {
		tracker.Resolve("cmd-B", CommandResponse{Status: "success"})
	}()
	<-chB
}

// TestTracker_ResponseCorrelation verifies that the tracker correctly passes
// through the response status for both "success" and "failed".
// TS-03-P2 (Property 2: Response Correlation Correctness)
func TestTracker_ResponseCorrelation(t *testing.T) {
	statuses := []string{"success", "failed"}

	for _, status := range statuses {
		t.Run(status, func(t *testing.T) {
			tracker := NewTracker(5 * time.Second)
			ch := tracker.Register("cmd-" + status)

			go func() {
				tracker.Resolve("cmd-"+status, CommandResponse{
					Status: status,
					Reason: "test-reason-" + status,
				})
			}()

			select {
			case resp := <-ch:
				if resp.Status != status {
					t.Errorf("expected status %q, got %q", status, resp.Status)
				}
				if resp.CommandID != "cmd-"+status {
					t.Errorf("expected command_id 'cmd-%s', got %q", status, resp.CommandID)
				}
			case <-time.After(2 * time.Second):
				t.Fatalf("timed out waiting for response with status %q", status)
			}
		})
	}
}

// TestTracker_Timeout verifies that a pending command times out and receives
// a "timeout" response when no MQTT response arrives.
// TS-03-P5 (Property 5: Timeout Guarantee)
func TestTracker_Timeout(t *testing.T) {
	// Use a very short timeout for testing
	tracker := NewTracker(100 * time.Millisecond)

	ch := tracker.Register("will-timeout")

	select {
	case resp := <-ch:
		if resp.Status != "timeout" {
			t.Errorf("expected status 'timeout', got %q", resp.Status)
		}
		if resp.CommandID != "will-timeout" {
			t.Errorf("expected command_id 'will-timeout', got %q", resp.CommandID)
		}
	case <-time.After(2 * time.Second):
		t.Fatal("timed out waiting for timeout response")
	}

	if tracker.HasPending("will-timeout") {
		t.Error("expected timed-out command to be removed from pending")
	}
}

// TestTracker_MultiVehicleIsolation verifies that concurrent commands for
// different VINs receive the correct responses when resolved in reverse order.
// TS-03-P6 (Property 6: Multi-Vehicle Isolation)
func TestTracker_MultiVehicleIsolation(t *testing.T) {
	tracker := NewTracker(5 * time.Second)

	chA := tracker.Register("cmd-for-VIN_A")
	chB := tracker.Register("cmd-for-VIN_B")

	// Resolve in reverse order
	tracker.Resolve("cmd-for-VIN_B", CommandResponse{Status: "success", Reason: "B"})
	tracker.Resolve("cmd-for-VIN_A", CommandResponse{Status: "failed", Reason: "A"})

	var respA, respB CommandResponse
	var wg sync.WaitGroup
	wg.Add(2)

	go func() {
		defer wg.Done()
		select {
		case respA = <-chA:
		case <-time.After(2 * time.Second):
			t.Error("timed out waiting for VIN_A response")
		}
	}()

	go func() {
		defer wg.Done()
		select {
		case respB = <-chB:
		case <-time.After(2 * time.Second):
			t.Error("timed out waiting for VIN_B response")
		}
	}()

	wg.Wait()

	if respA.Reason != "A" {
		t.Errorf("VIN_A: expected reason 'A', got %q", respA.Reason)
	}
	if respB.Reason != "B" {
		t.Errorf("VIN_B: expected reason 'B', got %q", respB.Reason)
	}
	if respA.Status != "failed" {
		t.Errorf("VIN_A: expected status 'failed', got %q", respA.Status)
	}
	if respB.Status != "success" {
		t.Errorf("VIN_B: expected status 'success', got %q", respB.Status)
	}
}

// TestTracker_ConcurrentAccess verifies the tracker is safe under concurrent
// access from multiple goroutines.
func TestTracker_ConcurrentAccess(t *testing.T) {
	tracker := NewTracker(5 * time.Second)

	const n = 50
	var wg sync.WaitGroup
	channels := make([]<-chan CommandResponse, n)

	// Register all commands
	for i := 0; i < n; i++ {
		cmdID := "concurrent-" + string(rune('A'+i%26)) + string(rune('0'+i/26))
		channels[i] = tracker.Register(cmdID)
	}

	// Resolve all concurrently
	for i := 0; i < n; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			cmdID := "concurrent-" + string(rune('A'+idx%26)) + string(rune('0'+idx/26))
			tracker.Resolve(cmdID, CommandResponse{Status: "success"})
		}(i)
	}

	// Read all responses
	for i := 0; i < n; i++ {
		select {
		case <-channels[i]:
		case <-time.After(5 * time.Second):
			t.Fatalf("timed out waiting for response %d", i)
		}
	}

	wg.Wait()

	if tracker.PendingCount() != 0 {
		t.Errorf("expected 0 pending after all resolved, got %d", tracker.PendingCount())
	}
}

// TestTracker_TimeoutDoesNotResolveAlreadyResolved verifies that the timeout
// goroutine does not interfere with an already-resolved command.
func TestTracker_TimeoutDoesNotResolveAlreadyResolved(t *testing.T) {
	tracker := NewTracker(200 * time.Millisecond)

	ch := tracker.Register("race-cmd")

	// Resolve immediately, before timeout
	resolved := tracker.Resolve("race-cmd", CommandResponse{Status: "success"})
	if !resolved {
		t.Fatal("expected Resolve to return true")
	}

	// Read the immediate response
	select {
	case resp := <-ch:
		if resp.Status != "success" {
			t.Errorf("expected 'success', got %q", resp.Status)
		}
	case <-time.After(100 * time.Millisecond):
		t.Fatal("expected immediate response")
	}

	// Wait for the timeout goroutine to fire (it should be a no-op)
	time.Sleep(300 * time.Millisecond)

	// Channel should be empty (no second message from timeout)
	select {
	case resp := <-ch:
		t.Errorf("unexpected second response from timeout goroutine: %+v", resp)
	default:
		// Expected: no second message
	}
}
