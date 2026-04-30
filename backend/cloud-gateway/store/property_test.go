package store_test

import (
	"fmt"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// TS-06-P2: Response Store Consistency
// Property 2 from design.md
// Validates: 06-REQ-2.1, 06-REQ-2.2
// For any stored response, GetResponse returns the same data.
func TestPropertyResponseStoreConsistency(t *testing.T) {
	s := store.NewStore()

	testCases := []model.CommandResponse{
		{CommandID: "cmd-p2-1", Status: "success", Reason: ""},
		{CommandID: "cmd-p2-2", Status: "failed", Reason: "door jammed"},
		{CommandID: "cmd-p2-3", Status: "timeout", Reason: ""},
		{CommandID: "cmd-p2-4", Status: "success", Reason: "completed quickly"},
		{CommandID: "cmd-p2-5", Status: "failed", Reason: "connection lost"},
	}

	for _, resp := range testCases {
		s.StoreResponse(resp)
	}

	for _, expected := range testCases {
		t.Run(expected.CommandID, func(t *testing.T) {
			got, found := s.GetResponse(expected.CommandID)
			if !found {
				t.Fatalf("response not found for %q", expected.CommandID)
			}
			if got.CommandID != expected.CommandID {
				t.Errorf("CommandID = %q, want %q", got.CommandID, expected.CommandID)
			}
			if got.Status != expected.Status {
				t.Errorf("Status = %q, want %q", got.Status, expected.Status)
			}
			if got.Reason != expected.Reason {
				t.Errorf("Reason = %q, want %q", got.Reason, expected.Reason)
			}
		})
	}
}

// TS-06-P3: Timeout Completeness
// Property 3 from design.md
// Validates: 06-REQ-1.3
// For any command with no response, after the timeout the status is "timeout".
func TestPropertyTimeoutCompleteness(t *testing.T) {
	cmdIDs := []string{
		"timeout-p3-1",
		"timeout-p3-2",
		"timeout-p3-3",
		"timeout-p3-4",
		"timeout-p3-5",
	}

	for _, cmdID := range cmdIDs {
		t.Run(cmdID, func(t *testing.T) {
			s := store.NewStore()
			timeout := 50 * time.Millisecond
			s.StartTimeout(cmdID, timeout)
			time.Sleep(timeout + 50*time.Millisecond)

			resp, found := s.GetResponse(cmdID)
			if !found {
				t.Fatalf("response not found after timeout for %q", cmdID)
			}
			if resp.Status != "timeout" {
				t.Errorf("Status = %q, want %q", resp.Status, "timeout")
			}
		})
	}
}

// TS-06-P5: Timeout Cancellation
// Property 6 from design.md
// Validates: 06-REQ-1.3, 06-REQ-2.2
// For any command that receives a response before timeout, the status is not "timeout".
func TestPropertyTimeoutCancellation(t *testing.T) {
	cmdIDs := []string{
		"cancel-p5-1",
		"cancel-p5-2",
		"cancel-p5-3",
		"cancel-p5-4",
		"cancel-p5-5",
	}

	for _, cmdID := range cmdIDs {
		t.Run(cmdID, func(t *testing.T) {
			s := store.NewStore()
			s.StartTimeout(cmdID, 500*time.Millisecond)

			// Store a real response before timeout fires
			s.StoreResponse(model.CommandResponse{
				CommandID: cmdID,
				Status:    "success",
			})

			// Wait past the timeout period
			time.Sleep(600 * time.Millisecond)

			resp, found := s.GetResponse(cmdID)
			if !found {
				t.Fatalf("response not found for %q", cmdID)
			}
			if resp.Status != "success" {
				t.Errorf("Status = %q, want %q (NOT timeout)", resp.Status, "success")
			}
		})
	}

	// Also test with different status values
	statuses := []string{"success", "failed"}
	for i, status := range statuses {
		t.Run(fmt.Sprintf("status_%s", status), func(t *testing.T) {
			s := store.NewStore()
			cmdID := fmt.Sprintf("cancel-status-%d", i)
			s.StartTimeout(cmdID, 500*time.Millisecond)
			s.StoreResponse(model.CommandResponse{
				CommandID: cmdID,
				Status:    status,
			})
			time.Sleep(600 * time.Millisecond)

			resp, found := s.GetResponse(cmdID)
			if !found {
				t.Fatalf("response not found for %q", cmdID)
			}
			if resp.Status == "timeout" {
				t.Errorf("expected status %q, got timeout", status)
			}
		})
	}
}
