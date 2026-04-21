package natsclient_test

import (
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// TestNATSConnectionRetryExhaustion verifies Connect retries and returns error (TS-06-E6).
// This test takes ~7+ seconds due to exponential backoff (1s+2s+4s between attempts).
func TestNATSConnectionRetryExhaustion(t *testing.T) {
	start := time.Now()
	_, err := natsclient.Connect("nats://localhost:19999", 5)
	elapsed := time.Since(start)
	if err == nil {
		t.Error("Connect: expected error for unreachable NATS server, got nil")
	}
	if elapsed < 7*time.Second {
		t.Errorf("Connect: elapsed %v, want >= 7s (exponential backoff 1+2+4s)", elapsed)
	}
}
