// Integration tests for LOCKING_SERVICE (spec 03_locking_service).
//
// Most tests require:
//   - grpcurl installed
//   - DATA_BROKER reachable on localhost:55556
//   - locking-service binary (built via cargo)
//
// TestConnectionRetryFailure (TS-03-E1) requires only cargo and works without
// any running infrastructure.
package lockingservice

import (
	"strings"
	"testing"
	"time"
)

// ── TS-03-1: Command Subscription ────────────────────────────────────────────

// TestCommandSubscription verifies that the service subscribes to
// Vehicle.Command.Door.Lock and processes commands from DATA_BROKER.
// A response must appear on Vehicle.Command.Door.Response within 5s.
//
// Requirements: 03-REQ-1.1, 03-REQ-1.2, 03-REQ-6.2
// Test Spec: TS-03-1
func TestCommandSubscription(t *testing.T) {
	requireGrpcurl(t)
	lsp := startLockingService(t) // skips if not ready (proto gap)

	// Set safe conditions first.
	brokerPublishFloat(t, signalSpeed, 0.0)
	brokerPublishBool(t, signalIsOpen, false)

	cmdID := "ts-03-1-sub"
	brokerPublishString(t, signalCommand, lockCmdJSON(cmdID))

	if !waitForBrokerStringContains(t, signalResponse, cmdID, 5*time.Second) {
		t.Errorf("no response appeared on %s within 5s after publishing command; logs:\n%s",
			signalResponse, lsp.logs())
	}
}

// ── TS-03-13: Initial State Published as False ────────────────────────────────

// TestInitialStateFalse verifies that the service publishes IsLocked = false
// during startup before processing any commands.
//
// Requirements: 03-REQ-4.3
// Test Spec: TS-03-13
func TestInitialStateFalse(t *testing.T) {
	requireGrpcurl(t)
	_ = startLockingService(t) // skips if not ready (proto gap)

	// After "locking-service ready" the initial state must already be published.
	isLocked := brokerGetBool(t, signalIsLocked)
	if isLocked {
		t.Errorf("expected IsLocked = false after startup, got true")
	}
}

// ── TS-03-SMOKE-1: Lock Happy Path ───────────────────────────────────────────

// TestSmokeLockHappyPath is an end-to-end smoke test for a successful lock:
// speed = 0.0, door closed → lock command → IsLocked = true, status "success".
//
// Requirements: 03-REQ-3.3, 03-REQ-4.1, 03-REQ-5.1
// Test Spec: TS-03-SMOKE-1
func TestSmokeLockHappyPath(t *testing.T) {
	requireGrpcurl(t)
	_ = startLockingService(t) // skips if not ready (proto gap)

	// Set safety conditions: stationary, door closed.
	brokerPublishFloat(t, signalSpeed, 0.0)
	brokerPublishBool(t, signalIsOpen, false)

	// Ensure starting state is unlocked.
	brokerPublishBool(t, signalIsLocked, false)

	cmdID := "ts-03-smoke-1-lock"
	brokerPublishString(t, signalCommand, lockCmdJSON(cmdID))

	// Wait for the response.
	if !waitForBrokerStringContains(t, signalResponse, `"success"`, 5*time.Second) {
		t.Fatalf("expected success response within 5s; last response: %q",
			brokerGetString(t, signalResponse))
	}

	// Verify IsLocked was set to true.
	if !waitForBrokerBool(t, signalIsLocked, true, 3*time.Second) {
		t.Errorf("expected IsLocked = true after successful lock command")
	}

	// Verify response fields.
	resp := brokerGetString(t, signalResponse)
	if !strings.Contains(resp, cmdID) {
		t.Errorf("expected command_id %q in response, got: %q", cmdID, resp)
	}
	if strings.Contains(resp, `"reason"`) {
		t.Errorf("success response must NOT include reason field; got: %q", resp)
	}
}

// ── TS-03-SMOKE-2: Unlock Happy Path ─────────────────────────────────────────

// TestSmokeUnlockHappyPath is an end-to-end smoke test for a successful unlock:
// set IsLocked = true → unlock command → IsLocked = false, status "success".
//
// Requirements: 03-REQ-3.4, 03-REQ-4.2, 03-REQ-5.1
// Test Spec: TS-03-SMOKE-2
func TestSmokeUnlockHappyPath(t *testing.T) {
	requireGrpcurl(t)
	_ = startLockingService(t) // skips if not ready (proto gap)

	// Start in a locked state.
	brokerPublishBool(t, signalIsLocked, true)

	cmdID := "ts-03-smoke-2-unlock"
	brokerPublishString(t, signalCommand, unlockCmdJSON(cmdID))

	// Wait for the response.
	if !waitForBrokerStringContains(t, signalResponse, `"success"`, 5*time.Second) {
		t.Fatalf("expected success response within 5s; last response: %q",
			brokerGetString(t, signalResponse))
	}

	// Verify IsLocked is now false.
	if !waitForBrokerBool(t, signalIsLocked, false, 3*time.Second) {
		t.Errorf("expected IsLocked = false after unlock command")
	}

	// Verify the response echoes the command_id.
	resp := brokerGetString(t, signalResponse)
	if !strings.Contains(resp, cmdID) {
		t.Errorf("expected command_id %q in response, got: %q", cmdID, resp)
	}
}

