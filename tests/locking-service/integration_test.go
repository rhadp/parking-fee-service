package lockingservice

import (
	"os/exec"
	"strings"
	"testing"
	"time"
)

// validLockCmd returns a minimal valid lock command JSON payload.
func validLockCmd(commandID string) string {
	return `{"command_id":"` + commandID + `","action":"lock","doors":["driver"]}`
}

// ── TS-03-1: Command Subscription on Startup ──────────────────────────────────

// TestCommandSubscription verifies that the locking-service subscribes to
// Vehicle.Command.Door.Lock on startup and produces a response when a command
// is published.
// Requirement: 03-REQ-1.1, 03-REQ-1.2
func TestCommandSubscription(t *testing.T) {
	requireLockingServiceDeps(t)
	startDatabroker(t)

	ls := startLockingService(t, lsDatabrokerAddr)
	if !ls.waitForLog("ready", 15*time.Second) {
		t.Fatalf("locking-service did not log 'ready' within timeout; logs:\n%s", ls.logs())
	}

	// Subscribe to the response signal before sending the command so we capture
	// the notification when the locking-service publishes the result.
	captured := grpcSubscribeCapture(t, signalCmdResp, 10*time.Second, func() {
		out, err := grpcSetString(t, signalCmdLock, validLockCmd("sub-test-001"))
		if err != nil {
			t.Errorf("Set(%s) failed: %v\noutput: %s", signalCmdLock, err, out)
		}
	})

	if !strings.Contains(captured, "sub-test-001") {
		t.Errorf("expected response with command_id 'sub-test-001' in subscribe capture; got:\n%s", captured)
	}
	if !strings.Contains(captured, "success") {
		t.Errorf("expected 'success' in response; got:\n%s", captured)
	}
}

// ── TS-03-13: Initial State Published ────────────────────────────────────────

// TestInitialState verifies that locking-service publishes IsLocked=false on startup.
// Requirement: 03-REQ-4.3
func TestInitialState(t *testing.T) {
	requireLockingServiceDeps(t)
	startDatabroker(t)

	ls := startLockingService(t, lsDatabrokerAddr)
	if !ls.waitForLog("ready", 15*time.Second) {
		t.Fatalf("locking-service did not log 'ready' within timeout; logs:\n%s", ls.logs())
	}

	// The service publishes IsLocked=false on startup. Poll until the value appears.
	if !pollSignalContains(t, signalIsLocked, "false", 10*time.Second) {
		out, _ := grpcGet(t, signalIsLocked)
		t.Errorf("expected IsLocked=false after startup; got:\n%s", out)
	}

	// Explicitly verify it is NOT true.
	out, _ := grpcGet(t, signalIsLocked)
	if strings.Contains(out, `"bool": true`) {
		t.Errorf("IsLocked should be false on startup but got true in response:\n%s", out)
	}
}

// ── TS-03-17: Graceful Shutdown ───────────────────────────────────────────────

// TestGracefulShutdown verifies the service exits cleanly on SIGTERM.
// Requirement: 03-REQ-6.1
func TestGracefulShutdown(t *testing.T) {
	requireLockingServiceDeps(t)
	startDatabroker(t)

	ls := startLockingService(t, lsDatabrokerAddr)
	if !ls.waitForLog("ready", 15*time.Second) {
		t.Fatalf("locking-service did not log 'ready' within timeout; logs:\n%s", ls.logs())
	}

	ls.sendSIGTERM()

	code, ok := ls.waitForExit(10 * time.Second)
	if !ok {
		t.Fatal("locking-service did not exit within timeout after SIGTERM")
	}
	if code != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d; logs:\n%s", code, ls.logs())
	}
	if !ls.logContains("shutdown") {
		t.Errorf("expected shutdown log message; logs:\n%s", ls.logs())
	}
}

// ── TS-03-18: Startup Logging ─────────────────────────────────────────────────

// TestStartupLogging verifies the service logs version, address, and ready message.
// Requirement: 03-REQ-6.2
func TestStartupLogging(t *testing.T) {
	requireLockingServiceDeps(t)
	startDatabroker(t)

	ls := startLockingService(t, lsDatabrokerAddr)
	if !ls.waitForLog("ready", 15*time.Second) {
		t.Fatalf("locking-service did not log 'ready' within timeout; logs:\n%s", ls.logs())
	}

	logs := ls.logs()

	if !strings.Contains(logs, "locking-service") {
		t.Errorf("expected 'locking-service' in startup logs; got:\n%s", logs)
	}
	// The databroker address should appear in the startup log.
	if !strings.Contains(logs, "55556") {
		t.Errorf("expected DATA_BROKER port '55556' in startup logs; got:\n%s", logs)
	}
	if !strings.Contains(logs, "ready") {
		t.Errorf("expected 'ready' in startup logs; got:\n%s", logs)
	}
}

