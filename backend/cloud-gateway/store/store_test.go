package store_test

import (
	"fmt"
	"sync"
	"testing"
	"time"

	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"
)

// TestCommandTimeout verifies that when no response is received within the timeout,
// the command status becomes "timeout".
// Test Spec: TS-06-3
// Requirements: 06-REQ-1.3, 06-REQ-6.3
func TestCommandTimeout(t *testing.T) {
	s := store.NewStore()
	s.StartTimeout("cmd-003", 100*time.Millisecond)
	time.Sleep(200 * time.Millisecond)

	resp, found := s.GetResponse("cmd-003")
	if !found {
		t.Fatal("expected command response after timeout, got not found")
	}
	if resp.CommandID != "cmd-003" {
		t.Errorf("expected command_id 'cmd-003', got %q", resp.CommandID)
	}
	if resp.Status != "timeout" {
		t.Errorf("expected status 'timeout', got %q", resp.Status)
	}
}

// TestResponseStoreThreadSafety verifies that concurrent reads and writes to the
// response store do not cause data races, and all written data is retrievable.
// Test Spec: TS-06-5
// Requirements: 06-REQ-2.2
// Note: run with -race flag to detect data races.
func TestResponseStoreThreadSafety(t *testing.T) {
	s := store.NewStore()
	var wg sync.WaitGroup
	wg.Add(200)

	for i := 0; i < 100; i++ {
		go func(i int) {
			defer wg.Done()
			s.StoreResponse(model.CommandResponse{
				CommandID: fmt.Sprintf("cmd-%d", i),
				Status:    "success",
			})
		}(i)
		go func(i int) {
			defer wg.Done()
			s.GetResponse(fmt.Sprintf("cmd-%d", i))
		}(i)
	}
	wg.Wait()

	for i := 0; i < 100; i++ {
		resp, found := s.GetResponse(fmt.Sprintf("cmd-%d", i))
		if !found {
			t.Errorf("expected cmd-%d to be found after all goroutines completed", i)
			continue
		}
		if resp.Status != "success" {
			t.Errorf("cmd-%d: expected status 'success', got %q", i, resp.Status)
		}
	}
}

// TestPropertyResponseStoreConsistency verifies that for any stored response,
// GetResponse returns the identical data.
// Test Spec: TS-06-P2
// Property: Property 2 from design.md
// Requirements: 06-REQ-2.1, 06-REQ-2.2
func TestPropertyResponseStoreConsistency(t *testing.T) {
	s := store.NewStore()

	cases := []model.CommandResponse{
		{CommandID: "uuid-aaa", Status: "success"},
		{CommandID: "uuid-bbb", Status: "failed", Reason: "door jammed"},
		{CommandID: "uuid-ccc", Status: "timeout"},
		{CommandID: "uuid-ddd", Status: "success", Reason: ""},
	}

	for _, resp := range cases {
		s.StoreResponse(resp)
		got, found := s.GetResponse(resp.CommandID)
		if !found {
			t.Errorf("StoreResponse then GetResponse: %s not found", resp.CommandID)
			continue
		}
		if got.CommandID != resp.CommandID {
			t.Errorf("%s: command_id mismatch: got %q", resp.CommandID, got.CommandID)
		}
		if got.Status != resp.Status {
			t.Errorf("%s: status mismatch: want %q, got %q", resp.CommandID, resp.Status, got.Status)
		}
		if got.Reason != resp.Reason {
			t.Errorf("%s: reason mismatch: want %q, got %q", resp.CommandID, resp.Reason, got.Reason)
		}
	}
}

// TestPropertyTimeoutCompleteness verifies that for any command that receives no
// response, after the timeout duration the status is "timeout".
// Test Spec: TS-06-P3
// Property: Property 3 from design.md
// Requirements: 06-REQ-1.3
func TestPropertyTimeoutCompleteness(t *testing.T) {
	timeoutDuration := 50 * time.Millisecond
	cmdIDs := []string{"p3-cmd-1", "p3-cmd-2", "p3-cmd-3"}

	for _, cmdID := range cmdIDs {
		s := store.NewStore()
		s.StartTimeout(cmdID, timeoutDuration)
		time.Sleep(timeoutDuration + 50*time.Millisecond)

		resp, found := s.GetResponse(cmdID)
		if !found {
			t.Errorf("cmd %s: expected timeout response, got not found", cmdID)
			continue
		}
		if resp.Status != "timeout" {
			t.Errorf("cmd %s: expected status 'timeout', got %q", cmdID, resp.Status)
		}
	}
}

// TestPropertyTimeoutCancellation verifies that when a real response arrives before
// the timeout, the stored status is not "timeout".
// Test Spec: TS-06-P5
// Property: Property 6 from design.md
// Requirements: 06-REQ-1.3, 06-REQ-2.2
func TestPropertyTimeoutCancellation(t *testing.T) {
	cmdIDs := []string{"p5-cmd-1", "p5-cmd-2", "p5-cmd-3"}

	for _, cmdID := range cmdIDs {
		s := store.NewStore()
		s.StartTimeout(cmdID, 500*time.Millisecond)
		// Store a real response before the timeout fires
		s.StoreResponse(model.CommandResponse{
			CommandID: cmdID,
			Status:    "success",
		})
		// Wait past the timeout to confirm it doesn't overwrite
		time.Sleep(600 * time.Millisecond)

		resp, found := s.GetResponse(cmdID)
		if !found {
			t.Errorf("cmd %s: expected response, got not found", cmdID)
			continue
		}
		if resp.Status != "success" {
			t.Errorf("cmd %s: expected status 'success' (not 'timeout'), got %q", cmdID, resp.Status)
		}
	}
}
