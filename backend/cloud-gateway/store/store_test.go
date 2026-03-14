package store_test

import (
	"math/rand"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// TS-06-4: Command Stored as Pending
func TestCommandStoredAsPending(t *testing.T) {
	s := store.NewStore()
	if s == nil {
		t.Fatal("NewStore returned nil")
	}
	s.Add(model.CommandStatus{CommandID: "cmd-001", Status: "pending", CreatedAt: time.Now()})
	cs, found := s.Get("cmd-001")
	if !found {
		t.Fatal("Get(cmd-001) returned found=false, want true")
	}
	if cs.Status != "pending" {
		t.Errorf("Status = %q, want %q", cs.Status, "pending")
	}
}

// TS-06-6: Pending Status Before Response
func TestPendingStatusBeforeResponse(t *testing.T) {
	s := store.NewStore()
	if s == nil {
		t.Fatal("NewStore returned nil")
	}
	s.Add(model.CommandStatus{CommandID: "cmd-002", Status: "pending", CreatedAt: time.Now()})
	cs, found := s.Get("cmd-002")
	if !found {
		t.Fatal("Get(cmd-002) returned found=false")
	}
	if cs.Status != "pending" {
		t.Errorf("Status = %q, want %q", cs.Status, "pending")
	}
}

// TS-06-7: Success and Failed Status
func TestSuccessAndFailedStatus(t *testing.T) {
	s := store.NewStore()
	if s == nil {
		t.Fatal("NewStore returned nil")
	}
	s.Add(model.CommandStatus{CommandID: "cmd-003", Status: "pending", CreatedAt: time.Now()})
	s.UpdateFromResponse(model.CommandResponse{
		CommandID: "cmd-003",
		Status:    "failed",
		Reason:    "vehicle_moving",
	})
	cs, found := s.Get("cmd-003")
	if !found {
		t.Fatal("Get(cmd-003) returned found=false")
	}
	if cs.Status != "failed" {
		t.Errorf("Status = %q, want %q", cs.Status, "failed")
	}
	if cs.Reason != "vehicle_moving" {
		t.Errorf("Reason = %q, want %q", cs.Reason, "vehicle_moving")
	}
}

// TS-06-11: Command Timeout
func TestCommandTimeout(t *testing.T) {
	s := store.NewStore()
	if s == nil {
		t.Fatal("NewStore returned nil")
	}
	s.Add(model.CommandStatus{
		CommandID: "cmd-007",
		Status:    "pending",
		CreatedAt: time.Now().Add(-2 * time.Second),
	})
	s.ExpireTimedOut(1 * time.Second)
	cs, found := s.Get("cmd-007")
	if !found {
		t.Fatal("Get(cmd-007) returned found=false")
	}
	if cs.Status != "timeout" {
		t.Errorf("Status = %q, want %q", cs.Status, "timeout")
	}
}

// TS-06-11: Non-expired command stays pending.
func TestCommandNotTimedOut(t *testing.T) {
	s := store.NewStore()
	if s == nil {
		t.Fatal("NewStore returned nil")
	}
	s.Add(model.CommandStatus{
		CommandID: "cmd-008",
		Status:    "pending",
		CreatedAt: time.Now(),
	})
	s.ExpireTimedOut(30 * time.Second)
	cs, found := s.Get("cmd-008")
	if !found {
		t.Fatal("Get(cmd-008) returned found=false")
	}
	if cs.Status != "pending" {
		t.Errorf("Status = %q, want %q", cs.Status, "pending")
	}
}

// Get for unknown command returns false.
func TestGetUnknownCommand(t *testing.T) {
	s := store.NewStore()
	if s == nil {
		t.Fatal("NewStore returned nil")
	}
	_, found := s.Get("nonexistent")
	if found {
		t.Error("Get(nonexistent) returned found=true, want false")
	}
}

// TS-06-P3: Response Status Update Property
func TestPropertyResponseStatusUpdate(t *testing.T) {
	statuses := []string{"success", "failed"}
	reasons := []string{"", "door_open", "vehicle_moving", "timeout_exceeded"}

	s := store.NewStore()
	if s == nil {
		t.Fatal("NewStore returned nil")
	}

	for i, status := range statuses {
		for j, reason := range reasons {
			cmdID := randomCmdID(i*10 + j)
			s.Add(model.CommandStatus{CommandID: cmdID, Status: "pending", CreatedAt: time.Now()})
			s.UpdateFromResponse(model.CommandResponse{
				CommandID: cmdID,
				Status:    status,
				Reason:    reason,
			})
			cs, found := s.Get(cmdID)
			if !found {
				t.Errorf("Get(%q) returned found=false", cmdID)
				continue
			}
			if cs.Status != status {
				t.Errorf("Get(%q).Status = %q, want %q", cmdID, cs.Status, status)
			}
			if cs.Reason != reason {
				t.Errorf("Get(%q).Reason = %q, want %q", cmdID, cs.Reason, reason)
			}
		}
	}
}

// TS-06-P4: Command Timeout Property
func TestPropertyCommandTimeout(t *testing.T) {
	type testCase struct {
		age     time.Duration
		timeout time.Duration
		wantExp bool
	}
	cases := []testCase{
		{5 * time.Second, 3 * time.Second, true},
		{10 * time.Second, 5 * time.Second, true},
		{1 * time.Second, 30 * time.Second, false},
		{0, 30 * time.Second, false},
		{29 * time.Second, 30 * time.Second, false},
		{31 * time.Second, 30 * time.Second, true},
	}

	for i, tc := range cases {
		s := store.NewStore()
		if s == nil {
			t.Fatal("NewStore returned nil")
		}
		cmdID := randomCmdID(100 + i)
		s.Add(model.CommandStatus{
			CommandID: cmdID,
			Status:    "pending",
			CreatedAt: time.Now().Add(-tc.age),
		})
		s.ExpireTimedOut(tc.timeout)
		cs, found := s.Get(cmdID)
		if !found {
			t.Errorf("case %d: Get(%q) returned found=false", i, cmdID)
			continue
		}
		if tc.wantExp && cs.Status != "timeout" {
			t.Errorf("case %d: age=%v timeout=%v: Status = %q, want %q",
				i, tc.age, tc.timeout, cs.Status, "timeout")
		}
		if !tc.wantExp && cs.Status != "pending" {
			t.Errorf("case %d: age=%v timeout=%v: Status = %q, want %q",
				i, tc.age, tc.timeout, cs.Status, "pending")
		}
	}
}

// randomCmdID generates a deterministic command ID for property tests.
func randomCmdID(seed int) string {
	r := rand.New(rand.NewSource(int64(seed)))
	const chars = "abcdefghijklmnopqrstuvwxyz0123456789"
	b := make([]byte, 8)
	for i := range b {
		b[i] = chars[r.Intn(len(chars))]
	}
	return "cmd-" + string(b)
}
