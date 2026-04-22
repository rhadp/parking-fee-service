//go:build integration

package natsclient_test

import (
	"encoding/json"
	"testing"
	"time"

	"github.com/nats-io/nats.go"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// TS-06-P6: NATS Header Propagation (Property Test)
// Property 5: For any command published to NATS, the message contains the
// bearer token from the originating REST request in the Authorization header.
// Validates: 06-REQ-1.2
func TestPropertyNATSHeaderPropagation(t *testing.T) {
	nc, err := natsclient.Connect("nats://localhost:4222", 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	// Connect a raw NATS subscriber to verify message headers.
	rawNC, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("failed to connect raw NATS client: %v", err)
	}
	defer rawNC.Close()

	tokens := []struct {
		token string
		vin   string
	}{
		{"test-token-001", "VIN_P6_001"},
		{"test-token-002", "VIN_P6_002"},
		{"test-token-003", "VIN_P6_003"},
	}

	for _, tc := range tokens {
		t.Run("token_"+tc.token, func(t *testing.T) {
			sub, err := rawNC.SubscribeSync("vehicles." + tc.vin + ".commands")
			if err != nil {
				t.Fatalf("failed to subscribe: %v", err)
			}
			defer sub.Unsubscribe()

			cmd := model.Command{
				CommandID: "p6-" + tc.token,
				Type:      "lock",
				Doors:     []string{"driver"},
			}

			if err := nc.PublishCommand(tc.vin, cmd, tc.token); err != nil {
				t.Fatalf("PublishCommand failed: %v", err)
			}

			msg, err := sub.NextMsg(2 * time.Second)
			if err != nil {
				t.Fatalf("did not receive NATS message: %v", err)
			}

			// Verify Authorization header.
			authHeader := msg.Header.Get("Authorization")
			expected := "Bearer " + tc.token
			if authHeader != expected {
				t.Errorf("expected Authorization header %q, got %q", expected, authHeader)
			}

			// Verify command payload.
			var received model.Command
			if err := json.Unmarshal(msg.Data, &received); err != nil {
				t.Fatalf("failed to unmarshal NATS message: %v", err)
			}
			if received.CommandID != cmd.CommandID {
				t.Errorf("expected command_id %q, got %q", cmd.CommandID, received.CommandID)
			}
		})
	}
}
