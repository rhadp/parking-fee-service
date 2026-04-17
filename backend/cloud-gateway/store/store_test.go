package store_test

import (
	"fmt"
	"sync"
	"testing"
	"time"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/model"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/store"
)

// TestCommandTimeout verifies that when no response is received within the timeout
// the store records a response with status "timeout".
// TS-06-3
func TestCommandTimeout(t *testing.T) {
	s := store.NewStore()
	s.StartTimeout("cmd-003", 100*time.Millisecond)
	time.Sleep(200 * time.Millisecond)

	resp, found := s.GetResponse("cmd-003")
	if !found {
		t.Fatal("GetResponse('cmd-003'): want found=true, got false")
	}
	if resp.CommandID != "cmd-003" {
		t.Errorf("CommandID: want 'cmd-003', got %q", resp.CommandID)
	}
	if resp.Status != "timeout" {
		t.Errorf("Status: want 'timeout', got %q", resp.Status)
	}
}

// TestResponseStoreThreadSafety verifies that concurrent writes and reads do not cause
// data races and that all stored responses are retrievable.
// TS-06-5
func TestResponseStoreThreadSafety(t *testing.T) {
	s := store.NewStore()
	const n = 100
	var wg sync.WaitGroup
	wg.Add(n * 2)

	for i := range n {
		id := fmt.Sprintf("cmd-%d", i)
		go func(id string) {
			defer wg.Done()
			s.StoreResponse(model.CommandResponse{CommandID: id, Status: "success"})
		}(id)
		go func(id string) {
			defer wg.Done()
			s.GetResponse(id)
		}(id)
	}
	wg.Wait()

	for i := range n {
		id := fmt.Sprintf("cmd-%d", i)
		resp, found := s.GetResponse(id)
		if !found {
			t.Errorf("GetResponse(%q): want found=true, got false", id)
			continue
		}
		if resp.Status != "success" {
			t.Errorf("GetResponse(%q): want status='success', got %q", id, resp.Status)
		}
	}
}

// TestPropertyResponseStoreConsistency verifies that for any stored response,
// GetResponse returns the exact same data.
// TS-06-P2
func TestPropertyResponseStoreConsistency(t *testing.T) {
	s := store.NewStore()

	responses := []model.CommandResponse{
		{CommandID: "a", Status: "success"},
		{CommandID: "b", Status: "failed", Reason: "door jammed"},
		{CommandID: "c", Status: "timeout"},
		{CommandID: "d", Status: "success", Reason: ""},
	}

	for _, resp := range responses {
		s.StoreResponse(resp)
	}

	for _, want := range responses {
		t.Run(want.CommandID, func(t *testing.T) {
			got, found := s.GetResponse(want.CommandID)
			if !found {
				t.Fatalf("GetResponse(%q): want found=true, got false", want.CommandID)
			}
			if got.CommandID != want.CommandID {
				t.Errorf("CommandID: want %q, got %q", want.CommandID, got.CommandID)
			}
			if got.Status != want.Status {
				t.Errorf("Status: want %q, got %q", want.Status, got.Status)
			}
			if got.Reason != want.Reason {
				t.Errorf("Reason: want %q, got %q", want.Reason, got.Reason)
			}
		})
	}
}

// TestPropertyTimeoutCompleteness verifies that for any command with no response,
// after the timeout elapses the store contains a response with status "timeout".
// TS-06-P3
func TestPropertyTimeoutCompleteness(t *testing.T) {
	cmdIDs := []string{"prop-timeout-1", "prop-timeout-2", "prop-timeout-3"}
	timeout := 50 * time.Millisecond

	for _, id := range cmdIDs {
		t.Run(id, func(t *testing.T) {
			s := store.NewStore()
			s.StartTimeout(id, timeout)
			time.Sleep(timeout + 50*time.Millisecond)

			resp, found := s.GetResponse(id)
			if !found {
				t.Fatalf("GetResponse(%q): want found=true, got false", id)
			}
			if resp.Status != "timeout" {
				t.Errorf("Status: want 'timeout', got %q", resp.Status)
			}
		})
	}
}

// TestPropertyTimeoutCancellation verifies that for any command that receives a real
// response before the timeout fires, the stored status is not "timeout".
// TS-06-P5
func TestPropertyTimeoutCancellation(t *testing.T) {
	cmdIDs := []string{"cancel-1", "cancel-2", "cancel-3"}
	timeout := 500 * time.Millisecond

	for _, id := range cmdIDs {
		t.Run(id, func(t *testing.T) {
			s := store.NewStore()
			s.StartTimeout(id, timeout)
			// Store a real response before the timeout fires.
			s.StoreResponse(model.CommandResponse{CommandID: id, Status: "success"})
			// Wait past the timeout to confirm it was cancelled.
			time.Sleep(timeout + 100*time.Millisecond)

			resp, _ := s.GetResponse(id)
			if resp == nil {
				t.Fatalf("GetResponse(%q): want non-nil response", id)
			}
			if resp.Status == "timeout" {
				t.Errorf("Status: want 'success', got 'timeout' (timeout was not cancelled)")
			}
		})
	}
}
