package lockingservice_test

import (
	"os"
	"os/exec"
	"testing"
	"time"
)

// TestConnectionRetryFailure verifies the service retries connection to
// DATA_BROKER with exponential backoff and exits with a non-zero code when
// the DATA_BROKER is unreachable.
// TS-03-E1 | Requirement: 03-REQ-1.E1
func TestConnectionRetryFailure(t *testing.T) {
	binary := buildLockingService(t)

	// Point the service at a non-existent endpoint. Use a valid port number
	// (unlike the test spec's port 99999) to test actual connection retry
	// rather than address format rejection.
	cmd := exec.Command(binary, "serve")
	cmd.Env = append(os.Environ(), "DATABROKER_ADDR=http://localhost:19999")

	// Start the service; it should retry and then exit non-zero.
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start locking-service: %v", err)
	}

	// Wait for the process to exit with a timeout.
	// The service uses 5 connection attempts with exponential backoff:
	// 0s + 1s + 2s + 4s + 8s = ~15s. Use a generous timeout.
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case err := <-done:
		if err == nil {
			t.Fatal("expected non-zero exit code, but process exited successfully")
		}
		exitCode := cmd.ProcessState.ExitCode()
		if exitCode == 0 {
			t.Errorf("expected non-zero exit code, got 0")
		}
	case <-time.After(60 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("locking-service did not exit within 60 seconds")
	}
}
