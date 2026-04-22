package lockingservice_test

import (
	"testing"
	"time"
)

// TestCommandSubscription verifies the service subscribes to
// Vehicle.Command.Door.Lock on startup and receives published commands.
// TS-03-1 | Requirement: 03-REQ-1.1, 03-REQ-1.2
func TestCommandSubscription(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	// Start the locking-service; it subscribes to Vehicle.Command.Door.Lock
	// and logs "locking-service ready" when initialised.
	_ = startLockingService(t, binary, "http://"+tcpTarget)

	// Publish a lock command to DATA_BROKER.
	cmdID := "sub-test-cmd-1"
	setString(t, client, signalCommand, makeLockJSON(cmdID))

	// A response should appear on Vehicle.Command.Door.Response within 5s.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)
	if resp == nil {
		t.Fatal("expected a response from locking-service, got nil")
	}
	// The response should have a non-empty status.
	if status, ok := resp["status"].(string); !ok || status == "" {
		t.Errorf("expected non-empty status in response, got %v", resp["status"])
	}
}

// TestInitialState verifies the service publishes IsLocked = false on startup.
// TS-03-13 | Requirement: 03-REQ-4.3
func TestInitialState(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	// Set IsLocked to true before starting the service so we can confirm the
	// service resets it to false on startup.
	setBool(t, client, signalIsLocked, true)

	// Start the locking-service.
	_ = startLockingService(t, binary, "http://"+tcpTarget)

	// After startup, IsLocked should be false.
	val := getBool(t, client, signalIsLocked)
	if val == nil {
		t.Fatal("expected IsLocked to have a value after service startup, got nil")
	}
	if *val != false {
		t.Errorf("expected IsLocked = false after startup, got %v", *val)
	}
}
