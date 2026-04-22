package natsclient_test

import (
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// TS-06-E6: NATS Connection Retry Exhaustion
// Requirement: 06-REQ-5.E1
func TestNATSConnectionRetryExhaustion(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping slow NATS retry test in short mode")
	}

	start := time.Now()
	_, err := natsclient.Connect("nats://localhost:19999", 5)
	elapsed := time.Since(start)

	if err == nil {
		t.Error("expected error when NATS server is unreachable, got nil")
	}
	if elapsed < 7*time.Second {
		t.Errorf("expected elapsed time >= 7s (backoff delays), got %v", elapsed)
	}
}
