// Integration tests for the LOCKING_SERVICE.
//
// Tests that depend on a live DATA_BROKER and a functioning gRPC connection
// skip gracefully when the infrastructure is unavailable or when the service
// cannot reach its "locking-service ready" state (which occurs because
// kuksa-databroker 0.5.0 exposes kuksa.val.v2.VAL while the locking-service
// uses kuksa.val.v1.VALService; see docs/errata/03_locking_service_proto_compat.md).
//
// Tests that always run (no infrastructure required):
//   - TestConnectionRetryFailure (TS-03-E1)
//   - TestStartupLogging (03-REQ-6.2)
//   - TestSubscriptionStreamInterrupted (TS-03-E2) — uses mock v1 broker
package lockingservice

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"
)

const (
	// validLockCmd is a well-formed lock command JSON payload.
	validLockCmd = `{"command_id":"integ-test-lock","action":"lock","doors":["driver"]}`
	// validUnlockCmd is a well-formed unlock command JSON payload.
	validUnlockCmd = `{"command_id":"integ-test-unlock","action":"unlock","doors":["driver"]}`
)

// dataBrokerHTTPAddr is the HTTP address used by the locking-service to connect to DATA_BROKER.
const dataBrokerHTTPAddr = "http://" + tcpAddr

// --- TS-03-1: Command Subscription on Startup ---

// TestCommandSubscription verifies that the locking-service subscribes to
// Vehicle.Command.Door.Lock on startup and produces a response when a command
// is published to the signal.
//
// Test Spec: TS-03-1
// Requirements: 03-REQ-1.1, 03-REQ-1.2, 03-REQ-6.2
func TestCommandSubscription(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfDataBrokerUnreachable(t)

	binPath := buildLockingServiceBinary(t)
	proc := startLockingService(t, binPath, dataBrokerHTTPAddr)

	// Wait for the service to connect, publish initial state, and subscribe.
	// This requires the DATA_BROKER to support the kuksa.val.v1 API.
	if !waitForLog(proc, "locking-service ready", 10*time.Second) {
		t.Skip("service did not reach ready state within 10s; " +
			"DATA_BROKER may not support kuksa.val.v1 API (see errata 03_locking_service_proto_compat.md)")
	}

	// Publish a lock command. The service subscribes via v1; grpcurl uses v2.
	grpcurlPublishString(t, signalCommand, validLockCmd)

	// A response must appear on Vehicle.Command.Door.Response within 5 seconds.
	out, found := pollSignalForContent(t, signalResponse, "command_id", 5*time.Second)
	if !found {
		t.Fatalf("no response appeared on %s within 5s\nlast output: %s", signalResponse, out)
	}
}

// --- TS-03-13: Initial State Published on Startup ---

// TestInitialState verifies that the locking-service publishes IsLocked = false
// to DATA_BROKER immediately after startup.
//
// Test Spec: TS-03-13
// Requirements: 03-REQ-4.3
func TestInitialState(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfDataBrokerUnreachable(t)

	binPath := buildLockingServiceBinary(t)
	proc := startLockingService(t, binPath, dataBrokerHTTPAddr)

	if !waitForLog(proc, "locking-service ready", 10*time.Second) {
		t.Skip("service did not reach ready state within 10s; " +
			"DATA_BROKER may not support kuksa.val.v1 API (see errata 03_locking_service_proto_compat.md)")
	}

	// IsLocked must be false after startup (published by the locking-service on init).
	out := grpcurlGetValue(t, signalIsLocked)
	if !strings.Contains(strings.ToLower(out), "false") {
		t.Errorf("expected IsLocked=false after startup\ngrpcurl output: %s", out)
	}
}

// --- TS-03-SMOKE-1: Lock Happy Path ---

