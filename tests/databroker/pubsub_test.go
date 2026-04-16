// Subscription tests — live gRPC pub/sub notification delivery.
//
// All tests in this file require a running DATA_BROKER container and skip
// automatically when it is unavailable.
//
// Subscription tests use grpcurl with a finite timeout (--max-time) to
// receive the first update and then exit.  The tests set a signal value
// from a concurrent goroutine to trigger the notification.
//
// Test Specs: TS-02-10, TS-02-11, TS-02-P4
// Requirements: 02-REQ-10.1
package databroker_test

import (
	"os/exec"
	"strings"
	"testing"
	"time"
)

// runSubscribeTCP starts a grpcurl Subscribe stream against the TCP endpoint
// and returns the combined output once the process exits (due to --max-time or
// natural completion).  The stream terminates after maxWait seconds.
func runSubscribeTCP(t *testing.T, signalPath string, maxWait int) (string, error) {
	t.Helper()
	body := `{"signal_ids":[{"path":"` + signalPath + `"}]}`
	_ = maxWait // callers pass a timeout hint; we use a fixed 5s via --max-time
	cmd := exec.Command("grpcurl",
		"-plaintext",
		"-max-time", "5",
		"-d", body,
		"localhost:55556",
		"kuksa.val.v2.VAL/Subscribe",
	)

	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err := cmd.Run()
	return stdout.String() + stderr.String(), err
}

// runSubscribeUDS starts a grpcurl Subscribe stream against the UDS endpoint.
func runSubscribeUDS(t *testing.T, signalPath string) (string, error) {
	t.Helper()
	body := `{"signal_ids":[{"path":"` + signalPath + `"}]}`
	cmd := exec.Command("grpcurl",
		"-plaintext",
		"-max-time", "5",
		"-d", body,
		udsGrpcTarget(),
		"kuksa.val.v2.VAL/Subscribe",
	)

	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err := cmd.Run()
	return stdout.String() + stderr.String(), err
}

// TestSubscriptionTCP verifies that a TCP subscriber receives a notification
// when a signal value is changed by another client.
//
// Test Spec: TS-02-10
// Requirements: 02-REQ-10.1
func TestSubscriptionTCP(t *testing.T) {
	requireDatabrokerTCP(t)

	signalPath := "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"

	// Reset the signal to false first.
	resetBody := `{"signal_id":{"path":"` + signalPath + `"},"data_point":{"bool":false}}`
	grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", resetBody)

	// Start the subscription in a goroutine.
	type result struct {
		output string
		err    error
	}
	ch := make(chan result, 1)
	go func() {
		out, err := runSubscribeTCP(t, signalPath, 5)
		ch <- result{out, err}
	}()

	// Wait briefly for the subscribe stream to establish, then set the signal.
	time.Sleep(500 * time.Millisecond)
	setBody := `{"signal_id":{"path":"` + signalPath + `"},"data_point":{"bool":true}}`
	grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", setBody)

	// Wait for the subscription to return.
	select {
	case r := <-ch:
		// grpcurl may exit with non-zero due to timeout (--max-time), which is expected.
		// The important thing is that the output contains the updated value.
		if !strings.Contains(r.output, "true") {
			t.Errorf("TCP subscription for %s: expected to see 'true' in update; got: %s", signalPath, r.output)
		}
	case <-time.After(8 * time.Second):
		t.Error("TCP subscription timed out waiting for update notification")
	}
}

// TestSubscriptionCrossTransport verifies that a UDS subscriber receives a
// notification when a signal is set via TCP.
//
// Test Spec: TS-02-11
// Requirements: 02-REQ-10.1, 02-REQ-4.1
func TestSubscriptionCrossTransport(t *testing.T) {
	requireDatabrokerUDS(t)
	requireDatabrokerTCP(t)

	signalPath := "Vehicle.Parking.SessionActive"

	// Reset the signal.
	resetBody := `{"signal_id":{"path":"` + signalPath + `"},"data_point":{"bool":false}}`
	grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", resetBody)

	// Start UDS subscription.
	type result struct {
		output string
		err    error
	}
	ch := make(chan result, 1)
	go func() {
		out, err := runSubscribeUDS(t, signalPath)
		ch <- result{out, err}
	}()

	// Wait for subscription to establish, then set via TCP.
	time.Sleep(500 * time.Millisecond)
	setBody := `{"signal_id":{"path":"` + signalPath + `"},"data_point":{"bool":true}}`
	grpcurlOK(t, "kuksa.val.v2.VAL/PublishValue", setBody)

	select {
	case r := <-ch:
		if !strings.Contains(r.output, "true") {
			t.Errorf("UDS subscription for %s: expected 'true' in update from TCP write; got: %s", signalPath, r.output)
		}
	case <-time.After(8 * time.Second):
		t.Error("UDS subscription timed out waiting for cross-transport update")
	}
}

// TestPermissiveModeWithArbitraryToken verifies that the DATA_BROKER accepts
// requests even when an invalid/arbitrary token is provided in the metadata.
//
// Test Spec: TS-02-E4
// Requirements: 02-REQ-7.E1
func TestPermissiveModeWithArbitraryToken(t *testing.T) {
	requireDatabrokerTCP(t)

	// Pass an invalid token via grpcurl -rpc-header.
	setBody := `{"signal_id":{"path":"Vehicle.Speed"},"data_point":{"float":10.0}}`
	cmd := exec.Command("grpcurl",
		"-plaintext",
		"-rpc-header", "authorization: Bearer invalid-token-12345",
		"-d", setBody,
		"localhost:55556",
		"kuksa.val.v2.VAL/PublishValue",
	)
	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err := cmd.Run()
	combined := stdout.String() + stderr.String()

	if err != nil {
		// In permissive mode, the request MUST succeed even with an invalid token.
		if strings.Contains(combined, "PermissionDenied") || strings.Contains(combined, "Unauthenticated") {
			t.Errorf("DATA_BROKER rejected request with invalid token in permissive mode; output: %s", combined)
		}
		t.Fatalf("PublishValue with invalid token failed unexpectedly: %v\noutput: %s", err, combined)
	}
}
