package state

import (
	"fmt"
	"sync"
	"testing"
)

func TestNewStoreIsEmpty(t *testing.T) {
	s := NewStore()
	if v := s.GetVehicle("VIN1"); v != nil {
		t.Errorf("expected nil for unregistered VIN, got %+v", v)
	}
}

func TestRegisterVehicle(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	v := s.GetVehicle("VIN1")
	if v == nil {
		t.Fatal("expected vehicle entry, got nil")
	}
	if v.VIN != "VIN1" {
		t.Errorf("VIN = %q, want %q", v.VIN, "VIN1")
	}
	if v.PairingPIN != "123456" {
		t.Errorf("PairingPIN = %q, want %q", v.PairingPIN, "123456")
	}
	if v.PairToken != "" {
		t.Errorf("PairToken should be empty before pairing, got %q", v.PairToken)
	}
}

func TestRegisterVehicleUpdatesPIN(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "111111")
	s.RegisterVehicle("VIN1", "222222")

	v := s.GetVehicle("VIN1")
	if v == nil {
		t.Fatal("expected vehicle entry, got nil")
	}
	if v.PairingPIN != "222222" {
		t.Errorf("PairingPIN = %q, want %q", v.PairingPIN, "222222")
	}
}

func TestGetVehicleReturnsCopy(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	v1 := s.GetVehicle("VIN1")
	v1.PairingPIN = "MODIFIED"

	v2 := s.GetVehicle("VIN1")
	if v2.PairingPIN == "MODIFIED" {
		t.Error("GetVehicle should return a copy, but internal state was modified")
	}
}

func TestGetVehicleNotFound(t *testing.T) {
	s := NewStore()
	if v := s.GetVehicle("NONEXISTENT"); v != nil {
		t.Errorf("expected nil for unregistered VIN, got %+v", v)
	}
}

func TestUpdateState(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	locked := true
	doorOpen := false
	speed := 42.5
	lat := 48.1351
	lon := 11.582
	parking := true

	s.UpdateState("VIN1", &locked, &doorOpen, &speed, &lat, &lon, &parking)

	v := s.GetVehicle("VIN1")
	if v == nil {
		t.Fatal("expected vehicle entry, got nil")
	}
	if v.IsLocked == nil || *v.IsLocked != true {
		t.Errorf("IsLocked = %v, want true", v.IsLocked)
	}
	if v.IsDoorOpen == nil || *v.IsDoorOpen != false {
		t.Errorf("IsDoorOpen = %v, want false", v.IsDoorOpen)
	}
	if v.Speed == nil || *v.Speed != 42.5 {
		t.Errorf("Speed = %v, want 42.5", v.Speed)
	}
	if v.Latitude == nil || *v.Latitude != 48.1351 {
		t.Errorf("Latitude = %v, want 48.1351", v.Latitude)
	}
	if v.Longitude == nil || *v.Longitude != 11.582 {
		t.Errorf("Longitude = %v, want 11.582", v.Longitude)
	}
	if v.ParkingSessionActive == nil || *v.ParkingSessionActive != true {
		t.Errorf("ParkingSessionActive = %v, want true", v.ParkingSessionActive)
	}
	if v.StateUpdatedAt.IsZero() {
		t.Error("StateUpdatedAt should be set after UpdateState")
	}
}

func TestUpdateStatePartial(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	locked := true
	s.UpdateState("VIN1", &locked, nil, nil, nil, nil, nil)

	v := s.GetVehicle("VIN1")
	if v.IsLocked == nil || *v.IsLocked != true {
		t.Errorf("IsLocked = %v, want true", v.IsLocked)
	}
	// Other fields should remain nil.
	if v.Speed != nil {
		t.Errorf("Speed should be nil, got %v", *v.Speed)
	}
}

func TestUpdateStateIgnoresUnknownVIN(t *testing.T) {
	s := NewStore()
	locked := true
	// Should not panic.
	s.UpdateState("NONEXISTENT", &locked, nil, nil, nil, nil, nil)
}

func TestAddCommand(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	cmd, err := s.AddCommand("VIN1", "cmd-1", "lock")
	if err != nil {
		t.Fatalf("AddCommand: %v", err)
	}
	if cmd.CommandID != "cmd-1" {
		t.Errorf("CommandID = %q, want %q", cmd.CommandID, "cmd-1")
	}
	if cmd.Type != "lock" {
		t.Errorf("Type = %q, want %q", cmd.Type, "lock")
	}
	if cmd.Status != "accepted" {
		t.Errorf("Status = %q, want %q", cmd.Status, "accepted")
	}

	// Verify command appears in vehicle entry.
	v := s.GetVehicle("VIN1")
	if _, ok := v.Commands["cmd-1"]; !ok {
		t.Error("command not found in vehicle entry")
	}
}

func TestAddCommandUnknownVIN(t *testing.T) {
	s := NewStore()
	_, err := s.AddCommand("NONEXISTENT", "cmd-1", "lock")
	if err == nil {
		t.Error("expected error for unknown VIN, got nil")
	}
}

