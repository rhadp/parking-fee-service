//go:build integration

package natsclient_test

import (
	"testing"
	"time"

	"github.com/nats-io/nats.go"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
)

// TestPropertyNATSHeaderPropagation verifies NATS messages include the bearer token (TS-06-P6).
func TestPropertyNATSHeaderPropagation(t *testing.T) {
	nc, err := natsclient.Connect("nats://localhost:4222", 3)
	if err != nil {
		t.Skipf("NATS not available: %v", err)
	}
	defer nc.Drain()

	tokens := []struct {
		token string
		vin   string
	}{
		{"demo-token-001", "VIN12345"},
		{"demo-token-002", "VIN67890"},
	}
	for _, tt := range tokens {
		// Subscribe directly using nats.go
		nconn, err := nats.Connect("nats://localhost:4222")
		if err != nil {
			t.Skipf("NATS direct connection not available: %v", err)
		}
		sub, err := nconn.SubscribeSync("vehicles." + tt.vin + ".commands")
		if err != nil {
			nconn.Close()
			t.Fatalf("Subscribe failed: %v", err)
		}
		// Flush ensures the SUB message is delivered to the server before publishing.
		if err := nconn.Flush(); err != nil {
			nconn.Close()
			t.Fatalf("Flush failed: %v", err)
		}
		cmd := model.Command{
			CommandID: "prop-cmd-" + tt.vin,
			Type:      "lock",
			Doors:     []string{"driver"},
		}
		if err := nc.PublishCommand(tt.vin, cmd, tt.token); err != nil {
			nconn.Close()
			t.Fatalf("PublishCommand failed: %v", err)
		}
		msg, err := sub.NextMsg(time.Second)
		if err != nil {
			nconn.Close()
			t.Fatalf("NextMsg failed: %v", err)
		}
		want := "Bearer " + tt.token
		if got := msg.Header.Get("Authorization"); got != want {
			t.Errorf("Authorization header: got %q, want %q", got, want)
		}
		nconn.Close()
	}
}
