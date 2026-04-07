package lockingservice_test

import (
	"fmt"
	"os"
	"os/exec"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-03-1: Command Subscription on Startup
// Requirement: 03-REQ-1.1, 03-REQ-1.2
// ---------------------------------------------------------------------------

// TestCommandSubscription verifies the service subscribes to
// Vehicle.Command.Door.Lock on startup and processes received commands.
func TestCommandSubscription(t *testing.T) {
	requirePodman(t)
	client := ensureDatabrokerReachable(t)
	resetSignals(t, client)

	binary := buildLockingService(t)
	_ = startLockingService(t, binary, fmt.Sprintf("http://%s", tcpAddr))

	// Give the service a moment to stabilize after becoming ready.
	time.Sleep(500 * time.Millisecond)

	// Publish a lock command via gRPC.
	cmdID := "ts03-1-sub-test"
	cmdJSON := makeLockJSON(cmdID)
	setSignalString(t, client, signalCommand, cmdJSON)

	// Expect a response within 5 seconds.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)
	if resp.CommandID != cmdID {
		t.Errorf("TS-03-1: expected command_id=%q, got %q", cmdID, resp.CommandID)
	}
	// The command should succeed since speed=0 and door=closed.
	if resp.Status != "success" {
		t.Errorf("TS-03-1: expected status=success, got %q (reason=%q)", resp.Status, resp.Reason)
	}
}

// ---------------------------------------------------------------------------
// TS-03-13: Initial State Published on Startup
// Requirement: 03-REQ-4.3
// ---------------------------------------------------------------------------

// TestInitialStateFalse verifies the service publishes IsLocked = false
// on startup.
func TestInitialStateFalse(t *testing.T) {
	requirePodman(t)
	client := ensureDatabrokerReachable(t)

	// Set IsLocked to true before starting the service, so we can detect
	// the service resetting it to false.
	setSignalBool(t, client, signalIsLocked, true)

	binary := buildLockingService(t)
	_ = startLockingService(t, binary, fmt.Sprintf("http://%s", tcpAddr))

	// After startup, IsLocked should be false.
	time.Sleep(500 * time.Millisecond)

	val, ok := getSignalBool(t, client, signalIsLocked)
	if !ok {
		t.Fatal("TS-03-13: IsLocked signal has no value after startup")
	}
	if val != false {
		t.Errorf("TS-03-13: expected IsLocked=false after startup, got true")
	}
}

// ---------------------------------------------------------------------------
// TS-03-SMOKE-1: Lock Happy Path
// Requirement: 03-REQ-3.3, 03-REQ-4.1, 03-REQ-5.1
// ---------------------------------------------------------------------------

// TestSmokeLockHappyPath verifies end-to-end lock: set speed=0 and door=closed,
// publish lock command, verify IsLocked=true and success response.
func TestSmokeLockHappyPath(t *testing.T) {
	requirePodman(t)
	client := ensureDatabrokerReachable(t)
	resetSignals(t, client)

	binary := buildLockingService(t)
	_ = startLockingService(t, binary, fmt.Sprintf("http://%s", tcpAddr))
	time.Sleep(500 * time.Millisecond)

	// Set safe conditions.
	setSignalFloat(t, client, signalSpeed, 0.0)
	setSignalBool(t, client, signalIsOpen, false)

	// Publish lock command.
	cmdID := "ts03-smoke1-lock"
	setSignalString(t, client, signalCommand, makeLockJSON(cmdID))

	// Wait for response.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)
	if resp.Status != "success" {
		t.Errorf("TS-03-SMOKE-1: expected status=success, got %q (reason=%q)", resp.Status, resp.Reason)
	}

	// Verify IsLocked is true.
	val, ok := getSignalBool(t, client, signalIsLocked)
	if !ok {
		t.Fatal("TS-03-SMOKE-1: IsLocked has no value after lock")
	}
	if !val {
		t.Error("TS-03-SMOKE-1: expected IsLocked=true after lock, got false")
	}
}

// ---------------------------------------------------------------------------
// TS-03-SMOKE-2: Unlock Happy Path
// Requirement: 03-REQ-3.4, 03-REQ-4.2, 03-REQ-5.1
// ---------------------------------------------------------------------------

// TestSmokeUnlockHappyPath verifies end-to-end unlock after lock: verify
// IsLocked=false and success response.
func TestSmokeUnlockHappyPath(t *testing.T) {
	requirePodman(t)
	client := ensureDatabrokerReachable(t)
	resetSignals(t, client)

	binary := buildLockingService(t)
	_ = startLockingService(t, binary, fmt.Sprintf("http://%s", tcpAddr))
	time.Sleep(500 * time.Millisecond)

	// First, lock the door.
	setSignalFloat(t, client, signalSpeed, 0.0)
	setSignalBool(t, client, signalIsOpen, false)
	lockID := "ts03-smoke2-lock"
	setSignalString(t, client, signalCommand, makeLockJSON(lockID))
	resp := waitForResponse(t, client, lockID, 5*time.Second)
	if resp.Status != "success" {
		t.Fatalf("TS-03-SMOKE-2: lock setup failed: status=%q reason=%q", resp.Status, resp.Reason)
	}

	// Now unlock.
	unlockID := "ts03-smoke2-unlock"
	setSignalString(t, client, signalCommand, makeUnlockJSON(unlockID))
	resp = waitForResponse(t, client, unlockID, 5*time.Second)
	if resp.Status != "success" {
		t.Errorf("TS-03-SMOKE-2: expected status=success for unlock, got %q (reason=%q)", resp.Status, resp.Reason)
	}

	// Verify IsLocked is false.
	val, ok := getSignalBool(t, client, signalIsLocked)
	if !ok {
		t.Fatal("TS-03-SMOKE-2: IsLocked has no value after unlock")
	}
	if val {
		t.Error("TS-03-SMOKE-2: expected IsLocked=false after unlock, got true")
	}
}

// ---------------------------------------------------------------------------
// TS-03-SMOKE-3: Lock Rejected (Vehicle Moving)
// Requirement: 03-REQ-3.1, 03-REQ-5.2
// ---------------------------------------------------------------------------

// TestSmokeLockRejectedMoving verifies end-to-end lock rejection due to
// vehicle speed.
func TestSmokeLockRejectedMoving(t *testing.T) {
	requirePodman(t)
	client := ensureDatabrokerReachable(t)
	resetSignals(t, client)

	binary := buildLockingService(t)
	_ = startLockingService(t, binary, fmt.Sprintf("http://%s", tcpAddr))
	time.Sleep(500 * time.Millisecond)

	// Set vehicle moving.
	setSignalFloat(t, client, signalSpeed, 50.0)
	setSignalBool(t, client, signalIsOpen, false)

	// Publish lock command.
	cmdID := "ts03-smoke3-moving"
	setSignalString(t, client, signalCommand, makeLockJSON(cmdID))

	// Wait for response.
	resp := waitForResponse(t, client, cmdID, 5*time.Second)
	if resp.Status != "failed" {
		t.Errorf("TS-03-SMOKE-3: expected status=failed, got %q", resp.Status)
	}
	if resp.Reason != "vehicle_moving" {
		t.Errorf("TS-03-SMOKE-3: expected reason=vehicle_moving, got %q", resp.Reason)
	}

	// Verify IsLocked remains false.
	val, ok := getSignalBool(t, client, signalIsLocked)
	if !ok {
		t.Fatal("TS-03-SMOKE-3: IsLocked has no value")
	}
	if val {
		t.Error("TS-03-SMOKE-3: expected IsLocked=false when vehicle moving, got true")
	}
}

// ---------------------------------------------------------------------------
// TS-03-E1: DATA_BROKER Connection Retry
// Requirement: 03-REQ-1.E1
// ---------------------------------------------------------------------------

// TestConnectionRetryFailure verifies the service retries connection to
// DATA_BROKER with exponential backoff and exits with non-zero code on failure.
func TestConnectionRetryFailure(t *testing.T) {
	binary := buildLockingService(t)

	// Point to a non-existent endpoint.
	cmd := exec.Command(binary, "serve")
	cmd.Env = append(os.Environ(), "DATABROKER_ADDR=http://localhost:99999")
	cmd.Stdout = nil
	cmd.Stderr = nil

	err := cmd.Start()
	if err != nil {
		t.Fatalf("TS-03-E1: failed to start locking-service: %v", err)
	}

	// Wait for the process to exit (it should retry and then fail).
	// The exponential backoff is 1+2+4+8 = 15 seconds max.
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err == nil {
			t.Error("TS-03-E1: expected non-zero exit code, got 0")
		}
		// Non-zero exit is expected.
		if exitErr, ok := err.(*exec.ExitError); ok {
			if exitErr.ExitCode() == 0 {
				t.Error("TS-03-E1: expected non-zero exit code, got 0")
			}
			t.Logf("TS-03-E1: service exited with code %d (expected non-zero)", exitErr.ExitCode())
		}
	case <-time.After(60 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("TS-03-E1: locking-service did not exit within 60 seconds")
	}
}
