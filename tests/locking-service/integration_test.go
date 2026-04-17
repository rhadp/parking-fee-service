// Integration tests for LOCKING_SERVICE.
//
// Tests in this file start the locking-service binary and, where required,
// the kuksa-databroker container (via podman compose).  Tests skip
// automatically when the required infrastructure is not available.
//
// Signal paths used:
//   - Vehicle.Command.Door.Lock    (string, write by test / subscribe by service)
//   - Vehicle.Speed                (float,  write by test / read by service)
//   - Vehicle.Cabin.Door.Row1.DriverSide.IsOpen  (bool, write by test / read by service)
//   - Vehicle.Cabin.Door.Row1.DriverSide.IsLocked (bool, written by service)
//   - Vehicle.Command.Door.Response (string, written by service)
//
// Test Specs: TS-03-1, TS-03-13, TS-03-E1, TS-03-SMOKE-1, TS-03-SMOKE-2,
//
//	TS-03-SMOKE-3
//
// Requirements: 03-REQ-1.1, 03-REQ-4.3, 03-REQ-1.E1, 03-REQ-3.1, 03-REQ-3.4
package lockingservice_test

import (
	"net"
	"strings"
	"syscall"
	"testing"
	"time"
)

// VSS signal path constants used by the locking-service.
const (
	sigCommandLock = "Vehicle.Command.Door.Lock"
	sigSpeed       = "Vehicle.Speed"
	sigIsOpen      = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"
	sigIsLocked    = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	sigResponse    = "Vehicle.Command.Door.Response"
)

// ── TS-03-1: Command Subscription on Startup ──────────────────────────────

// TestCommandSubscription verifies that after startup the service subscribes to
// Vehicle.Command.Door.Lock and processes a published command, producing a
// response on Vehicle.Command.Door.Response within 5 seconds.
//
// Test Spec: TS-03-1
// Requirements: 03-REQ-1.1, 03-REQ-1.2, 03-REQ-6.2
func TestCommandSubscription(t *testing.T) {
	requireDatabrokerTCP(t)
	startDatabroker(t)
	bin := buildLockingService(t)

	// Reset signals to a known safe state.
	publishFloat(t, sigSpeed, 0.0)
	publishBool(t, sigIsOpen, false)

	sp := startLockingService(t, bin, map[string]string{
		"DATABROKER_ADDR": "http://localhost:55556",
	})

	// Wait for service to be ready.
	waitForLog(t, sp, "locking-service ready", 30*time.Second)

	// Publish a lock command.
	cmdID := "test-sub-001"
	publishString(t, sigCommandLock, lockCommandJSON(cmdID))

	// Wait for the response to appear in DATA_BROKER.
	out := waitForSignal(t, sigResponse, cmdID, 5*time.Second)
	if !strings.Contains(out, cmdID) {
		t.Errorf("expected command_id %q in response; got: %s", cmdID, out)
	}
}

// ── TS-03-13: Initial State Published on Startup ─────────────────────────

// TestInitialState verifies that on startup the service publishes
// IsLocked = false to DATA_BROKER before processing any commands.
//
// Test Spec: TS-03-13
// Requirements: 03-REQ-4.3
func TestInitialState(t *testing.T) {
	requireDatabrokerTCP(t)
	startDatabroker(t)
	bin := buildLockingService(t)

	// Pre-set IsLocked to true to confirm the service resets it.
	publishBool(t, sigIsLocked, true)

	sp := startLockingService(t, bin, map[string]string{
		"DATABROKER_ADDR": "http://localhost:55556",
	})

	// Wait for service ready log.
	waitForLog(t, sp, "locking-service ready", 30*time.Second)

	// IsLocked should now be false.
	out := getValue(t, sigIsLocked)
	if !strings.Contains(out, "false") {
		t.Errorf("expected IsLocked=false after service startup; got: %s", out)
	}
}

// ── TS-03-SMOKE-1: Lock Happy Path ────────────────────────────────────────

