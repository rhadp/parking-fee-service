package lockingsvc_test

import (
	"os/exec"
	"runtime"
	"strings"
	"syscall"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TestGracefulShutdown: SIGTERM exits code 0
// Requirement: 03-REQ-6.1
// Description: Verify that SIGTERM causes the service to shut down gracefully
// and exit with code 0.
// ---------------------------------------------------------------------------

func TestGracefulShutdown(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Send SIGTERM.
	if err := proc.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	exitCode := proc.waitExit(10 * time.Second)
	if exitCode != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d", exitCode)
	}
}

// ---------------------------------------------------------------------------
// TestStartupLogging: Verify startup log contains version and DATABROKER_ADDR
// Requirement: 03-REQ-6.2
// Description: Verify the service logs its version and the DATA_BROKER address
// it is connecting to at startup.
// ---------------------------------------------------------------------------

func TestStartupLogging(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Send SIGTERM to stop the service so we can read complete stderr.
	if err := proc.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}
	proc.waitExit(10 * time.Second)

	stderr := proc.getStderr()

	// The startup log should contain the version.
	if !strings.Contains(stderr, "version") {
		t.Error("startup logs do not contain version information")
	}

	// The startup log should contain the DATABROKER_ADDR.
	if !strings.Contains(stderr, databrokerAddr) {
		t.Errorf("startup logs do not contain DATABROKER_ADDR %q", databrokerAddr)
	}

	// The startup log should contain "locking-service starting".
	if !strings.Contains(stderr, "locking-service starting") {
		t.Error("startup logs do not contain 'locking-service starting'")
	}
}

// ---------------------------------------------------------------------------
// TestSigtermDuringProcessing: Command completes before shutdown
// Requirement: 03-REQ-6.E1
// Description: Verify that SIGTERM received during command processing allows
// the current command to complete before exiting.
// ---------------------------------------------------------------------------

func TestSigtermDuringProcessing(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("signal tests not supported on Windows")
	}

	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)
	_, client := dialDatabroker(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Set safety conditions.
	publishValue(t, client, signalSpeed, floatValue(0.0))
	publishValue(t, client, signalDoorOpen, boolValue(false))

	// Publish a lock command.
	cmdID := "sigterm-during-001"
	publishValue(t, client, signalCommand, stringValue(lockCommandJSON(cmdID)))

	// Send SIGTERM shortly after the command is published.
	// The service should complete the in-flight command before exiting.
	time.Sleep(100 * time.Millisecond)
	if err := proc.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// The response for the in-flight command should be published.
	resp := waitForNewResponse(t, client, cmdID, 5*time.Second)
	if resp == nil {
		t.Fatal("expected a response for the in-flight command before shutdown")
	}

	// The service should exit with code 0.
	exitCode := proc.waitExit(10 * time.Second)
	if exitCode != 0 {
		t.Errorf("expected exit code 0 after SIGTERM, got %d", exitCode)
	}
}

// ---------------------------------------------------------------------------
// TestSequentialCommandProcessing: Commands processed in order
// Requirement: 03-REQ-1.3
// Description: Verify that commands are processed sequentially and all
// receive responses.
// ---------------------------------------------------------------------------

func TestSequentialCommandProcessing(t *testing.T) {
	skipIfTCPUnreachable(t)
	bin := buildLockingService(t)
	_, client := dialDatabroker(t)

	proc, lines := startLockingServiceWithStderrScanner(t, bin, databrokerAddr)
	waitReady(t, lines, 30*time.Second)

	// Set safety conditions.
	publishValue(t, client, signalSpeed, floatValue(0.0))
	publishValue(t, client, signalDoorOpen, boolValue(false))

	// Send a sequence of commands: lock, unlock, lock.
	commands := []struct {
		id     string
		json   string
		status string
	}{
		{"seq-001", lockCommandJSON("seq-001"), "success"},
		{"seq-002", unlockCommandJSON("seq-002"), "success"},
		{"seq-003", lockCommandJSON("seq-003"), "success"},
	}

	for _, cmd := range commands {
		publishValue(t, client, signalCommand, stringValue(cmd.json))
		resp := waitForNewResponse(t, client, cmd.id, 5*time.Second)
		if status, _ := resp["status"].(string); status != cmd.status {
			t.Errorf("command %s: expected status=%q, got %q", cmd.id, cmd.status, status)
		}
	}

	_ = proc
}

// ---------------------------------------------------------------------------
// TestUsageWithoutArgs: Verify the binary prints usage when run without args
// and exits with code 0.
// ---------------------------------------------------------------------------

func TestUsageWithoutArgs(t *testing.T) {
	bin := buildLockingService(t)

	cmd := exec.Command(bin)
	out, err := cmd.CombinedOutput()
	if err != nil {
		// The binary should exit 0 when no args are given.
		if exitErr, ok := err.(*exec.ExitError); ok {
			t.Errorf("expected exit 0 with no args, got %d\noutput: %s", exitErr.ExitCode(), string(out))
			return
		}
		t.Fatalf("failed to run binary: %v", err)
	}

	output := string(out)
	if !strings.Contains(output, "locking-service") {
		t.Error("usage output does not contain 'locking-service'")
	}
	if !strings.Contains(output, "serve") {
		t.Error("usage output does not mention 'serve' subcommand")
	}
}
