package lockingservice_test

import (
	"encoding/json"
	"testing"
	"time"
)

// TestSmokeLockHappyPath verifies the end-to-end lock flow: set speed=0 and
// door=closed, publish a lock command, verify IsLocked=true and a success
// response.
// TS-03-SMOKE-1 | Requirement: 03-REQ-3.3, 03-REQ-4.1, 03-REQ-5.1
func TestSmokeLockHappyPath(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	// Start the locking-service.
	_ = startLockingService(t, binary, "http://"+tcpTarget)

	// Set safety conditions: vehicle stationary, door closed.
	setFloat(t, client, signalSpeed, 0.0)
	setBool(t, client, signalDoorOpen, false)

	// Publish a lock command.
	cmdID := "smoke-lock-1"
	setString(t, client, signalCommand, makeLockJSON(cmdID))

	// Wait for the response.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)

	// Verify success response.
	if status, ok := resp["status"].(string); !ok || status != "success" {
		t.Errorf("expected status 'success', got %v", resp["status"])
	}

	// Verify IsLocked is now true.
	locked := getBool(t, client, signalIsLocked)
	if locked == nil {
		t.Fatal("expected IsLocked to have a value, got nil")
	}
	if !*locked {
		t.Error("expected IsLocked = true after lock command")
	}
}

// TestSmokeUnlockHappyPath verifies the end-to-end unlock flow: lock the door
// first, then publish an unlock command and verify IsLocked=false and a success
// response.
// TS-03-SMOKE-2 | Requirement: 03-REQ-3.4, 03-REQ-4.2, 03-REQ-5.1
func TestSmokeUnlockHappyPath(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	// Start the locking-service.
	_ = startLockingService(t, binary, "http://"+tcpTarget)

	// Set safety conditions: vehicle stationary, door closed.
	setFloat(t, client, signalSpeed, 0.0)
	setBool(t, client, signalDoorOpen, false)

	// First, lock the door.
	lockCmdID := "smoke-lock-2"
	setString(t, client, signalCommand, makeLockJSON(lockCmdID))
	waitForResponse(t, client, lockCmdID, 5*time.Second)

	// Now unlock the door.
	unlockCmdID := "smoke-unlock-2"
	setString(t, client, signalCommand, makeUnlockJSON(unlockCmdID))

	// Wait for the unlock response.
	resp := waitForResponse(t, client, unlockCmdID, 5*time.Second)

	// Verify success response.
	if status, ok := resp["status"].(string); !ok || status != "success" {
		t.Errorf("expected status 'success', got %v", resp["status"])
	}

	// Verify IsLocked is now false.
	locked := getBool(t, client, signalIsLocked)
	if locked == nil {
		t.Fatal("expected IsLocked to have a value, got nil")
	}
	if *locked {
		t.Error("expected IsLocked = false after unlock command")
	}
}

// TestSmokeLockRejectedMoving verifies the end-to-end lock rejection when the
// vehicle is moving: set speed=50, publish a lock command, verify IsLocked
// remains false and a failure response with reason "vehicle_moving".
// TS-03-SMOKE-3 | Requirement: 03-REQ-3.1, 03-REQ-5.2
func TestSmokeLockRejectedMoving(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	// Start the locking-service.
	_ = startLockingService(t, binary, "http://"+tcpTarget)

	// Set speed to 50 km/h (above safety threshold).
	setFloat(t, client, signalSpeed, 50.0)
	setBool(t, client, signalDoorOpen, false)

	// Publish a lock command.
	cmdID := "smoke-lock-rejected-1"
	setString(t, client, signalCommand, makeLockJSON(cmdID))

	// Wait for the response.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)

	// Verify failure response.
	if status, ok := resp["status"].(string); !ok || status != "failed" {
		t.Errorf("expected status 'failed', got %v", resp["status"])
	}
	if reason, ok := resp["reason"].(string); !ok || reason != "vehicle_moving" {
		t.Errorf("expected reason 'vehicle_moving', got %v", resp["reason"])
	}

	// Verify IsLocked is still false.
	locked := getBool(t, client, signalIsLocked)
	if locked == nil {
		t.Fatal("expected IsLocked to have a value, got nil")
	}
	if *locked {
		t.Error("expected IsLocked = false after rejected lock command")
	}
}

// TestSmokeInvalidCommandResponse verifies the end-to-end handling of an
// invalid command (unsupported door): the service publishes a failure response
// with reason "unsupported_door".
// Requirement: 03-REQ-2.2, 03-REQ-5.2
func TestSmokeInvalidCommandResponse(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	// Start the locking-service.
	_ = startLockingService(t, binary, "http://"+tcpTarget)

	// Publish a command with an unsupported door value.
	cmdID := "smoke-invalid-door-1"
	setString(t, client, signalCommand, makeInvalidDoorJSON(cmdID))

	// Wait for the response.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)

	// Verify failure response.
	if status, ok := resp["status"].(string); !ok || status != "failed" {
		t.Errorf("expected status 'failed', got %v", resp["status"])
	}
	if reason, ok := resp["reason"].(string); !ok || reason != "unsupported_door" {
		t.Errorf("expected reason 'unsupported_door', got %v", resp["reason"])
	}

	// Verify IsLocked is still false (no state change).
	locked := getBool(t, client, signalIsLocked)
	if locked == nil {
		t.Fatal("expected IsLocked to have a value, got nil")
	}
	if *locked {
		t.Error("expected IsLocked = false after invalid command")
	}
}

// TestSmokeInvalidJsonDiscarded verifies that an invalid JSON payload is
// discarded without publishing a response.
// Requirement: 03-REQ-2.E1
func TestSmokeInvalidJsonDiscarded(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	// Start the locking-service.
	_ = startLockingService(t, binary, "http://"+tcpTarget)

	// Send a valid lock command first to establish a known response baseline.
	setFloat(t, client, signalSpeed, 0.0)
	setBool(t, client, signalDoorOpen, false)
	baselineCmdID := "smoke-baseline-1"
	setString(t, client, signalCommand, makeLockJSON(baselineCmdID))
	waitForResponse(t, client, baselineCmdID, 5*time.Second)

	// Record the current response value.
	baselineResp := getString(t, client, signalResponse)
	if baselineResp == nil {
		t.Fatal("expected baseline response to be set")
	}

	// Now send invalid JSON.
	setString(t, client, signalCommand, "not valid json {{{")

	// Wait 2 seconds to allow the service to process (and discard) the payload.
	time.Sleep(2 * time.Second)

	// Verify the response signal has NOT changed from the baseline.
	currentResp := getString(t, client, signalResponse)
	if currentResp == nil {
		t.Fatal("expected response signal to still contain baseline value")
	}

	// Parse both to compare command_ids — the response should still be from
	// the baseline command, not from the invalid JSON.
	var baselineParsed, currentParsed map[string]any
	if err := json.Unmarshal([]byte(*baselineResp), &baselineParsed); err != nil {
		t.Fatalf("failed to parse baseline response: %v", err)
	}
	if err := json.Unmarshal([]byte(*currentResp), &currentParsed); err != nil {
		t.Fatalf("failed to parse current response: %v", err)
	}

	if baselineParsed["command_id"] != currentParsed["command_id"] {
		t.Errorf("response changed after invalid JSON: baseline command_id=%v, current command_id=%v",
			baselineParsed["command_id"], currentParsed["command_id"])
	}
}
