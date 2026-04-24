package databroker_test

import (
	"net"
	"testing"
	"time"
)

// ensureDatabrokerRunning starts the DATA_BROKER container if it is not
// already reachable on the TCP port. Unlike skipIfTCPUnreachable, this
// function self-bootstraps the container so smoke tests do not depend on
// external setup. It registers a t.Cleanup to tear down the container
// when the test completes.
//
// Skips the test only if Podman itself is unavailable.
func ensureDatabrokerRunning(t *testing.T) {
	t.Helper()

	// If the container is already reachable, nothing to do.
	if conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second); err == nil {
		conn.Close()
		return
	}

	// Podman must be available to start the container.
	skipIfPodmanUnavailable(t)

	// Tear down any leftover container first, then start fresh.
	composeDown(t)

	if err := composeUp(t); err != nil {
		t.Fatalf("failed to start databroker container: %v", err)
	}

	// Register cleanup to tear down the container after this test.
	t.Cleanup(func() {
		composeDown(t)
	})

	// Wait for the TCP port to become reachable (up to 10 seconds).
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", tcpTarget, 1*time.Second)
		if err == nil {
			conn.Close()
			return
		}
		time.Sleep(500 * time.Millisecond)
	}

	t.Fatalf("databroker TCP port %s not reachable within 10s of container start", tcpTarget)
}

// TestSmokeHealthCheck is a quick smoke test that verifies the DATA_BROKER
// container starts and accepts TCP gRPC connections. It self-bootstraps
// the container if not already running.
//
// Test Spec: TS-02-SMOKE-1
// Requirements: 02-REQ-1.1, 02-REQ-2.1
func TestSmokeHealthCheck(t *testing.T) {
	ensureDatabrokerRunning(t)

	client := newTCPClient(t)

	// Verify connectivity by performing a Get request for Vehicle.Speed.
	entry, err := getSignalValue(t, client, "Vehicle.Speed")
	if err != nil {
		t.Fatalf("smoke health check failed: Get returned error: %v", err)
	}
	if entry == nil {
		t.Fatal("smoke health check failed: Get returned nil entry")
	}
	if entry.Path != "Vehicle.Speed" {
		t.Errorf("expected path Vehicle.Speed, got %q", entry.Path)
	}

	// Strengthen assertion: verify the response contains a populated entry
	// (not just that the transport connected). The Path field must be set.
	if entry.Path == "" {
		t.Error("smoke health check: response entry has empty path, expected populated metadata")
	}
}

// TestSmokeFullSignalInventory is a quick smoke test that verifies all 8
// expected VSS signals (5 standard + 3 custom) are present after the
// DATA_BROKER starts. It self-bootstraps the container if not already running.
//
// Test Spec: TS-02-SMOKE-2
// Requirements: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestSmokeFullSignalInventory(t *testing.T) {
	ensureDatabrokerRunning(t)

	client := newTCPClient(t)

	signals := allSignals()
	foundCount := 0
	var missing []string

	for _, sig := range signals {
		t.Run(sig.Path, func(t *testing.T) {
			entry, err := getSignalValue(t, client, sig.Path)
			if err != nil {
				t.Errorf("signal %s not accessible: %v", sig.Path, err)
				return
			}
			if entry == nil {
				t.Errorf("signal %s returned nil entry", sig.Path)
				return
			}
			if entry.Path != sig.Path {
				t.Errorf("expected path %q, got %q", sig.Path, entry.Path)
				return
			}
			foundCount++
		})
	}

	// After all subtests, check that all 8 signals were found.
	if foundCount != 8 {
		for _, sig := range signals {
			entry, err := getSignalValue(t, client, sig.Path)
			if err != nil || entry == nil {
				missing = append(missing, sig.Path)
			}
		}
		t.Errorf("expected 8 signals, found %d; missing: %v", foundCount, missing)
	}
}
