package store

import (
	"fmt"
	"sync"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// ---------------------------------------------------------------------------
// TS-06-3: Command Timeout
// Requirement: 06-REQ-1.3
// ---------------------------------------------------------------------------

func TestCommandTimeout(t *testing.T) {
	s := NewStore()
	s.StartTimeout("cmd-003", 100*time.Millisecond)
	time.Sleep(200 * time.Millisecond)

	resp, found := s.GetResponse("cmd-003")
	if !found {
		t.Fatal("expected response to be found after timeout")
	}
	if resp.CommandID != "cmd-003" {
		t.Errorf("expected CommandID 'cmd-003', got %q", resp.CommandID)
	}
	if resp.Status != "timeout" {
		t.Errorf("expected Status 'timeout', got %q", resp.Status)
	}
}

// ---------------------------------------------------------------------------
// TS-06-5: Response Store Thread Safety
// Requirement: 06-REQ-2.2
// ---------------------------------------------------------------------------

func TestResponseStoreThreadSafety(t *testing.T) {
	s := NewStore()
	var wg sync.WaitGroup
	wg.Add(200)

	// 100 concurrent writers
	for i := 0; i < 100; i++ {
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
		go func(i int) {
			defer wg.Done()
			s.GetResponse(fmt.Sprintf("cmd-%d", i))
		}(i)
	}

	wg.Wait()

	// Verify all written responses are retrievable
	for i := 0; i < 100; i++ {
		resp, found := s.GetResponse(fmt.Sprintf("cmd-%d", i))
		if !found {
			t.Errorf("cmd-%d: expected response to be found", i)
			continue
		}
		if resp.Status != "success" {
			t.Errorf("cmd-%d: expected Status 'success', got %q", i, resp.Status)
		}
	}
}

// ---------------------------------------------------------------------------
// TS-06-P2: Response Store Consistency Property
// Property 2 from design.md
// Requirement: 06-REQ-2.1, 06-REQ-2.2
// ---------------------------------------------------------------------------

func TestPropertyResponseStoreConsistency(t *testing.T) {
	s := NewStore()

	testCases := []model.CommandResponse{
		{CommandID: "id-1", Status: "success"},
		{CommandID: "id-2", Status: "failed", Reason: "door jammed"},
		{CommandID: "id-3", Status: "timeout"},
		{CommandID: "id-4", Status: "success", Reason: ""},
		{CommandID: "id-5", Status: "failed", Reason: "battery low"},
	}

	for _, tc := range testCases {
		s.StoreResponse(tc)
		got, found := s.GetResponse(tc.CommandID)
		if !found {
			t.Errorf("CommandID %q: expected found=true", tc.CommandID)
			continue
		}
		if got.CommandID != tc.CommandID {
			t.Errorf("expected CommandID %q, got %q", tc.CommandID, got.CommandID)
		}
		if got.Status != tc.Status {
			t.Errorf("CommandID %q: expected Status %q, got %q",
				tc.CommandID, tc.Status, got.Status)
		}
		if got.Reason != tc.Reason {
			t.Errorf("CommandID %q: expected Reason %q, got %q",
				tc.CommandID, tc.Reason, got.Reason)
		}
	}
}

// ---------------------------------------------------------------------------
// TS-06-P3: Timeout Completeness Property
// Property 3 from design.md
// Requirement: 06-REQ-1.3
// ---------------------------------------------------------------------------

func TestPropertyTimeoutCompleteness(t *testing.T) {
	cmdIDs := []string{
		"timeout-001", "timeout-002", "timeout-003",
		"timeout-004", "timeout-005",
	}

	for _, cmdID := range cmdIDs {
		t.Run(cmdID, func(t *testing.T) {
			s := NewStore()
			timeout := 50 * time.Millisecond
			s.StartTimeout(cmdID, timeout)
			time.Sleep(timeout + 50*time.Millisecond)

			resp, found := s.GetResponse(cmdID)
			if !found {
				t.Errorf("CommandID %q: expected found=true after timeout", cmdID)
				return
			}
			if resp.Status != "timeout" {
				t.Errorf("CommandID %q: expected Status 'timeout', got %q",
					cmdID, resp.Status)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-06-P5: Timeout Cancellation Property
// Property 6 from design.md (TS-06-P5 per test_spec numbering)
// Requirement: 06-REQ-1.3, 06-REQ-2.2
// ---------------------------------------------------------------------------

func TestPropertyTimeoutCancellation(t *testing.T) {
	cmdIDs := []string{
		"cancel-001", "cancel-002", "cancel-003",
		"cancel-004", "cancel-005",
	}

	for _, cmdID := range cmdIDs {
		t.Run(cmdID, func(t *testing.T) {
			s := NewStore()
			s.StartTimeout(cmdID, 500*time.Millisecond)
			// Store a real response before timeout
			s.StoreResponse(model.CommandResponse{
				CommandID: cmdID,
				Status:    "success",
			})
			// Wait past the timeout duration
			time.Sleep(600 * time.Millisecond)

			resp, found := s.GetResponse(cmdID)
			if !found {
				t.Errorf("CommandID %q: expected found=true", cmdID)
				return
			}
			if resp.Status != "success" {
				t.Errorf("CommandID %q: expected Status 'success' (not 'timeout'), got %q",
					cmdID, resp.Status)
			}
		})
	}
}