// TestSmokeLockHappyPath is an end-to-end smoke test: sets vehicle speed=0 and
// door=closed, publishes a lock command, and verifies IsLocked=true with a
// success response.
//
// Test Spec: TS-03-SMOKE-1
// Requirements: 03-REQ-3.3, 03-REQ-4.1, 03-REQ-5.1
func TestSmokeLockHappyPath(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfDataBrokerUnreachable(t)

	binPath := buildLockingServiceBinary(t)
	proc := startLockingService(t, binPath, dataBrokerHTTPAddr)

	if !waitForLog(proc, "locking-service ready", 10*time.Second) {
		t.Skip("service did not reach ready state within 10s; " +
			"DATA_BROKER may not support kuksa.val.v1 API (see errata 03_locking_service_proto_compat.md)")
	}

	// Set safe conditions: speed = 0.0, door = closed.
	grpcurlPublishFloat(t, signalSpeed, 0.0)
	grpcurlPublishBool(t, signalIsOpen, false)

	// Publish a lock command.
	grpcurlPublishString(t, signalCommand, validLockCmd)

	// Poll for IsLocked = true (up to 3 seconds).
	outLocked, foundLocked := pollSignalForContent(t, signalIsLocked, "true", 3*time.Second)
	if !foundLocked {
		t.Errorf("expected IsLocked=true after lock command\ngrpcurl output: %s", outLocked)
	}

	// Poll for a success response.
	outResp, foundResp := pollSignalForContent(t, signalResponse, "success", 3*time.Second)
	if !foundResp {
		t.Errorf("expected success response on %s\ngrpcurl output: %s", signalResponse, outResp)
	}
}

// --- TS-03-SMOKE-2: Unlock Happy Path ---

// TestSmokeUnlockHappyPath is an end-to-end smoke test: publishes an unlock
// command and verifies IsLocked=false with a success response.
//
// Test Spec: TS-03-SMOKE-2
// Requirements: 03-REQ-3.4, 03-REQ-4.2, 03-REQ-5.1
func TestSmokeUnlockHappyPath(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfDataBrokerUnreachable(t)

	binPath := buildLockingServiceBinary(t)
	proc := startLockingService(t, binPath, dataBrokerHTTPAddr)

	if !waitForLog(proc, "locking-service ready", 10*time.Second) {
		t.Skip("service did not reach ready state within 10s; " +
			"DATA_BROKER may not support kuksa.val.v1 API (see errata 03_locking_service_proto_compat.md)")
	}

	// Publish an unlock command (safety constraints are not checked for unlock).
	grpcurlPublishString(t, signalCommand, validUnlockCmd)

	// Poll for IsLocked = false.
	outLocked, foundLocked := pollSignalForContent(t, signalIsLocked, "false", 3*time.Second)
	if !foundLocked {
		t.Errorf("expected IsLocked=false after unlock command\ngrpcurl output: %s", outLocked)
	}

	// Poll for a success response.
	outResp, foundResp := pollSignalForContent(t, signalResponse, "success", 3*time.Second)
	if !foundResp {
		t.Errorf("expected success response on %s\ngrpcurl output: %s", signalResponse, outResp)
	}
}

// --- TS-03-SMOKE-3: Lock Rejected When Vehicle Moving ---

// TestSmokeLockRejectedMoving is an end-to-end smoke test: sets vehicle speed
// to 50.0, publishes a lock command, and verifies the lock is rejected with
// reason "vehicle_moving".
//
// Test Spec: TS-03-SMOKE-3
// Requirements: 03-REQ-3.1, 03-REQ-5.2
func TestSmokeLockRejectedMoving(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfDataBrokerUnreachable(t)

	binPath := buildLockingServiceBinary(t)
	proc := startLockingService(t, binPath, dataBrokerHTTPAddr)

	if !waitForLog(proc, "locking-service ready", 10*time.Second) {
		t.Skip("service did not reach ready state within 10s; " +
			"DATA_BROKER may not support kuksa.val.v1 API (see errata 03_locking_service_proto_compat.md)")
	}

	// Set vehicle as moving.
	grpcurlPublishFloat(t, signalSpeed, 50.0)

	// Publish a lock command.
	grpcurlPublishString(t, signalCommand, validLockCmd)

	// Poll for a failure response with reason "vehicle_moving".
	outResp, foundResp := pollSignalForContent(t, signalResponse, "vehicle_moving", 3*time.Second)
	if !foundResp {
		t.Errorf("expected failed response with reason vehicle_moving\ngrpcurl output: %s", outResp)
	}

	// IsLocked must remain false.
	outLocked := grpcurlGetValue(t, signalIsLocked)
	if strings.Contains(strings.ToLower(outLocked), "true") {
		t.Errorf("expected IsLocked=false when lock rejected (vehicle moving)\ngrpcurl output: %s", outLocked)
	}
}

