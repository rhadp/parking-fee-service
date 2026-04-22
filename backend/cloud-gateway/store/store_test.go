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
func TestCommandTimeout(t *testing.T) {
	s := store.NewStore()
	s.StartTimeout("cmd-003", 100*time.Millisecond)
	time.Sleep(200 * time.Millisecond)

	resp, found := s.GetResponse("cmd-003")
	if !found {
		t.Fatal("expected timeout response to be stored")
	}
	if resp.CommandID != "cmd-003" {
		t.Errorf("expected command_id cmd-003, got %s", resp.CommandID)
	}
	if resp.Status != "timeout" {
		t.Errorf("expected status 'timeout', got '%s'", resp.Status)
	}
}

// TS-06-5: Response Store Thread Safety
// Requirement: 06-REQ-2.2
func TestResponseStoreThreadSafety(t *testing.T) {
	s := store.NewStore()
	var wg sync.WaitGroup

	// 100 concurrent writers.
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

	// 100 concurrent readers.
	for i := 0; i < 100; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			s.GetResponse(fmt.Sprintf("cmd-%d", i))
		}(i)
	}

	wg.Wait()

	// Verify all responses are retrievable.
	for i := 0; i < 100; i++ {
		resp, found := s.GetResponse(fmt.Sprintf("cmd-%d", i))
		if !found {
			t.Errorf("expected response for cmd-%d to be stored", i)
			continue
		}
		if resp.Status != "success" {
			t.Errorf("expected status 'success' for cmd-%d, got '%s'", i, resp.Status)
		}
	}
}

// TS-06-P2: Response Store Consistency (Property Test)
// Property 2: For any stored response, GetResponse returns the same data.
// Validates: 06-REQ-2.1, 06-REQ-2.2
func TestPropertyResponseStoreConsistency(t *testing.T) {
	responses := []model.CommandResponse{
		{CommandID: "p2-001", Status: "success", Reason: ""},
		{CommandID: "p2-002", Status: "failed", Reason: "door jammed"},
		{CommandID: "p2-003", Status: "timeout", Reason: ""},
		{CommandID: "p2-004", Status: "success", Reason: "completed quickly"},
		{CommandID: "p2-005", Status: "failed", Reason: "battery low"},
	}

	s := store.NewStore()
	for _, resp := range responses {
		s.StoreResponse(resp)
	}

	for _, want := range responses {
		t.Run(want.CommandID, func(t *testing.T) {
			got, found := s.GetResponse(want.CommandID)
			if !found {
				t.Fatalf("expected response for %s to be found", want.CommandID)
			}
			if got.CommandID != want.CommandID {
				t.Errorf("CommandID: expected %s, got %s", want.CommandID, got.CommandID)
			}
			if got.Status != want.Status {
				t.Errorf("Status: expected %s, got %s", want.Status, got.Status)
			}
			if got.Reason != want.Reason {
				t.Errorf("Reason: expected %q, got %q", want.Reason, got.Reason)
			}
		})
	}
}

// TS-06-P3: Timeout Completeness (Property Test)
// Property 3: For any command with no response, after the timeout the status is "timeout".
// Validates: 06-REQ-1.3
func TestPropertyTimeoutCompleteness(t *testing.T) {
	commandIDs := []string{
		"p3-001", "p3-002", "p3-003", "p3-004", "p3-005",
		"p3-006", "p3-007", "p3-008", "p3-009", "p3-010",
	}

	for _, cmdID := range commandIDs {
		t.Run(cmdID, func(t *testing.T) {
			s := store.NewStore()
			timeout := 50 * time.Millisecond
			s.StartTimeout(cmdID, timeout)
			time.Sleep(timeout + 50*time.Millisecond)

			resp, found := s.GetResponse(cmdID)
			if !found {
				t.Fatalf("expected timeout response for %s", cmdID)
			}
			if resp.Status != "timeout" {
				t.Errorf("expected status 'timeout' for %s, got '%s'", cmdID, resp.Status)
			}
		})
	}
}

// TS-06-P5: Timeout Cancellation (Property Test)
// Property 6: For any command that receives a response before timeout,
// the stored status matches the NATS response, not "timeout".
// Validates: 06-REQ-1.3, 06-REQ-2.2
func TestPropertyTimeoutCancellation(t *testing.T) {
	commandIDs := []string{
		"p5-001", "p5-002", "p5-003", "p5-004", "p5-005",
	}

	for _, cmdID := range commandIDs {
		t.Run(cmdID, func(t *testing.T) {
			s := store.NewStore()
			s.StartTimeout(cmdID, 500*time.Millisecond)

			// Store a real response before the timeout fires.
			s.StoreResponse(model.CommandResponse{
				CommandID: cmdID,
				Status:    "success",
			})

			// Wait past the timeout duration.
			time.Sleep(600 * time.Millisecond)

			resp, found := s.GetResponse(cmdID)
			if !found {
				t.Fatalf("expected response for %s", cmdID)
			}
			if resp.Status != "success" {
				t.Errorf("expected status 'success' for %s (not overwritten by timeout), got '%s'",
					cmdID, resp.Status)
			}
		})
	}
}
