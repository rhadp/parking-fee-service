package lockingservice_test

import (
	"testing"
	"time"
)

// TestSmokeLockHappyPath is an end-to-end lock test: set speed=0 and
// door=closed, publish lock command, verify IsLocked=true and success response.
//
// Test Spec: TS-03-SMOKE-1
// Requirements: 03-REQ-3.3, 03-REQ-4.1, 03-REQ-5.1
func TestSmokeLockHappyPath(t *testing.T) {
	skipIfTCPUnreachable(t)

	bin := buildLockingService(t)
	client := newClient(t)

	// Set safe preconditions.
	setSignalFloat(t, client, signalSpeed, 0.0)
	setSignalBool(t, client, signalIsOpen, false)

	_ = startLockingService(t, bin, "http://"+tcpTarget)

	// Send a lock command.
	cmdID := "smoke-lock-001"
	setSignalString(t, client, signalCommand, makeLockCommandJSON(cmdID))

	// Wait for response.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)
	if resp["status"] != "success" {
		t.Errorf("expected status=success, got %v", resp["status"])
	}

	// Verify IsLocked is true.
	val, ok := getBoolValue(t, client, signalIsLocked)
	if !ok {
		t.Fatal("IsLocked signal has no value after lock command")
	}
	if !val {
		t.Error("expected IsLocked=true after lock command")
	}
}

// TestSmokeUnlockHappyPath is an end-to-end unlock test after lock: verify
// IsLocked=false and success response.
//
// Test Spec: TS-03-SMOKE-2
// Requirements: 03-REQ-3.4, 03-REQ-4.2, 03-REQ-5.1
func TestSmokeUnlockHappyPath(t *testing.T) {
	skipIfTCPUnreachable(t)

	bin := buildLockingService(t)
	client := newClient(t)

	// Set safe preconditions.
	setSignalFloat(t, client, signalSpeed, 0.0)
	setSignalBool(t, client, signalIsOpen, false)

	_ = startLockingService(t, bin, "http://"+tcpTarget)

	// First lock, then unlock.
	lockID := "smoke-unlock-lock-001"
	setSignalString(t, client, signalCommand, makeLockCommandJSON(lockID))
	waitForResponse(t, client, lockID, 5*time.Second)

	// Now unlock.
	unlockID := "smoke-unlock-001"
	setSignalString(t, client, signalCommand, makeUnlockCommandJSON(unlockID))
	resp := waitForResponse(t, client, unlockID, 5*time.Second)

	if resp["status"] != "success" {
		t.Errorf("expected status=success, got %v", resp["status"])
	}

	// Verify IsLocked is false.
	val, ok := getBoolValue(t, client, signalIsLocked)
	if !ok {
		t.Fatal("IsLocked signal has no value after unlock command")
	}
	if val {
		t.Error("expected IsLocked=false after unlock command")
	}
}

// TestSmokeLockRejectedMoving is an end-to-end lock rejection test due to
// vehicle speed.
//
// Test Spec: TS-03-SMOKE-3
// Requirements: 03-REQ-3.1, 03-REQ-5.2
func TestSmokeLockRejectedMoving(t *testing.T) {
	skipIfTCPUnreachable(t)

	bin := buildLockingService(t)
	client := newClient(t)

	// Set speed > 1.0 to trigger rejection.
	setSignalFloat(t, client, signalSpeed, 50.0)
	setSignalBool(t, client, signalIsOpen, false)

	_ = startLockingService(t, bin, "http://"+tcpTarget)

	// Send a lock command.
	cmdID := "smoke-moving-001"
	setSignalString(t, client, signalCommand, makeLockCommandJSON(cmdID))

	// Wait for response.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)
	if resp["status"] != "failed" {
		t.Errorf("expected status=failed, got %v", resp["status"])
	}
	if resp["reason"] != "vehicle_moving" {
		t.Errorf("expected reason=vehicle_moving, got %v", resp["reason"])
	}

	// Verify IsLocked remains false.
	val, ok := getBoolValue(t, client, signalIsLocked)
	if !ok {
		// Signal might not be set if it was reset by startup; that's also
		// acceptable as the default is false.
		return
	}
	if val {
		t.Error("expected IsLocked=false when lock rejected, got true")
	}
}

// TestSmokeInvalidCommandResponse verifies that an unsupported door value
// results in a failure response with reason "unsupported_door".
//
// Requirements: 03-REQ-2.2, 03-REQ-5.2
func TestSmokeInvalidCommandResponse(t *testing.T) {
	skipIfTCPUnreachable(t)

	bin := buildLockingService(t)
	client := newClient(t)

	setSignalFloat(t, client, signalSpeed, 0.0)
	setSignalBool(t, client, signalIsOpen, false)

	_ = startLockingService(t, bin, "http://"+tcpTarget)

	// Send a command with an unsupported door.
	cmdID := "smoke-unsupported-001"
	cmdJSON := `{"command_id":"` + cmdID + `","action":"lock","doors":["passenger"]}`
	setSignalString(t, client, signalCommand, cmdJSON)

	resp := waitForResponse(t, client, cmdID, 5*time.Second)
	if resp["status"] != "failed" {
		t.Errorf("expected status=failed, got %v", resp["status"])
	}
	if resp["reason"] != "unsupported_door" {
		t.Errorf("expected reason=unsupported_door, got %v", resp["reason"])
	}
}

// TestSmokeInvalidJsonDiscarded verifies that invalid JSON payloads are
// discarded without publishing a response.
//
// Requirements: 03-REQ-2.E1
func TestSmokeInvalidJsonDiscarded(t *testing.T) {
	skipIfTCPUnreachable(t)

	bin := buildLockingService(t)
	client := newClient(t)

	setSignalFloat(t, client, signalSpeed, 0.0)
	setSignalBool(t, client, signalIsOpen, false)

	_ = startLockingService(t, bin, "http://"+tcpTarget)

	// Clear any prior response.
	setSignalString(t, client, signalResponse, "")

	// Send invalid JSON.
	setSignalString(t, client, signalCommand, "not valid json {{{")

	// Wait briefly - no response should appear.
	time.Sleep(2 * time.Second)

	raw := getStringValue(t, client, signalResponse)
	if raw != "" {
		t.Errorf("expected no response for invalid JSON, but got: %s", raw)
	}

	// Verify the service is still alive by sending a valid command.
	cmdID := "smoke-after-invalid-001"
	setSignalString(t, client, signalCommand, makeLockCommandJSON(cmdID))
	resp := waitForResponse(t, client, cmdID, 5*time.Second)
	if resp["status"] != "success" {
		t.Errorf("expected service to continue processing after invalid JSON, got status=%v", resp["status"])
	}
}
