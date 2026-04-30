package store_test

import (
	"fmt"
	"sync"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// TS-06-3: Command Timeout
// Requirement: 06-REQ-1.3
// When no response is received within the timeout, the command status becomes "timeout".
func TestCommandTimeout(t *testing.T) {
	s := store.NewStore()
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
// Requirement: 06-REQ-2.2
// Concurrent writes and reads to the response store do not cause data races.
// Run with -race flag to detect races.
func TestResponseStoreThreadSafety(t *testing.T) {
	s := store.NewStore()
	var wg sync.WaitGroup

	const n = 100
	wg.Add(n * 2)

	// 100 concurrent writers
	for i := range n {
		go func() {
			defer wg.Done()
			s.StoreResponse(model.CommandResponse{
				CommandID: fmt.Sprintf("cmd-%d", i),
				Status:    "success",
			})
		}()
	}

	// 100 concurrent readers
	for i := range n {
		go func() {
			defer wg.Done()
			s.GetResponse(fmt.Sprintf("cmd-%d", i))
		}()
	}

	wg.Wait()

	// After all goroutines complete, every response should be retrievable
	for i := range n {
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