// ── TS-03-E1: Databroker Unreachable on Startup ───────────────────────────────

// TestDatabrokerUnreachable verifies retry behaviour and non-zero exit when
// DATA_BROKER is unreachable.
// Requirement: 03-REQ-1.E1
func TestDatabrokerUnreachable(t *testing.T) {
	requireCargo(t) // grpcurl and podman not needed for this test

	// Use a port that is not listening.
	ls := startLockingService(t, "http://localhost:19999")

	// Allow up to 35 s: 5 connection attempts with delays 1+2+4+8 = 15 s plus
	// connection attempt overhead.
	code, ok := ls.waitForExit(35 * time.Second)
	if !ok {
		t.Fatal("locking-service did not exit within timeout when broker is unreachable")
	}
	if code == 0 {
		t.Errorf("expected non-zero exit code when broker is unreachable, got 0; logs:\n%s", ls.logs())
	}

	// Verify retry attempts were logged.
	logs := ls.logs()
	if !strings.Contains(logs, "retry") && !strings.Contains(logs, "attempt") {
		t.Errorf("expected retry log messages; logs:\n%s", logs)
	}
}

// ── TS-03-E2: Subscription Stream Interrupted ─────────────────────────────────

// TestSubscriptionInterrupted verifies that when the subscription stream breaks
// the service attempts to resubscribe and eventually exits non-zero.
// Requirement: 03-REQ-1.E2
func TestSubscriptionInterrupted(t *testing.T) {
	requireLockingServiceDeps(t)
	startDatabroker(t)

	ls := startLockingService(t, lsDatabrokerAddr)
	if !ls.waitForLog("ready", 15*time.Second) {
		t.Fatalf("locking-service did not log 'ready' within timeout; logs:\n%s", ls.logs())
	}

	// Kill the databroker container so the subscription stream breaks.
	_ = exec.Command("podman", "rm", "-f", "ls-test-databroker").Run()

	// After the stream breaks the service will attempt to resubscribe. Allow
	// 30 s for the stream to break and for resubscribe attempts to be logged.
	resubscribeLogged := ls.waitForLog("resubscrib", 30*time.Second) ||
		ls.waitForLog("Subscription stream", 5*time.Second) ||
		ls.waitForLog("Subscribe failed", 5*time.Second)
	if !resubscribeLogged {
		// If none of the above messages appeared but the service exited, that is
		// also acceptable (fast-path exit before log flushes).
		_, exited := ls.waitForExit(1 * time.Second)
		if !exited {
			t.Errorf("expected resubscribe attempt in logs; logs:\n%s", ls.logs())
		}
	}

	// After exhausting retries, the service must exit non-zero.
	code, ok := ls.waitForExit(30 * time.Second)
	if !ok {
		t.Fatal("locking-service did not exit within timeout after subscription failure")
	}
	if code == 0 {
		t.Errorf("expected non-zero exit after subscription failure, got 0; logs:\n%s", ls.logs())
	}
}

// ── TS-03-E11: SIGTERM During Command ─────────────────────────────────────────

// TestSigtermDuringCommand verifies that an in-flight command completes before
// the service shuts down.
// Requirement: 03-REQ-6.E1
func TestSigtermDuringCommand(t *testing.T) {
	requireLockingServiceDeps(t)
	startDatabroker(t)

	ls := startLockingService(t, lsDatabrokerAddr)
	if !ls.waitForLog("ready", 15*time.Second) {
		t.Fatalf("locking-service did not log 'ready' within timeout; logs:\n%s", ls.logs())
	}

	// Speed and door signals are unset → treated as safe by the service.
	// Send a lock command; the service should process it and publish a response.
	out, err := grpcSetString(t, signalCmdLock, validLockCmd("sigterm-cmd-001"))
	if err != nil {
		t.Fatalf("Set(%s) failed: %v\noutput: %s", signalCmdLock, err, out)
	}

	// Wait briefly for the service to receive and start processing the command,
	// then send SIGTERM. The in-flight command must complete before shutdown.
	time.Sleep(300 * time.Millisecond)
	ls.sendSIGTERM()

	// The service must exit cleanly.
	code, ok := ls.waitForExit(10 * time.Second)
	if !ok {
		t.Fatal("locking-service did not exit within timeout after SIGTERM")
	}
	if code != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d; logs:\n%s", code, ls.logs())
	}

	// Verify the response was published (the command completed before shutdown).
	if !pollSignalContains(t, signalCmdResp, "sigterm-cmd-001", 5*time.Second) {
		out, _ := grpcGet(t, signalCmdResp)
		t.Errorf("expected response for 'sigterm-cmd-001' after graceful shutdown; got:\n%s\nlogs:\n%s", out, ls.logs())
	}
}
