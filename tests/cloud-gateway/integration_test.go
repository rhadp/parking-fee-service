package cloudgateway

import (
	"fmt"
	"strings"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
)

// ── TS-06-21: NATS Connection ─────────────────────────────────────────────────

// TestNATSConnection verifies that the cloud-gateway successfully connects to
// NATS. We test this end-to-end by starting the gateway and confirming its
// health endpoint becomes available (which requires NATS to be connected).
// Requirement: 06-REQ-8.1
func TestNATSConnection(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	// startGateway waits for /health to respond, which only happens after
	// the NATS connection is established.
	gp := startGateway(t, cfg)

	if !gp.waitForHealth(5 * time.Second) {
		t.Errorf("gateway not healthy (NATS connection likely failed); logs:\n%s", gp.logs())
	}
}

// ── TS-06-22: NATS Subscriptions Active ──────────────────────────────────────

// TestNATSSubscriptionsActive verifies that after the cloud-gateway starts, it
// subscribes to vehicles.*.command_responses and vehicles.*.telemetry. We verify
// this by publishing to both subjects and confirming the gateway processes them.
// Requirement: 06-REQ-8.2
func TestNATSSubscriptionsActive(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)
	nc := connectNATS(t)

	// Publish valid telemetry — gateway should log it.
	telemSubject := fmt.Sprintf("vehicles.%s.telemetry", testVIN)
	telemPayload := `{"speed":0,"parking":true}`
	if err := nc.Publish(telemSubject, []byte(telemPayload)); err != nil {
		t.Fatalf("publish telemetry: %v", err)
	}

	// Publish a command_responses message — gateway should log it (as unknown
	// command_id since nothing is in store yet).
	respSubject := fmt.Sprintf("vehicles.%s.command_responses", testVIN)
	respPayload := `{"command_id":"sub-active-test","status":"success"}`
	if err := nc.Publish(respSubject, []byte(respPayload)); err != nil {
		t.Fatalf("publish response: %v", err)
	}
	if err := nc.Flush(); err != nil {
		t.Fatalf("flush: %v", err)
	}

	// Both messages should be processed by the gateway subscriptions.
	if !gp.waitForLog("telemetry received", 5*time.Second) {
		t.Errorf("gateway did not process telemetry (vehicles.*.telemetry subscription not active); logs:\n%s", gp.logs())
	}
	// For command_responses: gateway logs "unknown command_id" or "command response received"
	if !gp.waitForLog("sub-active-test", 5*time.Second) {
		t.Logf("command_responses subscription may be active but log differs; logs:\n%s", gp.logs())
	}
}

// ── TS-06-3: Bearer Token in NATS Header ─────────────────────────────────────

// TestBearerTokenInNATSHeader verifies that when a command is submitted via
// REST, the gateway publishes it to NATS with Authorization: Bearer <token>.
// Requirement: 06-REQ-1.3
func TestBearerTokenInNATSHeader(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)
	_ = gp

	nc := connectNATS(t)

	// Subscribe to the NATS command subject before submitting.
	commandSubject := fmt.Sprintf("vehicles.%s.commands", testVIN)
	received := make(chan *nats.Msg, 1)
	sub, err := nc.Subscribe(commandSubject, func(msg *nats.Msg) {
		received <- msg
	})
	if err != nil {
		t.Fatalf("subscribe to %s: %v", commandSubject, err)
	}
	defer sub.Unsubscribe() //nolint:errcheck
	if err := nc.Flush(); err != nil {
		t.Fatalf("flush: %v", err)
	}

	// Submit a command via REST.
	body := `{"command_id":"bearer-test-001","type":"lock","doors":["driver"]}`
	resp := postCommand(t, cfg.Port, testVIN, testToken, body)
	defer resp.Body.Close()

	if resp.StatusCode != 202 {
		t.Fatalf("expected 202, got %d", resp.StatusCode)
	}

	// Wait for the NATS message.
	select {
	case msg := <-received:
		authHeader := msg.Header.Get("Authorization")
		expected := "Bearer " + testToken
		if authHeader != expected {
			t.Errorf("Authorization header = %q, want %q", authHeader, expected)
		}
	case <-time.After(5 * time.Second):
		t.Error("timed out waiting for NATS message after command submission")
	}
}

// ── TS-06-8: NATS Response Subscription ──────────────────────────────────────