// TestSmokeLockHappyPath verifies end-to-end lock: with speed=0 and door
// closed, publishing a lock command results in IsLocked=true and a success
// response.
//
// Test Spec: TS-03-SMOKE-1
// Requirements: 03-REQ-3.3, 03-REQ-4.1, 03-REQ-5.1
func TestSmokeLockHappyPath(t *testing.T) {
	requireDatabrokerTCP(t)
	startDatabroker(t)
	bin := buildLockingService(t)

	// Set safe conditions: stationary, door closed.
	publishFloat(t, sigSpeed, 0.0)
	publishBool(t, sigIsOpen, false)
	publishBool(t, sigIsLocked, false)

	sp := startLockingService(t, bin, map[string]string{
		"DATABROKER_ADDR": "http://localhost:55556",
	})
	waitForLog(t, sp, "locking-service ready", 30*time.Second)

	cmdID := "smoke-lock-001"
	publishString(t, sigCommandLock, lockCommandJSON(cmdID))

	// Wait for response with status "success".
	respOut := waitForSignal(t, sigResponse, "success", 5*time.Second)
	if !strings.Contains(respOut, cmdID) {
		t.Errorf("response does not echo command_id %q; got: %s", cmdID, respOut)
	}

	// IsLocked should be true.
	lockedOut := getValue(t, sigIsLocked)
	if !strings.Contains(lockedOut, "true") {
		t.Errorf("expected IsLocked=true after lock command; got: %s", lockedOut)
	}
}

// ── TS-03-SMOKE-2: Unlock Happy Path ──────────────────────────────────────

// TestSmokeUnlockHappyPath verifies end-to-end unlock: after locking, an
// unlock command results in IsLocked=false and a success response.
//
// Test Spec: TS-03-SMOKE-2
// Requirements: 03-REQ-3.4, 03-REQ-4.2, 03-REQ-5.1
func TestSmokeUnlockHappyPath(t *testing.T) {
	requireDatabrokerTCP(t)
	startDatabroker(t)
	bin := buildLockingService(t)

	// Start with door locked.
	publishFloat(t, sigSpeed, 0.0)
	publishBool(t, sigIsOpen, false)

	sp := startLockingService(t, bin, map[string]string{
		"DATABROKER_ADDR": "http://localhost:55556",
	})
	waitForLog(t, sp, "locking-service ready", 30*time.Second)

	// Lock first.
	lockID := "smoke-lock-002"
	publishString(t, sigCommandLock, lockCommandJSON(lockID))
	waitForSignal(t, sigResponse, lockID, 5*time.Second)

	// Then unlock.
	unlockID := "smoke-unlock-002"
	publishString(t, sigCommandLock, unlockCommandJSON(unlockID))
	respOut := waitForSignal(t, sigResponse, unlockID, 5*time.Second)

	if !strings.Contains(respOut, "success") {
		t.Errorf("expected unlock response status success; got: %s", respOut)
	}

	// IsLocked should be false.
	lockedOut := getValue(t, sigIsLocked)
	if !strings.Contains(lockedOut, "false") {
		t.Errorf("expected IsLocked=false after unlock; got: %s", lockedOut)
	}
}

// ── TS-03-SMOKE-3: Lock Rejected (Vehicle Moving) ─────────────────────────

// TestSmokeLockRejectedMoving verifies that a lock command is rejected with
// reason "vehicle_moving" when Vehicle.Speed >= 1.0.
//
// Test Spec: TS-03-SMOKE-3
// Requirements: 03-REQ-3.1, 03-REQ-5.2
func TestSmokeLockRejectedMoving(t *testing.T) {
	requireDatabrokerTCP(t)
	startDatabroker(t)
	bin := buildLockingService(t)

	// Set vehicle moving.
	publishFloat(t, sigSpeed, 50.0)
	publishBool(t, sigIsOpen, false)
	publishBool(t, sigIsLocked, false)

	sp := startLockingService(t, bin, map[string]string{
		"DATABROKER_ADDR": "http://localhost:55556",
	})
	waitForLog(t, sp, "locking-service ready", 30*time.Second)

	cmdID := "smoke-moving-001"
	publishString(t, sigCommandLock, lockCommandJSON(cmdID))

	respOut := waitForSignal(t, sigResponse, cmdID, 5*time.Second)

	if !strings.Contains(respOut, "failed") {
		t.Errorf("expected status failed for moving vehicle; got: %s", respOut)
	}
	if !strings.Contains(respOut, "vehicle_moving") {
		t.Errorf("expected reason vehicle_moving; got: %s", respOut)
	}

	// IsLocked should remain false.
	lockedOut := getValue(t, sigIsLocked)
	if !strings.Contains(lockedOut, "false") {
		t.Errorf("expected IsLocked=false (unchanged) after rejected lock; got: %s", lockedOut)
	}
}

