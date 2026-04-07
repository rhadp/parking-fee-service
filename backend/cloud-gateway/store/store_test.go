package store

import (
	"fmt"
	"math/rand"
	"sync"
	"testing"
	"time"

	"parking-fee-service/backend/cloud-gateway/model"
)

// TS-06-3: Command Timeout
func TestCommandTimeout(t *testing.T) {
	s := NewStore()
	s.StartTimeout("cmd-003", 100*time.Millisecond)
	time.Sleep(200 * time.Millisecond)

	resp, found := s.GetResponse("cmd-003")
	if !found {
		t.Fatal("expected response to be found after timeout")
	}
	if resp.CommandID != "cmd-003" {
		t.Errorf("CommandID = %q, want %q", resp.CommandID, "cmd-003")
	}
	if resp.Status != "timeout" {
		t.Errorf("Status = %q, want %q", resp.Status, "timeout")
	}
}

// TS-06-5: Response Store Thread Safety
func TestResponseStoreThreadSafety(t *testing.T) {
	s := NewStore()
	var wg sync.WaitGroup

	// 100 concurrent writers
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			s.StoreResponse(model.CommandResponse{
				CommandID: fmt.Sprintf("cmd-%d", i),
				Status:    "success",
			})
		}(i)
	}

	// 100 concurrent readers
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			s.GetResponse(fmt.Sprintf("cmd-%d", i))
		}(i)
	}

	wg.Wait()

	// Verify all responses are retrievable
	for i := 0; i < 100; i++ {
		resp, found := s.GetResponse(fmt.Sprintf("cmd-%d", i))
		if !found {
			t.Errorf("response cmd-%d not found", i)
			continue
		}
		if resp.Status != "success" {
			t.Errorf("cmd-%d status = %q, want %q", i, resp.Status, "success")
		}
	}
}

// TS-06-P2: Property - Response Store Consistency
func TestPropertyResponseStoreConsistency(t *testing.T) {
	s := NewStore()
	rng := rand.New(rand.NewSource(42))

	for i := 0; i < 100; i++ {
		resp := model.CommandResponse{
			CommandID: fmt.Sprintf("cmd-%d", rng.Int()),
			Status:    []string{"success", "failed", "timeout"}[rng.Intn(3)],
			Reason:    fmt.Sprintf("reason-%d", i),
		}
		s.StoreResponse(resp)
		got, found := s.GetResponse(resp.CommandID)
		if !found {
			t.Errorf("response %q not found after StoreResponse", resp.CommandID)
			continue
		}
		if got.CommandID != resp.CommandID {
			t.Errorf("CommandID = %q, want %q", got.CommandID, resp.CommandID)
		}
		if got.Status != resp.Status {
			t.Errorf("Status = %q, want %q", got.Status, resp.Status)
		}
		if got.Reason != resp.Reason {
			t.Errorf("Reason = %q, want %q", got.Reason, resp.Reason)
		}
	}
}

// TS-06-P3: Property - Timeout Completeness
func TestPropertyTimeoutCompleteness(t *testing.T) {
	for i := 0; i < 10; i++ {
		s := NewStore()
		cmdID := fmt.Sprintf("timeout-cmd-%d", i)
		timeout := 50 * time.Millisecond
		s.StartTimeout(cmdID, timeout)
		time.Sleep(timeout + 50*time.Millisecond)

		resp, found := s.GetResponse(cmdID)
		if !found {
			t.Errorf("response %q not found after timeout", cmdID)
			continue
		}
		if resp.Status != "timeout" {
			t.Errorf("status = %q, want %q for %q", resp.Status, "timeout", cmdID)
		}
	}
}

// TS-06-P5: Property - Timeout Cancellation
func TestPropertyTimeoutCancellation(t *testing.T) {
	for i := 0; i < 10; i++ {
		s := NewStore()
		cmdID := fmt.Sprintf("cancel-cmd-%d", i)
		s.StartTimeout(cmdID, 500*time.Millisecond)
		// Store a real response before timeout fires
		s.StoreResponse(model.CommandResponse{
			CommandID: cmdID,
			Status:    "success",
		})
		// Wait past the timeout duration
		time.Sleep(600 * time.Millisecond)

		resp, found := s.GetResponse(cmdID)
		if !found {
			t.Errorf("response %q not found", cmdID)
			continue
		}
		if resp.Status != "success" {
			t.Errorf("status = %q, want %q (timeout should have been cancelled)", resp.Status, "success")
		}
	}
}
