package integration

import (
	"strings"
	"testing"
)

// ===========================================================================
// TS-04-34: CLI install command calls InstallAdapter
// Requirement: 04-REQ-9.1
// ===========================================================================

func TestCLI_Install(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires UPDATE_SERVICE running — will be enabled in task group 7")

	binary := cliBinary(t)
	stdout, _, exitCode := execCommand(t, binary,
		"install",
		"--image-ref", "localhost:5000/adaptor:v1",
		"--checksum", "abc123",
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if !strings.Contains(stdout, "job_id") {
		t.Error("expected output to contain 'job_id'")
	}
	if !strings.Contains(stdout, "adapter_id") {
		t.Error("expected output to contain 'adapter_id'")
	}
	if !strings.Contains(stdout, "state") {
		t.Error("expected output to contain 'state'")
	}
}

// ===========================================================================
// TS-04-35: CLI watch command streams events
// Requirement: 04-REQ-9.2
// ===========================================================================

func TestCLI_Watch(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires UPDATE_SERVICE running — will be enabled in task group 7")

	// Start watch in background, trigger a state change, verify output.
	// This test would start the CLI watch command, trigger an install via
	// gRPC, then check that the watch output contains adapter state events.
	binary := cliBinary(t)
	_ = binary
	t.Fatal("watch streaming test not yet implemented")
}

// ===========================================================================
// TS-04-36: CLI list command prints adapters table
// Requirement: 04-REQ-9.3
// ===========================================================================

func TestCLI_List(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires UPDATE_SERVICE running — will be enabled in task group 7")

	binary := cliBinary(t)
	stdout, _, exitCode := execCommand(t, binary, "list")

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	// Output should contain column headers or adapter data
	if !strings.Contains(stdout, "adapter_id") && !strings.Contains(stdout, "ID") {
		t.Error("expected output to contain 'adapter_id' or 'ID'")
	}
}

// ===========================================================================
// TS-04-37: CLI start-session command calls StartSession
// Requirement: 04-REQ-9.4
// ===========================================================================

func TestCLI_StartSession(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires PARKING_OPERATOR_ADAPTOR + mock operator running — will be enabled in task group 7")

	binary := cliBinary(t)
	stdout, _, exitCode := execCommand(t, binary,
		"start-session",
		"--vehicle-id", "VIN12345",
		"--zone-id", "zone-munich-central",
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if !strings.Contains(stdout, "session_id") {
		t.Error("expected output to contain 'session_id'")
	}
	if !strings.Contains(stdout, "status") {
		t.Error("expected output to contain 'status'")
	}
}

// ===========================================================================
// TS-04-38: CLI stop-session command calls StopSession
// Requirement: 04-REQ-9.5
// ===========================================================================

func TestCLI_StopSession(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires PARKING_OPERATOR_ADAPTOR + mock operator running — will be enabled in task group 7")

	binary := cliBinary(t)

	// First start a session
	startStdout, _, startExitCode := execCommand(t, binary,
		"start-session",
		"--vehicle-id", "VIN12345",
		"--zone-id", "zone-munich-central",
	)
	if startExitCode != 0 {
		t.Fatalf("expected exit code 0 from start-session, got %d", startExitCode)
	}

	// Extract session_id (simplified — in practice parse output)
	sessionID := extractField(startStdout, "session_id")
	if sessionID == "" {
		t.Fatal("could not extract session_id from start-session output")
	}

	// Stop the session
	stdout, _, exitCode := execCommand(t, binary,
		"stop-session",
		"--session-id", sessionID,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if !strings.Contains(stdout, "fee") {
		t.Error("expected output to contain 'fee'")
	}
	if !strings.Contains(stdout, "duration") {
		t.Error("expected output to contain 'duration'")
	}
	if !strings.Contains(stdout, "currency") {
		t.Error("expected output to contain 'currency'")
	}
}

// ===========================================================================
// TS-04-E17: CLI command when service unreachable
// Requirement: 04-REQ-9.E1
// ===========================================================================

func TestEdge_CLIServiceUnreachable(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	binary := cliBinary(t)

	// Test install command with unreachable UPDATE_SERVICE
	_, stderr1, exitCode1 := execCommandWithEnv(t,
		map[string]string{},
		binary,
		"install",
		"--update-addr", "localhost:19999",
		"--image-ref", "test:v1",
		"--checksum", "abc123",
	)

	if exitCode1 == 0 {
		t.Error("expected non-zero exit code when service is unreachable")
	}
	if !strings.Contains(stderr1, "19999") && !strings.Contains(stderr1, "unreachable") &&
		!strings.Contains(stderr1, "connection") && !strings.Contains(stderr1, "error") {
		t.Errorf("expected error output to mention address or connection issue, got: %s", stderr1)
	}

	// Test start-session command with unreachable PARKING_OPERATOR_ADAPTOR
	_, stderr2, exitCode2 := execCommandWithEnv(t,
		map[string]string{},
		binary,
		"start-session",
		"--adaptor-addr", "localhost:19998",
		"--vehicle-id", "VIN12345",
		"--zone-id", "zone1",
	)

	if exitCode2 == 0 {
		t.Error("expected non-zero exit code when adaptor is unreachable")
	}
	if !strings.Contains(stderr2, "19998") && !strings.Contains(stderr2, "unreachable") &&
		!strings.Contains(stderr2, "connection") && !strings.Contains(stderr2, "error") {
		t.Errorf("expected error output to mention address or connection issue, got: %s", stderr2)
	}
}

// extractField is a simple helper that tries to extract a field value from
// key-value output (e.g., "session_id: abc123" -> "abc123").
func extractField(output, field string) string {
	for _, line := range strings.Split(output, "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, field) {
			parts := strings.SplitN(line, ":", 2)
			if len(parts) == 2 {
				return strings.TrimSpace(parts[1])
			}
			parts = strings.SplitN(line, "=", 2)
			if len(parts) == 2 {
				return strings.TrimSpace(parts[1])
			}
		}
	}
	return ""
}
