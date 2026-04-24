//go:build integration

package natsclient_test

import (
	"bytes"
	"encoding/json"
	"fmt"
	"log/slog"
	"strings"
	"testing"
	"time"

	"github.com/nats-io/nats.go"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
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

// ---------------------------------------------------------------------------
// TS-06-6: NATS Response Subscription
// Requirement: 06-REQ-5.1, 06-REQ-5.2
// Description: The service subscribes to command responses on NATS and stores them.
// ---------------------------------------------------------------------------

func TestNATSResponseSubscription(t *testing.T) {
	natsURL := "nats://localhost:4222"

	nc, err := natsclient.Connect(natsURL, 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	s := store.NewStore()
	if err := nc.SubscribeResponses(s); err != nil {
		t.Fatalf("failed to subscribe to responses: %v", err)
	}

	// Connect a raw NATS client to publish a command response.
	rawNC, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect raw NATS client: %v", err)
	}
	defer rawNC.Close()

	respJSON := `{"command_id":"cmd-005","status":"success"}`
	if err := rawNC.Publish("vehicles.VIN12345.command_responses", []byte(respJSON)); err != nil {
		t.Fatalf("failed to publish response: %v", err)
	}
	rawNC.Flush()

	time.Sleep(100 * time.Millisecond) // allow subscription processing

	resp, found := s.GetResponse("cmd-005")
	if !found {
		t.Fatal("expected response to be found in store")
	}
	if resp.CommandID != "cmd-005" {
		t.Errorf("expected CommandID 'cmd-005', got %q", resp.CommandID)
	}
	if resp.Status != "success" {
		t.Errorf("expected Status 'success', got %q", resp.Status)
	}
}

// ---------------------------------------------------------------------------
// TS-06-7: Telemetry Subscription Logging
// Requirement: 06-REQ-5.3
// Description: The service subscribes to telemetry on NATS and logs it
// without storing.
// ---------------------------------------------------------------------------

func TestTelemetrySubscriptionLogging(t *testing.T) {
	natsURL := "nats://localhost:4222"

	// Capture slog output.
	var buf bytes.Buffer
	logger := slog.New(slog.NewTextHandler(&buf, nil))
	original := slog.Default()
	slog.SetDefault(logger)
	defer slog.SetDefault(original)

	nc, err := natsclient.Connect(natsURL, 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	if err := nc.SubscribeTelemetry(); err != nil {
		t.Fatalf("failed to subscribe to telemetry: %v", err)
	}

	// Connect a raw NATS client to publish telemetry.
	rawNC, err := nats.Connect(natsURL)
	if err != nil {
		t.Fatalf("failed to connect raw NATS client: %v", err)
	}
	defer rawNC.Close()

	telemetryJSON := `{"speed": 60, "location": {"lat": 48.137, "lon": 11.575}}`
	if err := rawNC.Publish("vehicles.VIN12345.telemetry", []byte(telemetryJSON)); err != nil {
		t.Fatalf("failed to publish telemetry: %v", err)
	}
	rawNC.Flush()

	time.Sleep(100 * time.Millisecond) // allow subscription processing

	logOutput := buf.String()
	if !strings.Contains(logOutput, "VIN12345") {
		t.Errorf("expected log to contain 'VIN12345', got: %s", logOutput)
	}
	if !strings.Contains(logOutput, "telemetry") {
		t.Errorf("expected log to contain 'telemetry', got: %s", logOutput)
	}
}