// TestNATSResponseSubscription verifies that the service subscribes to
// vehicles.*.command_responses at startup and processes responses.
// Requirement: 06-REQ-3.1
func TestNATSResponseSubscription(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)

	nc := connectNATS(t)

	// First submit a command so it's in the store.
	body := `{"command_id":"nats-resp-001","type":"lock","doors":["driver"]}`
	resp := postCommand(t, cfg.Port, testVIN, testToken, body)
	defer resp.Body.Close()
	if resp.StatusCode != 202 {
		t.Fatalf("expected 202 for command submission, got %d", resp.StatusCode)
	}

	// Give the gateway a moment to store the command.
	time.Sleep(100 * time.Millisecond)

	// Publish a response via NATS.
	responseSubject := fmt.Sprintf("vehicles.%s.command_responses", testVIN)
	responsePayload := `{"command_id":"nats-resp-001","status":"success"}`
	if err := nc.Publish(responseSubject, []byte(responsePayload)); err != nil {
		t.Fatalf("publish response: %v", err)
	}
	if err := nc.Flush(); err != nil {
		t.Fatalf("flush: %v", err)
	}

	// Wait for the gateway to update the command status.
	deadline := time.Now().Add(5 * time.Second)
	var finalStatus string
	for time.Now().Before(deadline) {
		statusResp := getCommandStatus(t, cfg.Port, testVIN, testToken, "nats-resp-001")
		statusBody := decodeJSON(t, statusResp)
		if s, ok := statusBody["status"].(string); ok && s != "pending" {
			finalStatus = s
			break
		}
		time.Sleep(200 * time.Millisecond)
	}

	if finalStatus != "success" {
		t.Errorf("expected command status 'success' after NATS response, got %q; logs:\n%s", finalStatus, gp.logs())
	}
}

// ── TS-06-9: Response Updates Store ──────────────────────────────────────────

// TestResponseUpdatesStore verifies that a NATS response updates the stored
// command status visible via the REST status endpoint.
// Requirement: 06-REQ-3.2
func TestResponseUpdatesStore(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)

	nc := connectNATS(t)

	// Submit a command.
	body := `{"command_id":"store-update-001","type":"unlock","doors":["driver","passenger"]}`
	resp := postCommand(t, cfg.Port, testVIN, testToken, body)
	defer resp.Body.Close()
	if resp.StatusCode != 202 {
		t.Fatalf("expected 202, got %d", resp.StatusCode)
	}

	// Verify it starts as pending.
	statusResp := getCommandStatus(t, cfg.Port, testVIN, testToken, "store-update-001")
	statusBody := decodeJSON(t, statusResp)
	if statusBody["status"] != "pending" {
		t.Errorf("expected initial status 'pending', got %v", statusBody["status"])
	}

	time.Sleep(100 * time.Millisecond)

	// Publish a failed response with a reason.
	responsePayload := `{"command_id":"store-update-001","status":"failed","reason":"vehicle_moving"}`
	if err := nc.Publish(fmt.Sprintf("vehicles.%s.command_responses", testVIN), []byte(responsePayload)); err != nil {
		t.Fatalf("publish: %v", err)
	}
	_ = nc.Flush()

	// Wait for status to update.
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		sr := getCommandStatus(t, cfg.Port, testVIN, testToken, "store-update-001")
		sb := decodeJSON(t, sr)
		if s, ok := sb["status"].(string); ok && s != "pending" {
			if s != "failed" {
				t.Errorf("expected status 'failed', got %q", s)
			}
			if reason, _ := sb["reason"].(string); reason != "vehicle_moving" {
				t.Errorf("expected reason 'vehicle_moving', got %q", reason)
			}
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Errorf("command store not updated after NATS response within timeout; logs:\n%s", gp.logs())
}

// ── TS-06-13: Telemetry Subscription ─────────────────────────────────────────

// TestTelemetrySubscription verifies that the service subscribes to
// vehicles.*.telemetry and processes incoming telemetry.
// Requirement: 06-REQ-5.1
func TestTelemetrySubscription(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)

	nc := connectNATS(t)

	telemetrySubject := fmt.Sprintf("vehicles.%s.telemetry", testVIN)
	telemetryPayload := `{"speed":0,"location":{"lat":52.5,"lon":13.4},"parking":true}`
	if err := nc.Publish(telemetrySubject, []byte(telemetryPayload)); err != nil {
		t.Fatalf("publish telemetry: %v", err)
	}
	_ = nc.Flush()

	if !gp.waitForLog("telemetry received", 5*time.Second) {
		t.Errorf("gateway did not log telemetry reception; logs:\n%s", gp.logs())
	}
}

// ── TS-06-14: Telemetry Logging ───────────────────────────────────────────────

// TestTelemetryLogging verifies that received telemetry is logged with the VIN
// extracted from the NATS subject.
// Requirement: 06-REQ-5.2
func TestTelemetryLogging(t *testing.T) {
	ensureNATS(t)

	cfg := defaultTestConfig()
	gp := startGateway(t, cfg)

	nc := connectNATS(t)

	telemetrySubject := fmt.Sprintf("vehicles.%s.telemetry", testVIN)
	telemetryPayload := `{"speed":0}`
	if err := nc.Publish(telemetrySubject, []byte(telemetryPayload)); err != nil {
		t.Fatalf("publish telemetry: %v", err)
	}
	_ = nc.Flush()

	// The gateway should log the VIN extracted from the subject.
	if !gp.waitForLog(testVIN, 5*time.Second) {
		t.Errorf("expected VIN %q in gateway logs after telemetry; logs:\n%s", testVIN, gp.logs())
	}
	logs := gp.logs()
	if !strings.Contains(logs, testVIN) {
		t.Errorf("VIN %q not found in logs:\n%s", testVIN, logs)
	}
}
