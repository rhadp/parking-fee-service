package natsclient_test

import (
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// TestNATSConnectionRetryExhaustion verifies Connect retries and returns error (TS-06-E6).
// With maxRetries=5, the implementation makes 5 connection attempts with 4 inter-attempt
// waits using exponential backoff: 1s + 2s + 4s + 8s = 15s of backoff sleep.
// Each connection attempt also has a 2-second timeout, but connections to non-listening
// ports typically fail immediately with "connection refused".
// Lower bound: >= 7s (per test spec TS-06-E6).
// Upper bound: < 30s (5 connect timeouts of 2s each + 15s backoff + margin).
// See docs/errata/06_cloud_gateway.md E1 for the spec ambiguity.
func TestNATSConnectionRetryExhaustion(t *testing.T) {
	start := time.Now()
	_, err := natsclient.Connect("nats://localhost:19999", 5)
	elapsed := time.Since(start)
	if err == nil {
		t.Error("Connect: expected error for unreachable NATS server, got nil")
	}
	if elapsed < 7*time.Second {
		t.Errorf("Connect: elapsed %v, want >= 7s (exponential backoff)", elapsed)
	}
	if elapsed > 30*time.Second {
		t.Errorf("Connect: elapsed %v, want < 30s (upper bound for 5 attempts with backoff)", elapsed)
	}
}
