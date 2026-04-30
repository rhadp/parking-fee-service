package lockingsvc_test

import (
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-03-SMOKE-1: Lock Happy Path
// Description: End-to-end lock: set speed=0 and door=closed, publish lock
// command, verify IsLocked=true and success response.
// ---------------------------------------------------------------------------

func TestSmokeLockHappyPath(t *testing.T) {
	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)
	_, client := dialDatabroker(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Set safety conditions: vehicle stationary, door closed.
	publishValue(t, client, signalSpeed, floatValue(0.0))
	publishValue(t, client, signalDoorOpen, boolValue(false))

	// Publish a lock command.
	cmdID := "smoke-lock-001"
	publishValue(t, client, signalCommand, stringValue(lockCommandJSON(cmdID)))

	// Wait for response.
	resp := waitForNewResponse(t, client, cmdID, 5*time.Second)

	// Verify success response.
	if status, _ := resp["status"].(string); status != "success" {
		t.Errorf("expected status=success, got %q", status)
	}

	// Verify IsLocked is now true.
	dp := getValueOrFail(t, client, signalIsLocked)
	if dp == nil || dp.GetValue() == nil {
		t.Fatal("expected IsLocked signal to have a value")
	}
	if dp.GetValue().GetBool() != true {
		t.Errorf("expected IsLocked=true after lock, got %v", dp.GetValue().GetBool())
	}

	_ = proc
}

// ---------------------------------------------------------------------------
// TS-03-SMOKE-2: Unlock Happy Path
// Description: End-to-end unlock after lock: verify IsLocked=false and
// success response.
// ---------------------------------------------------------------------------

func TestSmokeUnlockHappyPath(t *testing.T) {
	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)
	_, client := dialDatabroker(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Set safety conditions: vehicle stationary, door closed.
	publishValue(t, client, signalSpeed, floatValue(0.0))
	publishValue(t, client, signalDoorOpen, boolValue(false))

	// First lock the door.
	lockCmdID := "smoke-unlock-lock-001"
	publishValue(t, client, signalCommand, stringValue(lockCommandJSON(lockCmdID)))
	waitForNewResponse(t, client, lockCmdID, 5*time.Second)

	// Now unlock.
	unlockCmdID := "smoke-unlock-002"
	publishValue(t, client, signalCommand, stringValue(unlockCommandJSON(unlockCmdID)))

	// Wait for unlock response.
	resp := waitForNewResponse(t, client, unlockCmdID, 5*time.Second)

	// Verify success response.
	if status, _ := resp["status"].(string); status != "success" {
		t.Errorf("expected status=success, got %q", status)
	}

	// Verify IsLocked is now false.
	dp := getValueOrFail(t, client, signalIsLocked)
	if dp == nil || dp.GetValue() == nil {
		t.Fatal("expected IsLocked signal to have a value")
	}
	if dp.GetValue().GetBool() != false {
		t.Errorf("expected IsLocked=false after unlock, got %v", dp.GetValue().GetBool())
	}

	_ = proc
}

// ---------------------------------------------------------------------------
// TS-03-SMOKE-3: Lock Rejected (Vehicle Moving)
// Description: End-to-end lock rejection due to vehicle speed.
// ---------------------------------------------------------------------------

func TestSmokeLockRejectedMoving(t *testing.T) {
	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)
	_, client := dialDatabroker(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Set speed to 50.0 km/h (above the 1.0 km/h threshold).
	publishValue(t, client, signalSpeed, floatValue(50.0))
	publishValue(t, client, signalDoorOpen, boolValue(false))

	// Publish a lock command.
	cmdID := "smoke-moving-001"
	publishValue(t, client, signalCommand, stringValue(lockCommandJSON(cmdID)))

	// Wait for response.
	resp := waitForNewResponse(t, client, cmdID, 5*time.Second)

	// Verify failure response with reason "vehicle_moving".
	if status, _ := resp["status"].(string); status != "failed" {
		t.Errorf("expected status=failed, got %q", status)
	}
	if reason, _ := resp["reason"].(string); reason != "vehicle_moving" {
		t.Errorf("expected reason=vehicle_moving, got %q", reason)
	}

	// Verify IsLocked remains false.
	dp := getValueOrFail(t, client, signalIsLocked)
	if dp != nil && dp.GetValue() != nil && dp.GetValue().GetBool() != false {
		t.Errorf("expected IsLocked=false when vehicle moving, got %v", dp.GetValue().GetBool())
	}

	_ = proc
}

// ---------------------------------------------------------------------------
// TestSmokeInvalidCommandResponse: Unsupported door value
// Requirement: 03-REQ-2.2
// Description: Verify that a command with an unsupported door value returns
// a failure response with reason "unsupported_door".
// ---------------------------------------------------------------------------

func TestSmokeInvalidCommandResponse(t *testing.T) {
	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)
	_, client := dialDatabroker(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Publish a lock command with unsupported door "passenger".
	cmdID := "smoke-invalid-door-001"
	cmdJSON := `{"command_id":"` + cmdID + `","action":"lock","doors":["passenger"]}`
	publishValue(t, client, signalCommand, stringValue(cmdJSON))

	// Wait for response.
	resp := waitForNewResponse(t, client, cmdID, 5*time.Second)

	// Verify failure response with reason "unsupported_door".
	if status, _ := resp["status"].(string); status != "failed" {
		t.Errorf("expected status=failed, got %q", status)
	}
	if reason, _ := resp["reason"].(string); reason != "unsupported_door" {
		t.Errorf("expected reason=unsupported_door, got %q", reason)
	}

	_ = proc
}

// ---------------------------------------------------------------------------
// TestSmokeInvalidJsonDiscarded: Invalid JSON discarded without response
// Requirement: 03-REQ-2.E1
// Description: Verify that an invalid JSON payload is discarded without
// publishing a response. A subsequent valid command still gets a response.
// ---------------------------------------------------------------------------

func TestSmokeInvalidJsonDiscarded(t *testing.T) {
	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)
	_, client := dialDatabroker(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Set safety conditions.
	publishValue(t, client, signalSpeed, floatValue(0.0))
	publishValue(t, client, signalDoorOpen, boolValue(false))

	// Publish invalid JSON.
	publishValue(t, client, signalCommand, stringValue("not valid json {{{"))

	// Wait a moment for the service to process (and discard) the invalid JSON.
	time.Sleep(1 * time.Second)

	// Now publish a valid command to verify the service is still processing.
	cmdID := "smoke-after-invalid-001"
	publishValue(t, client, signalCommand, stringValue(lockCommandJSON(cmdID)))

	// The valid command should get a response.
	resp := waitForNewResponse(t, client, cmdID, 5*time.Second)
	if status, _ := resp["status"].(string); status != "success" {
		t.Errorf("expected status=success for valid command after invalid JSON, got %q", status)
	}

	_ = proc
}
