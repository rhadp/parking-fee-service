//go:build integration

package natsclient_test

import (
	"encoding/json"
	"os"
	"testing"
	"time"

	"github.com/nats-io/nats.go"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// natsURL returns the NATS server URL for integration tests.
func natsURL() string {
	if url := os.Getenv("NATS_URL"); url != "" {
		return url
	}
	return "nats://localhost:4222"
}

// TS-06-P6: NATS Header Propagation
// Property 5 from design.md
// Validates: 06-REQ-1.2
// For any command published to NATS, the message contains the bearer token
// from the originating REST request in the Authorization header.
func TestPropertyNATSHeaderPropagation(t *testing.T) {
	nc, err := natsclient.Connect(natsURL(), 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	tokens := []struct {
		token string
		vin   string
	}{
		{"token-prop-1", "VIN-PROP-1"},
		{"token-prop-2", "VIN-PROP-2"},
		{"token-prop-3", "VIN-PROP-3"},
	}

	// Use a raw NATS connection for subscribing to verify headers
	rawNC, err := nats.Connect(natsURL())
	if err != nil {
		t.Fatalf("failed to create raw NATS connection: %v", err)
	}
	defer rawNC.Close()

	for _, tt := range tokens {
		t.Run(tt.token, func(t *testing.T) {
			subject := "vehicles." + tt.vin + ".commands"
			sub, err := rawNC.SubscribeSync(subject)
			if err != nil {
				t.Fatalf("failed to subscribe: %v", err)
			}
			defer sub.Unsubscribe()

			cmd := model.Command{
				CommandID: "prop-" + tt.token,
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

			// Verify Authorization header
			authHeader := msg.Header.Get("Authorization")
			expected := "Bearer " + tt.token
			if authHeader != expected {
				t.Errorf("Authorization header = %q, want %q", authHeader, expected)
			}

			// Verify command payload
			var received model.Command
			if err := json.Unmarshal(msg.Data, &received); err != nil {
				t.Fatalf("failed to unmarshal command: %v", err)
			}
			if received.CommandID != cmd.CommandID {
				t.Errorf("CommandID = %q, want %q", received.CommandID, cmd.CommandID)
			}
		})
	}
}
