package lockingsvc_test

import (
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-03-E1: DATA_BROKER Connection Retry
// Requirement: 03-REQ-1.E1
// Description: Verify the service retries connection to DATA_BROKER with
// exponential backoff and exits with non-zero code after exhausting retries.
// ---------------------------------------------------------------------------

func TestConnectionRetryFailure(t *testing.T) {
	bin := buildLockingService(t)

	// Start the service with a non-existent DATA_BROKER address.
	// The service uses 5 attempts with delays of 1s, 2s, 4s, 8s (~15s total).
	proc := startLockingService(t, bin, "http://localhost:19999")

	// Wait for the service to exit (should take ~15-20 seconds for all retries).
	exitCode := proc.waitExit(60 * time.Second)
	if exitCode == -1 {
		t.Fatal("timed out waiting for locking-service to exit after connection retries")
	}

	if exitCode == 0 {
		t.Error("expected non-zero exit code when DATA_BROKER is unreachable, got 0")
	}
}
