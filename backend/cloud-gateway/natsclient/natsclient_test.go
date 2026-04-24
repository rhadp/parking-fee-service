package natsclient_test

import (
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// ---------------------------------------------------------------------------
// TS-06-E6: NATS Connection Retry Exhaustion
// Requirement: 06-REQ-5.E1
// Description: When NATS is unreachable, the client retries with backoff
// and returns an error after max attempts.
// ---------------------------------------------------------------------------

func TestNATSConnectionRetryExhaustion(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping slow retry exhaustion test in short mode")
	}

	start := time.Now()
	_, err := natsclient.Connect("nats://localhost:19999", 5)
	elapsed := time.Since(start)

	if err == nil {
		t.Fatal("expected error for unreachable NATS server, got nil")
	}

	if elapsed < 7*time.Second {
		t.Errorf("expected elapsed time >= 7s (exponential backoff), got %v", elapsed)
	}
}
