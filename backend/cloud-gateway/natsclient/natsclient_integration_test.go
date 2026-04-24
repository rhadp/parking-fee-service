//go:build integration

package natsclient_test

import (
	"encoding/json"
	"fmt"
	"testing"
	"time"

	"github.com/nats-io/nats.go"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// ---------------------------------------------------------------------------
// TS-06-P6: NATS Header Propagation Property
// Property 5 from design.md (TS-06-P6 per test_spec numbering)
// Requirement: 06-REQ-1.2
// ---------------------------------------------------------------------------

func TestPropertyNATSHeaderPropagation(t *testing.T) {
	natsURL := "nats://localhost:4222"

	nc, err := natsclient.Connect(natsURL, 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	// Connect a raw NATS client to subscribe and verify headers
	rawNC, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect raw NATS client: %v", err)
	}
	defer rawNC.Close()

	tokens := []struct {
		token string
		vin   string
	}{
		{"test-token-001", "VIN001"},
		{"test-token-002", "VIN002"},
		{"test-token-003", "VIN003"},
	}

	for i, tt := range tokens {
		t.Run(fmt.Sprintf("token_%d", i), func(t *testing.T) {
			subject := "vehicles." + tt.vin + ".commands"
			sub, err := rawNC.SubscribeSync(subject)
			if err != nil {
				t.Fatalf("failed to subscribe to %s: %v", subject, err)
			}
			defer sub.Unsubscribe()

			cmd := model.Command{
				CommandID: fmt.Sprintf("prop-cmd-%d", i),
				Type:      "lock",
				Doors:     []string{"driver"},
			}

			if err := nc.PublishCommand(tt.vin, cmd, tt.token); err != nil {
				t.Fatalf("PublishCommand failed: %v", err)
			}

			msg, err := sub.NextMsg(1 * time.Second)
			if err != nil {
				t.Fatalf("did not receive NATS message: %v", err)
			}

			// Verify the command payload
			var receivedCmd model.Command
			if err := json.Unmarshal(msg.Data, &receivedCmd); err != nil {
				t.Fatalf("failed to unmarshal NATS message: %v", err)
			}
			if receivedCmd.CommandID != cmd.CommandID {
				t.Errorf("expected command_id %q, got %q",
					cmd.CommandID, receivedCmd.CommandID)
			}

			// Verify the Authorization header
			authHeader := msg.Header.Get("Authorization")
			expectedAuth := "Bearer " + tt.token
			if authHeader != expectedAuth {
				t.Errorf("expected Authorization header %q, got %q",
					expectedAuth, authHeader)
			}
		})
	}
}