func TestUpdateCommandResult(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")
	s.AddCommand("VIN1", "cmd-1", "lock")

	ok := s.UpdateCommandResult("VIN1", "cmd-1", "SUCCESS")
	if !ok {
		t.Fatal("UpdateCommandResult returned false")
	}

	v := s.GetVehicle("VIN1")
	cmd := v.Commands["cmd-1"]
	if cmd.Result != "SUCCESS" {
		t.Errorf("Result = %q, want %q", cmd.Result, "SUCCESS")
	}
	if cmd.Status != "success" {
		t.Errorf("Status = %q, want %q", cmd.Status, "success")
	}
}

func TestUpdateCommandResultRejected(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")
	s.AddCommand("VIN1", "cmd-1", "lock")

	ok := s.UpdateCommandResult("VIN1", "cmd-1", "REJECTED_SPEED")
	if !ok {
		t.Fatal("UpdateCommandResult returned false")
	}

	v := s.GetVehicle("VIN1")
	cmd := v.Commands["cmd-1"]
	if cmd.Result != "REJECTED_SPEED" {
		t.Errorf("Result = %q, want %q", cmd.Result, "REJECTED_SPEED")
	}
	if cmd.Status != "rejected" {
		t.Errorf("Status = %q, want %q", cmd.Status, "rejected")
	}
}

func TestUpdateCommandResultUnknownVIN(t *testing.T) {
	s := NewStore()
	ok := s.UpdateCommandResult("NONEXISTENT", "cmd-1", "SUCCESS")
	if ok {
		t.Error("expected false for unknown VIN")
	}
}

func TestUpdateCommandResultUnknownCommandID(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")
	ok := s.UpdateCommandResult("VIN1", "NONEXISTENT", "SUCCESS")
	if ok {
		t.Error("expected false for unknown command_id")
	}
}

func TestPairVehicleSuccess(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	token, err := s.PairVehicle("VIN1", "123456")
	if err != nil {
		t.Fatalf("PairVehicle: %v", err)
	}
	if token == "" {
		t.Error("expected non-empty token")
	}

	v := s.GetVehicle("VIN1")
	if v.PairToken != token {
		t.Errorf("PairToken = %q, want %q", v.PairToken, token)
	}
}

func TestPairVehicleUnknownVIN(t *testing.T) {
	s := NewStore()
	_, err := s.PairVehicle("NONEXISTENT", "123456")
	if err != ErrVehicleNotFound {
		t.Errorf("expected ErrVehicleNotFound, got %v", err)
	}
}

func TestPairVehicleWrongPIN(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	_, err := s.PairVehicle("VIN1", "999999")
	if err != ErrPINMismatch {
		t.Errorf("expected ErrPINMismatch, got %v", err)
	}
}

func TestPairVehicleRepairing(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	token1, _ := s.PairVehicle("VIN1", "123456")
	token2, _ := s.PairVehicle("VIN1", "123456")

	// Old token should be invalidated.
	if s.ValidateToken(token1, "VIN1") {
		t.Error("old token should be invalidated after re-pairing")
	}
	// New token should work.
	if !s.ValidateToken(token2, "VIN1") {
		t.Error("new token should be valid after re-pairing")
	}
}

func TestValidateToken(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")
	token, _ := s.PairVehicle("VIN1", "123456")

	// Valid token and VIN.
	if !s.ValidateToken(token, "VIN1") {
		t.Error("expected valid token")
	}

	// Valid token but wrong VIN.
	s.RegisterVehicle("VIN2", "654321")
	if s.ValidateToken(token, "VIN2") {
		t.Error("token should not be valid for a different VIN")
	}

	// Invalid token.
	if s.ValidateToken("bogus-token", "VIN1") {
		t.Error("bogus token should not be valid")
	}
}

func TestValidateTokenEmptyToken(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	if s.ValidateToken("", "VIN1") {
		t.Error("empty token should not be valid")
	}
}

func TestConcurrentAccess(t *testing.T) {
	s := NewStore()
	s.RegisterVehicle("VIN1", "123456")

	var wg sync.WaitGroup
	const goroutines = 50

	// Concurrent reads and writes.
	wg.Add(goroutines * 3)

	// Writers: update state.
	for i := 0; i < goroutines; i++ {
		go func() {
			defer wg.Done()
			speed := float64(i)
			s.UpdateState("VIN1", nil, nil, &speed, nil, nil, nil)
		}()
	}

	// Writers: add commands.
	for i := 0; i < goroutines; i++ {
		go func(idx int) {
			defer wg.Done()
			cmdID := fmt.Sprintf("cmd-%d", idx)
			s.AddCommand("VIN1", cmdID, "lock")
		}(i)
	}

	// Readers: get vehicle.
	for i := 0; i < goroutines; i++ {
		go func() {
			defer wg.Done()
			s.GetVehicle("VIN1")
		}()
	}

	wg.Wait()

	// If we got here without a race detector complaint, concurrent access
	// is safe.
	v := s.GetVehicle("VIN1")
	if v == nil {
		t.Fatal("expected vehicle entry after concurrent access")
	}
	if len(v.Commands) != goroutines {
		t.Errorf("expected %d commands, got %d", goroutines, len(v.Commands))
	}
}

func TestGenerateTokenUniqueness(t *testing.T) {
	tokens := make(map[string]bool)
	for i := 0; i < 100; i++ {
		tok, err := generateToken()
		if err != nil {
			t.Fatalf("generateToken: %v", err)
		}
		if tokens[tok] {
			t.Fatalf("duplicate token generated: %s", tok)
		}
		tokens[tok] = true
	}
}