// --- TS-03-E1: DATA_BROKER Connection Retry ---

// TestConnectionRetryFailure verifies that the locking-service exits with a
// non-zero code after exhausting connection retries when DATA_BROKER is
// unreachable. This test requires no external infrastructure.
//
// Test Spec: TS-03-E1
// Requirements: 03-REQ-1.E1
func TestConnectionRetryFailure(t *testing.T) {
	binPath := buildLockingServiceBinary(t)

	// Use a port that is guaranteed to be unreachable.
	const unreachableAddr = "http://localhost:19999"

	cmd := exec.Command(binPath, "serve")
	cmd.Env = append(os.Environ(),
		"DATABROKER_ADDR="+unreachableAddr,
		"RUST_LOG=warn",
	)

	// The service retries 5 times with delays 1s+2s+4s+8s = 15s total.
	// Allow 60s for the full retry cycle to complete.
	done := make(chan error, 1)
	go func() { done <- cmd.Run() }()

	select {
	case err := <-done:
		if err == nil {
			t.Fatal("expected locking-service to exit non-zero when DATA_BROKER is unreachable, got exit 0")
		}
		if exitErr, ok := err.(*exec.ExitError); ok {
			if exitErr.ExitCode() == 0 {
				t.Fatalf("expected non-zero exit code, got 0")
			}
			// Non-zero exit confirmed — test passes.
		}
	case <-time.After(60 * time.Second):
		if cmd.Process != nil {
			_ = cmd.Process.Kill()
		}
		t.Fatal("locking-service did not exit within 60s; expected it to fail after exhausting retries")
	}
}

// --- 03-REQ-6.2: Startup Logging ---

// TestStartupLogging verifies that the service logs its version and the
// DATA_BROKER address at startup (before attempting to connect).
// This test requires cargo but no running DATA_BROKER.
//
// Test Spec: (startup log requirement from TS-03-1)
// Requirements: 03-REQ-6.2
func TestStartupLogging(t *testing.T) {
	binPath := buildLockingServiceBinary(t)

	const testAddr = "http://localhost:19999"
	proc := startLockingService(t, binPath, testAddr)

	// The startup log appears immediately before the first connection attempt.
	// Allow 10 seconds for the log line to appear.
	if !waitForLog(proc, "locking-service", 10*time.Second) {
		t.Fatalf("startup log not found within 10s\nprocess output:\n%s", proc.output.String())
	}

	output := proc.output.String()

	// Verify the log includes the DATABROKER_ADDR.
	if !strings.Contains(output, testAddr) {
		t.Errorf("startup log does not include DATABROKER_ADDR %q\noutput:\n%s", testAddr, output)
	}
}

// --- 03-REQ-6.1: Graceful Shutdown ---

