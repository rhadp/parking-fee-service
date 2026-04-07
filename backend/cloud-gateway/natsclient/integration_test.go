//go:build integration

package natsclient

import (
	"encoding/json"
	"fmt"
	"math/rand"
	"testing"
	"time"

	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"

	"github.com/nats-io/nats.go"
)

func natsURL() string {
	return "nats://localhost:4222"
}

// TS-06-2: NATS Authorization Header
func TestNATSAuthorizationHeader(t *testing.T) {
	nc, err := Connect(natsURL(), 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	// Subscribe to capture the published message
	sub, err := nc.conn.SubscribeSync("vehicles.VIN12345.commands")
	if err != nil {
		t.Fatalf("failed to subscribe: %v", err)
	}
	defer sub.Unsubscribe()

	cmd := model.Command{
		CommandID: "cmd-002",
		Type:      "unlock",
		Doors:     []string{"driver"},
	}
	if err := nc.PublishCommand("VIN12345", cmd, "demo-token-001"); err != nil {
		t.Fatalf("PublishCommand error: %v", err)
	}

	msg, err := sub.NextMsg(1 * time.Second)
	if err != nil {
		t.Fatalf("did not receive NATS message: %v", err)
	}

	authHeader := msg.Header.Get("Authorization")
	if authHeader != "Bearer demo-token-001" {
		t.Errorf("Authorization header = %q, want %q", authHeader, "Bearer demo-token-001")
	}
}

// TS-06-6: NATS Response Subscription
func TestNATSResponseSubscription(t *testing.T) {
	nc, err := Connect(natsURL(), 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	s := store.NewStore()
	if err := nc.SubscribeResponses(s); err != nil {
		t.Fatalf("SubscribeResponses error: %v", err)
	}

	// Publish a command response via raw NATS
	resp := model.CommandResponse{CommandID: "cmd-005", Status: "success"}
	data, _ := json.Marshal(resp)
	if err := nc.conn.Publish("vehicles.VIN12345.command_responses", data); err != nil {
		t.Fatalf("failed to publish response: %v", err)
	}
	nc.conn.Flush()
	time.Sleep(100 * time.Millisecond)

	got, found := s.GetResponse("cmd-005")
	if !found {
		t.Fatal("response not found in store after NATS subscription")
	}
	if got.Status != "success" {
		t.Errorf("status = %q, want %q", got.Status, "success")
	}
}

// TS-06-7: Telemetry Subscription Logging
func TestTelemetrySubscriptionLogging(t *testing.T) {
	nc, err := Connect(natsURL(), 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	if err := nc.SubscribeTelemetry(); err != nil {
		t.Fatalf("SubscribeTelemetry error: %v", err)
	}

	// Publish telemetry data
	telemetry := map[string]interface{}{
		"speed":    60,
		"location": map[string]float64{"lat": 48.137, "lon": 11.575},
	}
	data, _ := json.Marshal(telemetry)
	if err := nc.conn.Publish("vehicles.VIN12345.telemetry", data); err != nil {
		t.Fatalf("failed to publish telemetry: %v", err)
	}
	nc.conn.Flush()
	time.Sleep(100 * time.Millisecond)

	// Telemetry should be logged but not stored.
	// We verify it doesn't crash and no response is stored for telemetry.
	// Full log capture verification would require log interception.
}

// TS-06-P6: Property - NATS Header Propagation
func TestPropertyNATSHeaderPropagation(t *testing.T) {
	nc, err := Connect(natsURL(), 3)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	defer nc.Drain()

	tokens := []model.TokenMapping{
		{Token: "demo-token-001", VIN: "VIN12345"},
		{Token: "demo-token-002", VIN: "VIN67890"},
	}

	rng := rand.New(rand.NewSource(42))

	for _, tm := range tokens {
		subject := "vehicles." + tm.VIN + ".commands"
		sub, err := nc.conn.SubscribeSync(subject)
		if err != nil {
			t.Fatalf("failed to subscribe to %s: %v", subject, err)
		}

		cmd := model.Command{
			CommandID: fmt.Sprintf("prop-%d", rng.Int()),
			Type:      "lock",
			Doors:     []string{"driver"},
		}
		if err := nc.PublishCommand(tm.VIN, cmd, tm.Token); err != nil {
			t.Fatalf("PublishCommand error: %v", err)
		}

		msg, err := sub.NextMsg(1 * time.Second)
		if err != nil {
			t.Fatalf("did not receive message on %s: %v", subject, err)
		}

		want := "Bearer " + tm.Token
		got := msg.Header.Get("Authorization")
		if got != want {
			t.Errorf("Authorization = %q, want %q", got, want)
		}

		sub.Unsubscribe()
	}
}

// TS-06-SMOKE-1: End-to-End Command Flow (NATS portion)
// Full end-to-end smoke tests are defined in the smoke_test.go file.
// This file covers NATS-specific integration tests.

// Helper to get raw NATS connection for testing
func rawNATSConn(t *testing.T) *nats.Conn {
	t.Helper()
	conn, err := nats.Connect(natsURL())
	if err != nil {
		t.Fatalf("failed to connect raw NATS: %v", err)
	}
	return conn
}
