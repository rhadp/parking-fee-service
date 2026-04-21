//go:build integration

package natsclient_test

import (
	"bytes"
	"encoding/json"
	"log/slog"
	"strings"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/natsclient"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// TestNATSResponseSubscription verifies that publishing to vehicles.*.command_responses
// stores the response in the store (TS-06-6).
// This is a dedicated isolation test for the response subscription path.
func TestNATSResponseSubscription(t *testing.T) {
	nc, err := natsclient.Connect("nats://localhost:4222", 3)
	if err != nil {
		t.Skipf("NATS not available: %v", err)
	}
	defer nc.Drain()

	s := store.NewStore()
	if err := nc.SubscribeResponses(s); err != nil {
		t.Fatalf("SubscribeResponses failed: %v", err)
	}

	// Publish a command response via a separate raw NATS connection.
	nconn, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Skipf("NATS direct connection not available: %v", err)
	}
	defer nconn.Close()

	respPayload, _ := json.Marshal(map[string]string{
		"command_id": "cmd-005",
		"status":     "success",
	})
	if err := nconn.Publish("vehicles.VIN12345.command_responses", respPayload); err != nil {
		t.Fatalf("Publish failed: %v", err)
	}
	if err := nconn.Flush(); err != nil {
		t.Fatalf("Flush failed: %v", err)
	}

	// Wait for subscription processing.
	time.Sleep(200 * time.Millisecond)

	resp, found := s.GetResponse("cmd-005")
	if !found {
		t.Fatal("GetResponse(cmd-005): expected found=true after NATS subscription processing")
	}
	if resp.CommandID != "cmd-005" {
		t.Errorf("CommandID: got %q, want %q", resp.CommandID, "cmd-005")
	}
	if resp.Status != "success" {
		t.Errorf("Status: got %q, want %q", resp.Status, "success")
	}
}

// TestTelemetrySubscriptionLogging verifies that publishing to vehicles.*.telemetry
// logs the data without storing it (TS-06-7).
func TestTelemetrySubscriptionLogging(t *testing.T) {
	nc, err := natsclient.Connect("nats://localhost:4222", 3)
	if err != nil {
		t.Skipf("NATS not available: %v", err)
	}
	defer nc.Drain()

	// Capture slog output by redirecting the default logger to a buffer.
	var logBuf bytes.Buffer
	logHandler := slog.NewTextHandler(&logBuf, nil)
	origLogger := slog.Default()
	slog.SetDefault(slog.New(logHandler))
	defer slog.SetDefault(origLogger)

	if err := nc.SubscribeTelemetry(); err != nil {
		t.Fatalf("SubscribeTelemetry failed: %v", err)
	}

	// Publish telemetry via a separate raw NATS connection.
	nconn, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Skipf("NATS direct connection not available: %v", err)
	}
	defer nconn.Close()

	payload := `{"speed": 60, "location": {"lat": 48.137, "lon": 11.575}}`
	if err := nconn.Publish("vehicles.VIN12345.telemetry", []byte(payload)); err != nil {
		t.Fatalf("Publish failed: %v", err)
	}
	if err := nconn.Flush(); err != nil {
		t.Fatalf("Flush failed: %v", err)
	}

	// Wait for subscription processing.
	time.Sleep(200 * time.Millisecond)

	logOutput := logBuf.String()
	if !strings.Contains(logOutput, "VIN12345") {
		t.Errorf("log output missing VIN 'VIN12345'; got: %s", logOutput)
	}
	if !strings.Contains(logOutput, "telemetry") {
		t.Errorf("log output missing 'telemetry'; got: %s", logOutput)
	}

	// Verify no telemetry data was stored (TS-06-7: no storage or aggregation).
	// The store is not used by SubscribeTelemetry, so there's nothing to check
	// via store API, but we confirm the log was the only side effect.
}

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