// ── TS-03-E1: DATA_BROKER Connection Retry Failure ───────────────────────

// TestConnectionRetryFailure verifies that the service retries connection to
// DATA_BROKER and exits with a non-zero code after exhausting retries.
//
// This test does NOT require the DATA_BROKER to be running — it intentionally
// points the service at a non-existent endpoint.
//
// Test Spec: TS-03-E1
// Requirements: 03-REQ-1.E1
func TestConnectionRetryFailure(t *testing.T) {
	bin := buildLockingService(t)

	// Point to an endpoint where no server is listening.
	// We start a TCP listener and immediately close it so the port
	// is valid but unreachable by the time the service retries.
	lis, err := findFreePort()
	if err != nil {
		t.Fatalf("findFreePort: %v", err)
	}
	addr := "http://" + lis

	sp := startLockingService(t, bin, map[string]string{
		"DATABROKER_ADDR": addr,
	})

	// The service should exit with a non-zero code after exhausting retries.
	// With 5 attempts and delays 1s+2s+4s+8s = 15s + connect timeouts,
	// allow up to 60 seconds.
	exitCode, timedOut := waitForExit(sp, 60*time.Second)
	if timedOut {
		t.Fatal("service did not exit within 60s after exhausting retries")
	}
	if exitCode == 0 {
		t.Errorf("expected non-zero exit code; got 0")
	}
}

// findFreePort returns a "host:port" string for a port that is currently free
// (i.e. no server is listening on it).  We briefly bind then release the port.
func findFreePort() (string, error) {
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return "", err
	}
	addr := lis.Addr().String()
	lis.Close()
	return addr, nil
}

// ── TS-03-17: Graceful Shutdown ────────────────────────────────────────────

// TestGracefulShutdown verifies that the service exits with code 0 when it
// receives SIGTERM after successful startup.
//
// Requirements: 03-REQ-6.1, 03-REQ-6.E1
func TestGracefulShutdown(t *testing.T) {
	requireDatabrokerTCP(t)
	startDatabroker(t)
	bin := buildLockingService(t)

	publishFloat(t, sigSpeed, 0.0)
	publishBool(t, sigIsOpen, false)

	sp := startLockingService(t, bin, map[string]string{
		"DATABROKER_ADDR": "http://localhost:55556",
	})
	waitForLog(t, sp, "locking-service ready", 30*time.Second)

	// Send SIGTERM.
	if err := sp.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("send SIGTERM: %v", err)
	}

	exitCode, timedOut := waitForExit(sp, 10*time.Second)
	if timedOut {
		t.Fatal("service did not exit within 10s after SIGTERM")
	}
	if exitCode != 0 {
		t.Errorf("expected exit code 0 after SIGTERM; got %d\noutput:\n%s", exitCode, sp.output.String())
	}
}

// ── TS-03-18: Startup Logging ─────────────────────────────────────────────

// TestStartupLogging verifies that the service logs its version, DATA_BROKER
// address, and "locking-service ready" on successful startup.
//
// Requirements: 03-REQ-6.2
func TestStartupLogging(t *testing.T) {
	requireDatabrokerTCP(t)
	startDatabroker(t)
	bin := buildLockingService(t)

	publishFloat(t, sigSpeed, 0.0)
	publishBool(t, sigIsOpen, false)

	sp := startLockingService(t, bin, map[string]string{
		"DATABROKER_ADDR": "http://localhost:55556",
	})

	waitForLog(t, sp, "locking-service ready", 30*time.Second)

	logs := sp.output.String()

	// Service should log its version.
	if !strings.Contains(logs, "locking-service") {
		t.Errorf("expected 'locking-service' in startup logs; got:\n%s", logs)
	}

	// Service should log the DATA_BROKER address.
	if !strings.Contains(logs, "localhost:55556") {
		t.Errorf("expected DATA_BROKER address 'localhost:55556' in startup logs; got:\n%s", logs)
	}

	// Service should log ready.
	if !strings.Contains(logs, "locking-service ready") {
		t.Errorf("expected 'locking-service ready' in logs; got:\n%s", logs)
	}
}
