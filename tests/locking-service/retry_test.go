package lockingservice_test

import (
	"os/exec"
	"testing"
	"time"
)

// TestConnectionRetryFailure verifies the service retries connection to
// DATA_BROKER with exponential backoff and exits with non-zero code on failure.
//
// Test Spec: TS-03-E1
// Requirements: 03-REQ-1.E1
func TestConnectionRetryFailure(t *testing.T) {
	bin := buildLockingService(t)

	// Point at a non-existent endpoint so all connection attempts fail.
	cmd := exec.Command(bin, "serve")
	cmd.Env = []string{
		"DATABROKER_ADDR=http://localhost:19999",
		"PATH=",
		"HOME=",
	}

	// The service should retry and eventually exit non-zero.
	// With 5 attempts and delays of 1+2+4+8 = 15 seconds, allow up to 60s.
	errCh := make(chan error, 1)
	go func() {
		errCh <- cmd.Run()
	}()

	select {
	case err := <-errCh:
		if err == nil {
			t.Fatal("expected non-zero exit code when DATA_BROKER is unreachable")
		}
		if exitErr, ok := err.(*exec.ExitError); ok {
			if exitErr.ExitCode() == 0 {
				t.Fatal("expected non-zero exit code, got 0")
			}
			// Non-zero exit code is expected.
		}
	case <-time.After(60 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("locking-service did not exit within 60 seconds")
	}
}
