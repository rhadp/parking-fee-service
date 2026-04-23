package lockingservice_test

import (
	"encoding/json"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TestGracefulShutdown verifies the service exits with code 0 when receiving
// SIGTERM while idle (no command being processed).
// Requirement: 03-REQ-6.1
func TestGracefulShutdown(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	cmd, _ := startLockingServiceManual(t, binary, "http://"+tcpTarget)

	// Send SIGTERM to the service process.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit.
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err != nil {
			t.Errorf("expected clean exit, got error: %v", err)
		}
		exitCode := cmd.ProcessState.ExitCode()
		if exitCode != 0 {
			t.Errorf("expected exit code 0, got %d", exitCode)
		}
	case <-time.After(10 * time.Second):
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatal("service did not exit within 10 seconds after SIGTERM")
	}
}

// TestStartupLogging verifies the service logs its version and the
// DATABROKER_ADDR it is connecting to on startup.
// Requirement: 03-REQ-6.2
func TestStartupLogging(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	addr := "http://" + tcpTarget
	cmd, getLogs := startLockingServiceManual(t, binary, addr)
	defer func() {
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
	}()

	// Check that startup logs contain version and databroker address.
	logs := getLogs()

	foundVersion := false
	foundAddr := false
	for _, line := range logs {
		// The tracing output includes structured fields like:
		//   locking-service starting version=0.1.0 databroker_addr=http://...
		if strings.Contains(line, "locking-service starting") {
			if strings.Contains(line, "version") {
				foundVersion = true
			}
			if strings.Contains(line, addr) {
				foundAddr = true
			}
		}
	}

	if !foundVersion {
		t.Errorf("startup log does not contain version; logs:\n%s", strings.Join(logs, "\n"))
	}
	if !foundAddr {
		t.Errorf("startup log does not contain DATABROKER_ADDR (%s); logs:\n%s", addr, strings.Join(logs, "\n"))
	}
}

// TestSigtermDuringProcessing verifies the service completes the current
// command before exiting when SIGTERM is received during command processing.
// Requirement: 03-REQ-6.E1
func TestSigtermDuringProcessing(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	cmd, _ := startLockingServiceManual(t, binary, "http://"+tcpTarget)

	// Set safe conditions so the lock command will succeed.
	setFloat(t, client, signalSpeed, 0.0)
	setBool(t, client, signalDoorOpen, false)

	// Send a lock command.
	cmdID := "sigterm-during-processing-1"
	setString(t, client, signalCommand, makeLockJSON(cmdID))

	// Small delay to allow the service to start processing.
	time.Sleep(100 * time.Millisecond)

	// Send SIGTERM while command may still be in-flight.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit.
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err != nil {
			t.Errorf("expected clean exit, got error: %v", err)
		}
		exitCode := cmd.ProcessState.ExitCode()
		if exitCode != 0 {
			t.Errorf("expected exit code 0, got %d", exitCode)
		}
	case <-time.After(10 * time.Second):
		_ = cmd.Process.Kill()
		_ = cmd.Wait()
		t.Fatal("service did not exit within 10 seconds after SIGTERM")
	}

	// Check that the command response was published (the command completed
	// before shutdown). The response must match our command_id.
	resp := getString(t, client, signalResponse)
	if resp == nil || *resp == "" {
		t.Fatal("expected a response to be published before shutdown, got nil/empty")
	}
	var parsed map[string]interface{}
	if err := parseJSON(*resp, &parsed); err != nil {
		t.Fatalf("failed to parse response JSON: %v", err)
	}
	if id, ok := parsed["command_id"].(string); !ok || id != cmdID {
		t.Errorf("expected command_id %q in response, got %v", cmdID, parsed["command_id"])
	}
	if status, ok := parsed["status"].(string); !ok || status != "success" {
		t.Errorf("expected status 'success', got %v", parsed["status"])
	}
}

// TestSequentialCommandProcessing verifies that commands are processed
// sequentially: the first command completes before the second one is handled.
// Requirement: 03-REQ-1.3
func TestSequentialCommandProcessing(t *testing.T) {
	skipIfTCPUnreachable(t)

	binary := buildLockingService(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	_ = startLockingService(t, binary, "http://"+tcpTarget)

	// Set safe conditions.
	setFloat(t, client, signalSpeed, 0.0)
	setBool(t, client, signalDoorOpen, false)

	// Send first command: lock.
	lockCmdID := "seq-lock-1"
	setString(t, client, signalCommand, makeLockJSON(lockCmdID))
	resp1 := waitForResponse(t, client, lockCmdID, 5*time.Second)

	if status, ok := resp1["status"].(string); !ok || status != "success" {
		t.Errorf("first command (lock) expected status 'success', got %v", resp1["status"])
	}

	// Verify lock state is true after first command.
	locked := getBool(t, client, signalIsLocked)
	if locked == nil || !*locked {
		t.Error("expected IsLocked = true after first lock command")
	}

	// Send second command: unlock.
	unlockCmdID := "seq-unlock-1"
	setString(t, client, signalCommand, makeUnlockJSON(unlockCmdID))
	resp2 := waitForResponse(t, client, unlockCmdID, 5*time.Second)

	if status, ok := resp2["status"].(string); !ok || status != "success" {
		t.Errorf("second command (unlock) expected status 'success', got %v", resp2["status"])
	}

	// Verify lock state is false after second command.
	locked = getBool(t, client, signalIsLocked)
	if locked == nil || *locked {
		t.Error("expected IsLocked = false after unlock command")
	}
}

// parseJSON is a small helper to unmarshal JSON into a map.
func parseJSON(s string, out *map[string]interface{}) error {
	return json.Unmarshal([]byte(s), out)
}