// ── TS-03-SMOKE-3: Lock Rejected — Vehicle Moving ────────────────────────────

// TestSmokeLockRejectedMoving verifies that a lock command is rejected with
// reason "vehicle_moving" when the vehicle speed is >= 1.0 km/h, and that
// IsLocked remains false.
//
// Requirements: 03-REQ-3.1, 03-REQ-5.2
// Test Spec: TS-03-SMOKE-3
func TestSmokeLockRejectedMoving(t *testing.T) {
	requireGrpcurl(t)
	_ = startLockingService(t) // skips if not ready (proto gap)

	// Ensure unlocked state.
	brokerPublishBool(t, signalIsLocked, false)

	// Set vehicle as moving.
	brokerPublishFloat(t, signalSpeed, 50.0)

	cmdID := "ts-03-smoke-3-moving"
	brokerPublishString(t, signalCommand, lockCmdJSON(cmdID))

	// Wait for a "failed" response.
	if !waitForBrokerStringContains(t, signalResponse, `"failed"`, 5*time.Second) {
		t.Fatalf("expected failed response within 5s; last response: %q",
			brokerGetString(t, signalResponse))
	}

	// Verify rejection reason and IsLocked remains false.
	resp := brokerGetString(t, signalResponse)
	if !strings.Contains(resp, "vehicle_moving") {
		t.Errorf("expected reason vehicle_moving in response, got: %q", resp)
	}
	if brokerGetBool(t, signalIsLocked) {
		t.Errorf("IsLocked must remain false when lock is rejected")
	}
}

// ── TS-03-E1: Connection Retry Failure ───────────────────────────────────────

// TestConnectionRetryFailure verifies that the service exits with a non-zero
// code when DATA_BROKER is not reachable after exhausting retries.
//
// This test does NOT require any infrastructure — it intentionally points the
// service at a port with nothing listening.
//
// Requirements: 03-REQ-1.E1
// Test Spec: TS-03-E1
func TestConnectionRetryFailure(t *testing.T) {
	// Deliberately bad endpoint — nothing listens on this port.
	badAddr := "http://localhost:19999"

	lsp := startLockingServiceRaw(t, map[string]string{
		"DATABROKER_ADDR": badAddr,
		"RUST_LOG":        "warn",
	})

	// The service should try 5 times (with delays of 1+2+4+8 = 15s minimum)
	// and then exit non-zero. Give it up to 60s.
	exitCode, ok := lsp.waitForExit(60 * time.Second)
	if !ok {
		lsp.kill()
		t.Fatalf("locking-service did not exit within 60s despite unreachable DATA_BROKER; logs:\n%s",
			lsp.logs())
	}
	if exitCode == 0 {
		t.Errorf("expected non-zero exit code when DATA_BROKER is unreachable, got 0; logs:\n%s",
			lsp.logs())
	}
}

// ── TS-03-6: Graceful Shutdown (SIGTERM) ─────────────────────────────────────

// TestGracefulShutdown verifies that SIGTERM causes the service to complete
// its current work and exit with code 0.
//
// Requirements: 03-REQ-6.1, 03-REQ-6.E1
func TestGracefulShutdown(t *testing.T) {
	requireGrpcurl(t)
	lsp := startLockingService(t) // skips if not ready (proto gap)

	lsp.sendSIGTERM()

	code, ok := lsp.waitForExit(15 * time.Second)
	if !ok {
		lsp.kill()
		t.Fatalf("locking-service did not exit within 15s after SIGTERM; logs:\n%s", lsp.logs())
	}
	if code != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d; logs:\n%s", code, lsp.logs())
	}
}

// ── TS-03-6.2: Startup Logging ────────────────────────────────────────────────

// TestStartupLogging verifies that the service logs its version and
// DATABROKER_ADDR at startup.
//
// Requirements: 03-REQ-6.2
// Test Spec: TS-03-1 (startup log check)
func TestStartupLogging(t *testing.T) {
	requireGrpcurl(t)
	lsp := startLockingService(t) // skips if not ready (proto gap)

	logs := lsp.logs()
	if !strings.Contains(logs, "locking-service") {
		t.Errorf("expected service name in startup logs; logs:\n%s", logs)
	}
	if !strings.Contains(logs, tcpEndpoint) && !strings.Contains(logs, "55556") {
		t.Errorf("expected DATA_BROKER address in startup logs; logs:\n%s", logs)
	}
}
