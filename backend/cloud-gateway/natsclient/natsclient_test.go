package natsclient

import (
	"testing"
	"time"
)

// TS-06-E6: NATS Connection Retry Exhaustion
// This test verifies that Connect retries with exponential backoff and returns
// an error after max attempts when the NATS server is unreachable.
// NOTE: This test is intentionally slow (~7s) due to backoff waits.
func TestNATSConnectionRetryExhaustion(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping slow test in short mode")
	}

	// Use an unlikely port to ensure no NATS server is reachable.
	start := time.Now()
	_, err := Connect("nats://localhost:19999", 5)
	elapsed := time.Since(start)

	if err == nil {
		t.Fatal("Connect should return error when NATS is unreachable")
	}
	// Expect at least 7 seconds of backoff (1+2+4 between 5 attempts)
	if elapsed.Seconds() < 7 {
		t.Errorf("elapsed = %v, want >= 7s (exponential backoff)", elapsed)
	}
}
