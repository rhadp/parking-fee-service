//go:build integration

package natsclient_test

import (
	"bytes"
	"encoding/json"
	"log/slog"
	"testing"
	"time"

	"github.com/nats-io/nats.go"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
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

// TS-06-6: NATS Response Subscription
// Requirements: 06-REQ-5.1, 06-REQ-5.2
// The service subscribes to command responses on NATS and stores them.
func TestNATSResponseSubscription(t *testing.T) {
	nc, err := natsclient.Connect("nats://localhost:4222", 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	s := store.NewStore()

	// Subscribe to responses.
	if err := nc.SubscribeResponses(s); err != nil {
		t.Fatalf("failed to subscribe to responses: %v", err)
	}

	// Connect a raw NATS publisher to simulate a vehicle response.
	rawNC, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("failed to connect raw NATS client: %v", err)
	}
	defer rawNC.Close()

	// Publish a command response.
	respData, _ := json.Marshal(model.CommandResponse{
		CommandID: "cmd-005",
		Status:    "success",
	})
	if err := rawNC.Publish("vehicles.VIN12345.command_responses", respData); err != nil {
		t.Fatalf("failed to publish response: %v", err)
	}
	rawNC.Flush()

	// Allow subscription processing time.
	time.Sleep(200 * time.Millisecond)

	// Verify the response was stored.
	resp, found := s.GetResponse("cmd-005")
	if !found {
		t.Fatal("expected response for cmd-005 to be stored, but not found")
	}
	if resp.Status != "success" {
		t.Errorf("expected status 'success', got '%s'", resp.Status)
	}
}

// TS-06-7: Telemetry Subscription Logging
// Requirement: 06-REQ-5.3
// The service subscribes to telemetry on NATS and logs it without storing.
func TestTelemetrySubscriptionLogging(t *testing.T) {
	// Capture log output by setting a custom default logger.
	var logBuf bytes.Buffer
	logger := slog.New(slog.NewTextHandler(&logBuf, nil))
	slog.SetDefault(logger)

	nc, err := natsclient.Connect("nats://localhost:4222", 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	// Subscribe to telemetry.
	if err := nc.SubscribeTelemetry(); err != nil {
		t.Fatalf("failed to subscribe to telemetry: %v", err)
	}

	// Connect a raw NATS publisher to simulate telemetry.
	rawNC, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("failed to connect raw NATS client: %v", err)
	}
	defer rawNC.Close()

	// Publish telemetry data.
	telemetryData := `{"speed": 60, "location": {"lat": 48.137, "lon": 11.575}}`
	if err := rawNC.Publish("vehicles.VIN12345.telemetry", []byte(telemetryData)); err != nil {
		t.Fatalf("failed to publish telemetry: %v", err)
	}
	rawNC.Flush()

	// Allow subscription processing time.
	time.Sleep(200 * time.Millisecond)

	logs := logBuf.String()

	if len(logs) == 0 {
		t.Error("expected log output for telemetry, got none")
	}

	if !bytes.Contains(logBuf.Bytes(), []byte("VIN12345")) {
		t.Errorf("expected log to contain 'VIN12345', got: %s", logs)
	}

	if !bytes.Contains(logBuf.Bytes(), []byte("telemetry")) {
		t.Errorf("expected log to contain 'telemetry', got: %s", logs)
	}
}
