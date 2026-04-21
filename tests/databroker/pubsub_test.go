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

// countOccurrences returns how many times substr appears in s (case-insensitive).
func countOccurrences(s, substr string) int {
	s = strings.ToLower(s)
	substr = strings.ToLower(substr)
	count := 0
	for i := 0; i+len(substr) <= len(s); i++ {
		if s[i:i+len(substr)] == substr {
			count++
		}
	}
	return count
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

// TestPropertySubscriptionDelivery verifies that for any signal, subscribing and then
// setting a new value delivers the update to the subscriber exactly once.
// Per TS-02-P4 pseudocode, after receiving the expected update, a second Recv with a
// short timeout must return TIMEOUT (no additional update). The grpcurl approach verifies
// this by using a distinctive marker value and counting its occurrences in the captured
// subscription output — the marker should appear exactly once.
// Test Spec: TS-02-P4
// Requirement: 02-REQ-10.1
func TestPropertySubscriptionDelivery(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	// Each signal is tested with a type-appropriate value and a distinctive substring
	// to verify exactly-once delivery.
	type subCase struct {
		signal  string
		pubReq  string // full PublishValueRequest JSON
		checkIn string // substring expected in the subscription output (case-insensitive)
	}

	cases := []subCase{
		{
			signal:  "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
			pubReq:  `{"signal_id": {"path": "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"}, "value": {"bool_value": true}}`,
			checkIn: "IsLocked",
		},
		{
			signal:  "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
			pubReq:  `{"signal_id": {"path": "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"}, "value": {"bool_value": true}}`,
			checkIn: "IsOpen",
		},
		{
			signal:  "Vehicle.CurrentLocation.Latitude",
			pubReq:  `{"signal_id": {"path": "Vehicle.CurrentLocation.Latitude"}, "value": {"double_value": 52.5200}}`,
			checkIn: "52",
		},
		{
			signal:  "Vehicle.CurrentLocation.Longitude",
			pubReq:  `{"signal_id": {"path": "Vehicle.CurrentLocation.Longitude"}, "value": {"double_value": 13.4050}}`,
			checkIn: "13",
		},
		{
			signal:  "Vehicle.Speed",
			pubReq:  `{"signal_id": {"path": "Vehicle.Speed"}, "value": {"float_value": 120.0}}`,
			checkIn: "120",
		},
		{
			signal:  "Vehicle.Parking.SessionActive",
			pubReq:  `{"signal_id": {"path": "Vehicle.Parking.SessionActive"}, "value": {"bool_value": true}}`,
			checkIn: "SessionActive",
		},
		{
			signal:  "Vehicle.Command.Door.Lock",
			pubReq:  `{"signal_id": {"path": "Vehicle.Command.Door.Lock"}, "value": {"string_value": "sub-delivery-test"}}`,
			checkIn: "sub-delivery-test",
		},
		{
			signal:  "Vehicle.Command.Door.Response",
			pubReq:  `{"signal_id": {"path": "Vehicle.Command.Door.Response"}, "value": {"string_value": "sub-resp-test"}}`,
			checkIn: "sub-resp-test",
		},
	}

	const target = "localhost:55556"

	for _, tc := range cases {
		t.Run(tc.signal, func(t *testing.T) {
			// Use a longer capture window (3s) split into:
			//   - 300ms: subscriber setup
			//   - publish
			//   - remaining time: wait for delivery + verify no extra delivery
			type result struct{ out string }
			ch := make(chan result, 1)
			go func() {
				// 3s total: enough to capture the update and verify no more arrive.
				out := subscribeAndCapture(t, target, tc.signal, 3*time.Second)
				ch <- result{out}
			}()

			// Allow subscriber to establish before publishing.
			time.Sleep(300 * time.Millisecond)

			// Publish the value exactly once.
			grpcurlTCP(t, "PublishValue", tc.pubReq)

			// Wait for the subscriber output.
			select {
			case r := <-ch:
				outLower := strings.ToLower(r.out)
				checkLower := strings.ToLower(tc.checkIn)

				if !strings.Contains(outLower, checkLower) {
					t.Errorf("subscriber did not receive expected value %q for %s\noutput: %s",
						tc.checkIn, tc.signal, r.out)
				}

				// Exactly-once verification: the subscription stream output from
				// grpcurl contains one JSON object per update. Count how many times
				// the signal path appears in the output (each update block contains
				// the signal path once). More than 1 indicates duplicate delivery.
				pathCount := countOccurrences(r.out, tc.signal)
				if pathCount > 1 {
					t.Errorf("expected exactly 1 update for %s but got %d occurrences in subscription output\noutput: %s",
						tc.signal, pathCount, r.out)
				}
			case <-time.After(6 * time.Second):
				t.Errorf("timed out waiting for subscription update for %s", tc.signal)
			}
		})
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
