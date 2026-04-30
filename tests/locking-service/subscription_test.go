package lockingsvc_test

import (
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-03-1: Command Subscription on Startup
// Requirement: 03-REQ-1.1, 03-REQ-1.2
// Description: Verify the service subscribes to Vehicle.Command.Door.Lock
// on startup and receives published commands.
// ---------------------------------------------------------------------------

func TestCommandSubscription(t *testing.T) {
	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)
	_, client := dialDatabroker(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Publish a lock command to Vehicle.Command.Door.Lock.
	cmdID := "sub-test-001"
	publishValue(t, client, signalCommand, stringValue(lockCommandJSON(cmdID)))

	// A response should appear on Vehicle.Command.Door.Response within 5 seconds.
	resp := waitForNewResponse(t, client, cmdID, 5*time.Second)
	if resp == nil {
		t.Fatal("expected a response on Vehicle.Command.Door.Response")
	}

	_ = proc // kept alive by t.Cleanup
}

// ---------------------------------------------------------------------------
// TS-03-13: Initial State Published on Startup
// Requirement: 03-REQ-4.3
// Description: Verify the service publishes IsLocked = false on startup.
// ---------------------------------------------------------------------------

func TestInitialState(t *testing.T) {
	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)
	_, client := dialDatabroker(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Read the IsLocked signal from DATA_BROKER.
	dp := getValueOrFail(t, client, signalIsLocked)
	if dp == nil || dp.GetValue() == nil {
		t.Fatal("expected IsLocked signal to have a value after startup")
	}

	if dp.GetValue().GetBool() != false {
		t.Errorf("expected IsLocked=false after startup, got %v", dp.GetValue().GetBool())
	}

	_ = proc
}
