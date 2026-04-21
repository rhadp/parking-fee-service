package store_test

import (
	"fmt"
	"sync"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// TestCommandTimeout verifies a command times out when no response is received (TS-06-3).
func TestCommandTimeout(t *testing.T) {
	s := store.NewStore()
	s.StartTimeout("cmd-003", 100*time.Millisecond)
	time.Sleep(200 * time.Millisecond)
	resp, found := s.GetResponse("cmd-003")
	if !found {
		t.Fatal("GetResponse: expected to find timeout response, got not found")
	}
	if resp.CommandID != "cmd-003" {
		t.Errorf("CommandID: got %q, want %q", resp.CommandID, "cmd-003")
	}
	if resp.Status != "timeout" {
		t.Errorf("Status: got %q, want %q", resp.Status, "timeout")
	}
}

// TestResponseStoreThreadSafety verifies concurrent access is safe (TS-06-5).
func TestResponseStoreThreadSafety(t *testing.T) {
	s := store.NewStore()
	var wg sync.WaitGroup
	const n = 100
	wg.Add(n * 2)
	for i := 0; i < n; i++ {
		i := i
		go func() {
			defer wg.Done()
			s.StoreResponse(model.CommandResponse{
				CommandID: fmt.Sprintf("cmd-%d", i),
				Status:    "success",
			})
		}()
		go func() {
			defer wg.Done()
			s.GetResponse(fmt.Sprintf("cmd-%d", i))
		}()
	}
	wg.Wait()
	for i := 0; i < n; i++ {
		resp, found := s.GetResponse(fmt.Sprintf("cmd-%d", i))
		if !found {
			t.Errorf("GetResponse(cmd-%d): expected found=true", i)
			continue
		}
		if resp.Status != "success" {
			t.Errorf("GetResponse(cmd-%d).Status: got %q, want %q", i, resp.Status, "success")
		}
	}
}

// TestPropertyResponseStoreConsistency verifies StoreResponse followed by GetResponse (TS-06-P2).
func TestPropertyResponseStoreConsistency(t *testing.T) {
	cases := []model.CommandResponse{
		{CommandID: "a", Status: "success"},
		{CommandID: "b", Status: "failed", Reason: "door jammed"},
		{CommandID: "c", Status: "timeout"},
	}
	s := store.NewStore()
	for _, c := range cases {
		s.StoreResponse(c)
		got, found := s.GetResponse(c.CommandID)
		if !found {
			t.Errorf("GetResponse(%q): expected found=true", c.CommandID)
			continue
		}
		if got.CommandID != c.CommandID {
			t.Errorf("CommandID: got %q, want %q", got.CommandID, c.CommandID)
		}
		if got.Status != c.Status {
			t.Errorf("Status: got %q, want %q", got.Status, c.Status)
		}
		if got.Reason != c.Reason {
			t.Errorf("Reason: got %q, want %q", got.Reason, c.Reason)
		}
	}
}

// TestPropertyTimeoutCompleteness verifies no response -> timeout status (TS-06-P3).
func TestPropertyTimeoutCompleteness(t *testing.T) {
	cmdIDs := []string{"prop-cmd-1", "prop-cmd-2", "prop-cmd-3"}
	timeout := 50 * time.Millisecond
	for _, cmdID := range cmdIDs {
		s := store.NewStore()
		s.StartTimeout(cmdID, timeout)
		time.Sleep(timeout + 50*time.Millisecond)
		resp, found := s.GetResponse(cmdID)
		if !found {
			t.Errorf("GetResponse(%q): expected found=true after timeout", cmdID)
			continue
		}
		if resp.Status != "timeout" {
			t.Errorf("Status(%q): got %q, want %q", cmdID, resp.Status, "timeout")
		}
	}
}

// TestPropertyTimeoutCancellation verifies that a response before timeout wins (TS-06-P5).
func TestPropertyTimeoutCancellation(t *testing.T) {
	cmdIDs := []string{"cancel-cmd-1", "cancel-cmd-2", "cancel-cmd-3"}
	for _, cmdID := range cmdIDs {
		s := store.NewStore()
		s.StartTimeout(cmdID, 500*time.Millisecond)
		s.StoreResponse(model.CommandResponse{CommandID: cmdID, Status: "success"})
		time.Sleep(600 * time.Millisecond) // wait past timeout
		resp, found := s.GetResponse(cmdID)
		if !found {
			t.Errorf("GetResponse(%q): expected found=true", cmdID)
			continue
		}
		if resp.Status != "success" {
			t.Errorf("Status(%q): got %q, want %q (timeout must not overwrite real response)", cmdID, resp.Status, "success")
		}
	}
}
