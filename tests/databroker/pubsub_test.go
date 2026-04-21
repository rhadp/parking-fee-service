// Subscription and pub/sub integration tests for the DATA_BROKER.
// Tests verify that Subscribe delivers notifications when signal values change.
// All tests skip when the DATA_BROKER container is unavailable or grpcurl is not installed.
package databroker

import (
	"context"
	"os/exec"
	"strings"
	"testing"
	"time"
)

// subscribeAndCapture starts a grpcurl Subscribe call on the given transport (TCP address
// or unix://path) for the named signal. It runs the subscriber for duration d, then
// cancels and returns the captured output. This helper is used by subscription tests
// to observe update notifications.
func subscribeAndCapture(t *testing.T, target, signalPath string, d time.Duration) string {
	t.Helper()
	reqJSON := `{"entries": [{"signal_id": {"path": "` + signalPath + `"}}]}`
	args := []string{"-plaintext", "-d", reqJSON, target, grpcService + "/Subscribe"}

	ctx, cancel := context.WithTimeout(context.Background(), d)
	defer cancel()

	cmd := exec.CommandContext(ctx, "grpcurl", args...)
	out, _ := cmd.CombinedOutput() // ignore error; context cancellation causes non-zero exit
	return string(out)
}

// TestSubscriptionViaTCP verifies that a TCP subscriber receives update notifications
// when a signal value changes.
// Test Spec: TS-02-10
// Requirement: 02-REQ-10.1
func TestSubscriptionViaTCP(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	const signal = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	const target = "localhost:55556"

	// Start subscriber in a goroutine; give it 5 seconds to receive updates.
	type result struct{ out string }
	ch := make(chan result, 1)
	go func() {
		out := subscribeAndCapture(t, target, signal, 5*time.Second)
		ch <- result{out}
	}()

	// Allow subscriber to establish before publishing.
	time.Sleep(300 * time.Millisecond)

	// Publish a value change via a second TCP connection.
	grpcurlTCP(t, "PublishValue",
		`{"signal_id": {"path": "`+signal+`"}, "value": {"bool_value": true}}`)

	// Wait for the subscriber to capture the update.
	select {
	case r := <-ch:
		if !strings.Contains(strings.ToLower(r.out), "true") {
			t.Errorf("subscriber did not receive bool_value true for %s\noutput: %s", signal, r.out)
		}
	case <-time.After(6 * time.Second):
		t.Errorf("timed out waiting for subscription update for %s", signal)
	}
}

// TestSubscriptionCrossTransport verifies that a UDS subscriber receives update notifications
// when a signal is set via TCP.
// Test Spec: TS-02-11
// Requirements: 02-REQ-10.1, 02-REQ-4.1
func TestSubscriptionCrossTransport(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)
	sockPath := effectiveUDSSocket(t)

	const signal = "Vehicle.Parking.SessionActive"
	udsTarget := "unix://" + sockPath

	// Start a subscriber on UDS.
	type result struct{ out string }
	ch := make(chan result, 1)
	go func() {
		out := subscribeAndCapture(t, udsTarget, signal, 5*time.Second)
		ch <- result{out}
	}()

	// Allow subscriber to establish.
	time.Sleep(300 * time.Millisecond)

	// Publish a value change via TCP.
	grpcurlTCP(t, "PublishValue",
		`{"signal_id": {"path": "`+signal+`"}, "value": {"bool_value": true}}`)

	// Wait for the UDS subscriber to capture the update.
	select {
	case r := <-ch:
		if !strings.Contains(strings.ToLower(r.out), "true") {
			t.Errorf("UDS subscriber did not receive bool_value true for %s\noutput: %s", signal, r.out)
		}
	case <-time.After(6 * time.Second):
		t.Errorf("timed out waiting for UDS subscription update for %s", signal)
	}
}

// TestPermissiveModeWithArbitraryToken verifies that the DATA_BROKER accepts requests
// even when an invalid/arbitrary authorization token is provided in the metadata.
// In permissive mode, the broker must not reject requests based on token content.
// Test Spec: TS-02-E4
// Requirement: 02-REQ-7.E1
func TestPermissiveModeWithArbitraryToken(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	// Send a GetValue request with a bogus Authorization header; it must still succeed.
	args := []string{
		"-plaintext",
		"-H", "Authorization: Bearer invalid-token-12345",
		"-d", `{"signal_id": {"path": "Vehicle.Speed"}}`,
		tcpAddr,
		grpcService + "/GetValue",
	}
	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected success in permissive mode with arbitrary token, got error: %v\noutput: %s", err, out)
	}
}
