package lockingservice_test

import (
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