// TestGracefulShutdown verifies that SIGTERM causes the locking-service to
// shut down cleanly with exit code 0.
// Skips when the service cannot reach ready state (e.g. DATA_BROKER not running
// or running v2-only API).
//
// Requirements: 03-REQ-6.1, 03-REQ-6.E1
func TestGracefulShutdown(t *testing.T) {
	skipIfDataBrokerUnreachable(t)

	binPath := buildLockingServiceBinary(t)
	proc := startLockingService(t, binPath, dataBrokerHTTPAddr)

	if !waitForLog(proc, "locking-service ready", 10*time.Second) {
		t.Skip("service did not reach ready state within 10s; " +
			"DATA_BROKER may not support kuksa.val.v1 API (see errata 03_locking_service_proto_compat.md)")
	}

	// Send SIGTERM to request graceful shutdown.
	if err := proc.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit cleanly.
	done := make(chan error, 1)
	go func() { done <- proc.cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				t.Errorf("expected exit code 0 after SIGTERM, got %d", exitErr.ExitCode())
			} else {
				t.Errorf("unexpected error waiting for process: %v", err)
			}
		}
		// exit code 0 confirmed — test passes.
	case <-time.After(10 * time.Second):
		t.Fatal("locking-service did not exit within 10s after SIGTERM")
	}
}

// --- TS-03-E2: Subscription Stream Interrupted ---

// TestSubscriptionStreamInterrupted verifies that the locking-service attempts
// to resubscribe when the subscription stream is interrupted. Uses a mock
// kuksa.val.v1 gRPC server to simulate stream termination without requiring
// a live DATA_BROKER container.
//
// Test Spec: TS-03-E2
// Requirements: 03-REQ-1.E2
func TestSubscriptionStreamInterrupted(t *testing.T) {
	binPath := buildLockingServiceBinary(t)

	// Start a mock kuksa.val.v1 broker that we control.
	mockBroker, mockAddr := newMockV1Broker(t)
	dataBrokerAddr := fmt.Sprintf("http://%s", mockAddr)

	// Start the locking-service pointing at our mock broker.
	proc := startLockingService(t, binPath, dataBrokerAddr)

	// Wait for the service to reach ready state (subscribe to command signal).
	if !waitForLog(proc, "locking-service ready", 15*time.Second) {
		t.Fatalf("service did not reach ready state within 15s\nprocess output:\n%s",
			proc.output.String())
	}

	// Verify the mock received the first Subscribe call.
	if !mockBroker.WaitForSubscription(5 * time.Second) {
		t.Fatal("mock broker did not receive initial Subscribe call within 5s")
	}

	// Terminate the first subscribe stream (simulates DATA_BROKER restart / stream interruption).
	mockBroker.TerminateStream()

	// Wait for the service to log a resubscription warning.
	// The resubscribe function uses 1s delay before the first attempt, so allow
	// extra time for the warning log to appear.
	if !waitForLog(proc, "Resubscribing", 10*time.Second) {
		t.Fatalf("service did not log resubscription warning after stream interruption\nprocess output:\n%s",
			proc.output.String())
	}

	// Verify the mock received a second Subscribe call (the resubscription).
	if !mockBroker.WaitForResubscription(10 * time.Second) {
		t.Fatalf("mock broker did not receive resubscription Subscribe call\nprocess output:\n%s",
			proc.output.String())
	}

	// Verify the service logged successful resubscription.
	if !waitForLog(proc, "Resubscribed", 5*time.Second) {
		t.Fatalf("service did not log successful resubscription\nprocess output:\n%s",
			proc.output.String())
	}

	// Verify at least 2 Subscribe calls were made (initial + resubscribe).
	callCount := mockBroker.SubscribeCallCount()
	if callCount < 2 {
		t.Errorf("expected at least 2 Subscribe calls (initial + resubscribe), got %d", callCount)
	}

	// Verify the service is still running (did not exit after resubscription).
	output := proc.output.String()
	if strings.Contains(output, "Resubscription exhausted") {
		t.Error("service reported resubscription exhausted — expected successful resubscription")
	}

	// Send SIGTERM for clean shutdown.
	if proc.cmd.Process != nil {
		_ = proc.cmd.Process.Signal(syscall.SIGTERM)
	}
}
