package lockingservice_test

import (
	"testing"
	"time"
)

// TestCommandSubscription verifies the service subscribes to
// Vehicle.Command.Door.Lock on startup and receives published commands.
//
// Test Spec: TS-03-1
// Requirements: 03-REQ-1.1, 03-REQ-1.2
func TestCommandSubscription(t *testing.T) {
	skipIfTCPUnreachable(t)

	bin := buildLockingService(t)
	client := newClient(t)

	// Set safe preconditions so the command can succeed.
	setSignalFloat(t, client, signalSpeed, 0.0)
	setSignalBool(t, client, signalIsOpen, false)

	svc := startLockingService(t, bin, "http://"+tcpTarget)

	// The startup log should be present (03-REQ-6.2).
	if !svc.logs.contains("locking-service starting") {
		t.Error("expected startup log containing 'locking-service starting'")
	}

	// Publish a lock command via gRPC.
	cmdID := "sub-test-001"
	setSignalString(t, client, signalCommand, makeLockCommandJSON(cmdID))

	// A response should appear on Vehicle.Command.Door.Response within 5 seconds.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)
	if resp == nil {
		t.Fatal("expected a response from the locking-service")
	}
}

// TestInitialState verifies the service publishes IsLocked = false on startup.
//
// Test Spec: TS-03-13
// Requirements: 03-REQ-4.3
func TestInitialState(t *testing.T) {
	skipIfTCPUnreachable(t)

	bin := buildLockingService(t)
	client := newClient(t)

	// First set IsLocked to true so we can verify the service resets it.
	setSignalBool(t, client, signalIsLocked, true)

	_ = startLockingService(t, bin, "http://"+tcpTarget)

	// Wait briefly for the initial state to be published.
	time.Sleep(500 * time.Millisecond)

	val, ok := getBoolValue(t, client, signalIsLocked)
	if !ok {
		t.Fatal("IsLocked signal has no value after service startup")
	}
	if val != false {
		t.Errorf("expected IsLocked=false after startup, got %v", val)
	}
}
