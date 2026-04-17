package natsclient_test

import (
	"testing"
	"time"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/natsclient"
)

// TestNATSConnectionRetryExhaustion verifies that Connect retries with exponential
// backoff and returns an error after maxRetries attempts. The total elapsed time must
// be at least 7 seconds (1+2+4 seconds of backoff between 5 attempts).
// TS-06-E6
//
// This test will FAIL with the stub because the stub returns immediately.
func TestNATSConnectionRetryExhaustion(t *testing.T) {
	// Port 19999 — nothing should be listening there.
	const unreachableURL = "nats://localhost:19999"
	const maxRetries = 5

	start := time.Now()
	_, err := natsclient.Connect(unreachableURL, maxRetries)
	elapsed := time.Since(start)

	if err == nil {
		t.Fatal("Connect to unreachable URL: want non-nil error, got nil")
	}
	// With 5 retries and backoff delays of 1s, 2s, 4s (capped or continued),
	// the minimum total wait is 1+2+4 = 7 seconds.
	const minElapsed = 7 * time.Second
	if elapsed < minElapsed {
		t.Errorf("elapsed: want >= %v, got %v (retry backoff not implemented)", minElapsed, elapsed)
	}
}
