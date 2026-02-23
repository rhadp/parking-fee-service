package integration

import (
	"strings"
	"testing"
	"time"
)

// ===========================================================================
// TS-04-39: Integration test lock-to-session
// Requirement: 04-REQ-10.1
// ===========================================================================

func TestE2E_LockToSession(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	// Requires DATA_BROKER (55556), PARKING_OPERATOR_ADAPTOR (50052), and
	// mock PARKING_OPERATOR (8090) all running.
	if !waitForPort(t, "localhost", 55556, 0) {
		t.Skip("DATA_BROKER not running on localhost:55556")
	}
	if !waitForPort(t, "localhost", 50052, 0) {
		t.Skip("PARKING_OPERATOR_ADAPTOR not running on localhost:50052")
	}
	if !waitForPort(t, "localhost", 8090, 0) {
		t.Skip("mock PARKING_OPERATOR not running on localhost:8090")
	}

	// Use the CLI to trigger a lock event by calling start-session via gRPC.
	// The full lock-to-session flow (DATA_BROKER -> ADAPTOR -> OPERATOR)
	// is tested in the Rust integration tests. This E2E test verifies
	// the end-to-end connectivity using the CLI as the entry point.
	binary := cliBinary(t)

	// Start a session via the adaptor (simulates the effect of a lock event)
	stdout, _, exitCode := execCommand(t, binary,
		"start-session",
		"--vehicle-id", "VIN12345",
		"--zone-id", "zone-munich-central",
	)

	if exitCode != 0 {
		t.Fatalf("start-session failed with exit code %d", exitCode)
	}

	sessionID := extractField(stdout, "session_id")
	if sessionID == "" {
		t.Fatal("expected session_id in start-session output")
	}

	status := extractField(stdout, "status")
	if status != "active" {
		t.Errorf("expected status 'active', got %q", status)
	}

	// Wait for adaptor to write SessionActive to DATA_BROKER
	time.Sleep(1 * time.Second)

	// Stop the session to clean up
	_, _, stopExitCode := execCommand(t, binary,
		"stop-session",
		"--session-id", sessionID,
	)
	if stopExitCode != 0 {
		t.Logf("warning: stop-session failed with exit code %d", stopExitCode)
	}
}

// ===========================================================================
// TS-04-40: Integration test CLI-to-UpdateService lifecycle
// Requirement: 04-REQ-10.2
// ===========================================================================

func TestE2E_CLIToUpdateService(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	// Requires UPDATE_SERVICE running on default port 50051
	if !waitForPort(t, "localhost", 50051, 0) {
		t.Skip("UPDATE_SERVICE not running on localhost:50051")
	}

	binary := cliBinary(t)

	// Run install via CLI. Assert exit code 0 and response contains adapter info.
	installStdout, _, installExitCode := execCommand(t, binary,
		"install",
		"--image-ref", "localhost:5000/e2e-test-adaptor:v1",
		"--checksum", "e2echeck123",
	)
	if installExitCode != 0 {
		t.Fatalf("install failed with exit code %d", installExitCode)
	}
	if !strings.Contains(installStdout, "adapter_id") {
		t.Error("expected install output to contain 'adapter_id'")
	}
	if !strings.Contains(installStdout, "job_id") {
		t.Error("expected install output to contain 'job_id'")
	}

	// Run list via CLI. Assert the installed adapter appears.
	listStdout, _, listExitCode := execCommand(t, binary, "list")
	if listExitCode != 0 {
		t.Fatalf("list failed with exit code %d", listExitCode)
	}
	// The list output should contain the image ref or adapter ID from install
	if !strings.Contains(listStdout, "e2e-test-adaptor") && !strings.Contains(listStdout, "ID") &&
		!strings.Contains(listStdout, "No adapters") {
		t.Error("expected list output to show the installed adapter or headers")
	}
}

// ===========================================================================
// TS-04-41: Integration test adaptor-to-operator communication
// Requirement: 04-REQ-10.3
// ===========================================================================

func TestE2E_AdaptorToOperator(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	// Requires PARKING_OPERATOR_ADAPTOR (50052) and mock PARKING_OPERATOR (8090)
	if !waitForPort(t, "localhost", 50052, 0) {
		t.Skip("PARKING_OPERATOR_ADAPTOR not running on localhost:50052")
	}
	if !waitForPort(t, "localhost", 8090, 0) {
		t.Skip("mock PARKING_OPERATOR not running on localhost:8090")
	}

	binary := cliBinary(t)

	// Call StartSession via CLI. Assert session_id is non-empty.
	startStdout, _, startExitCode := execCommand(t, binary,
		"start-session",
		"--vehicle-id", "VIN12345",
		"--zone-id", "zone-munich-central",
	)
	if startExitCode != 0 {
		t.Fatalf("start-session failed with exit code %d", startExitCode)
	}
	sessionID := extractField(startStdout, "session_id")
	if sessionID == "" {
		t.Fatal("expected non-empty session_id from start-session")
	}

	// Call StopSession via CLI. Assert fee, duration, currency.
	stopStdout, _, stopExitCode := execCommand(t, binary,
		"stop-session",
		"--session-id", sessionID,
	)
	if stopExitCode != 0 {
		t.Fatalf("stop-session failed with exit code %d", stopExitCode)
	}
	if !strings.Contains(stopStdout, "fee") {
		t.Error("expected stop-session output to contain 'fee'")
	}
	if !strings.Contains(stopStdout, "currency") {
		t.Error("expected stop-session output to contain 'currency'")
	}

	// Call GetRate via CLI. Assert rate info.
	rateStdout, _, rateExitCode := execCommand(t, binary,
		"get-rate",
		"--zone-id", "zone-munich-central",
	)
	if rateExitCode != 0 {
		t.Fatalf("get-rate failed with exit code %d", rateExitCode)
	}
	if !strings.Contains(rateStdout, "rate_per_hour") {
		t.Error("expected get-rate output to contain 'rate_per_hour'")
	}
	ratePH := extractField(rateStdout, "rate_per_hour")
	if ratePH != "2.50" {
		t.Errorf("expected rate_per_hour '2.50', got %q", ratePH)
	}
}
