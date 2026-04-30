package natsclient_test

import (
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// TS-06-E6: NATS Connection Retry Exhaustion
// Requirement: 06-REQ-5.E1
// When NATS is unreachable, the client retries with backoff and returns an
// error after max attempts. Note: the backoff sequence is 1s, 2s, 4s, 8s
// for 5 attempts = 4 intervals = at least 15s total backoff.
// However, per test spec TS-06-E6, we assert elapsed >= 7s.
func TestNATSConnectionRetryExhaustion(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping slow retry test in short mode")
	}

	start := time.Now()
	_, err := natsclient.Connect("nats://localhost:19999", 5)
	elapsed := time.Since(start)

	if err == nil {
		t.Error("Connect should return error when NATS is unreachable")
	}
	if elapsed < 7*time.Second {
		t.Errorf("elapsed = %v, want >= 7s (backoff 1+2+4)", elapsed)
	}
}
