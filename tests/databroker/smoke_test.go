// Smoke tests for the DATA_BROKER component.
// Smoke tests provide fast CI/CD verification. TestSmokeHealthCheck will attempt to start
// the kuksa-databroker container via Podman Compose if the TCP port is not already reachable,
// wait for it to become ready, and verify basic connectivity. TestSmokeFullSignalInventory
// checks that all 8 expected VSS signals are present.
//
// Both tests skip gracefully when Podman or the databroker is unavailable.
package databroker

import (
	"net"
	"os/exec"
	"strings"
	"testing"
	"time"
)

// allExpectedSignals is the complete list of VSS signals the DATA_BROKER must expose:
// 5 standard VSS v5.1 signals (built-in) and 3 custom overlay signals.
var allExpectedSignals = []struct {
	path     string
	typeHint string // substring expected in the ListMetadata response (case-insensitive)
}{
	// Standard VSS signals (built-in)
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", "bool"},
	{"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", "bool"},
	{"Vehicle.CurrentLocation.Latitude", "double"},
	{"Vehicle.CurrentLocation.Longitude", "double"},
	{"Vehicle.Speed", "float"},
	// Custom overlay signals
	{"Vehicle.Parking.SessionActive", "bool"},
	{"Vehicle.Command.Door.Lock", "string"},
	{"Vehicle.Command.Door.Response", "string"},
}

// waitForPort blocks until the TCP address is reachable or the deadline expires.
// Returns true if the port became reachable within the timeout.
func waitForPort(addr string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 500*time.Millisecond)
		if err == nil {
			conn.Close()
			return true
		}
		time.Sleep(500 * time.Millisecond)
	}
	return false
}

// TestSmokeHealthCheck is a quick CI smoke test for the DATA_BROKER.
// If the TCP port is already reachable it runs immediately. Otherwise it attempts to start
// the kuksa-databroker container via Podman Compose, waits up to 10 s for the port, verifies
// gRPC connectivity, and tears down the container via t.Cleanup.
//
// Test Spec: TS-02-SMOKE-1
// Requirements: 02-REQ-1.1, 02-REQ-2.1
func TestSmokeHealthCheck(t *testing.T) {
	skipIfGrpcurlMissing(t)

	containerStarted := false

	// Fast path: container is already running.
	conn, err := net.DialTimeout("tcp", tcpAddr, 2*time.Second)
	if err != nil {
		// Slow path: try to start the container.
		skipIfPodmanNotRunning(t)

		t.Log("TCP port 55556 not reachable; starting kuksa-databroker via podman compose...")
		out, startErr := runPodmanCompose(t, "up", "-d", "kuksa-databroker")
		if startErr != nil {
			t.Skipf("podman compose up kuksa-databroker failed: %v\noutput: %s", startErr, out)
		}
		containerStarted = true

		// Register cleanup to bring down the container after the test.
		t.Cleanup(func() {
			t.Log("TestSmokeHealthCheck cleanup: running podman compose down...")
			if out, err := runPodmanCompose(t, "down"); err != nil {
				t.Logf("podman compose down failed: %v\noutput: %s", err, out)
			}
		})

		// Wait up to 10 s for the TCP port to become reachable.
		t.Log("Waiting up to 10 s for DATA_BROKER TCP port 55556...")
		if !waitForPort(tcpAddr, 10*time.Second) {
			t.Fatalf("DATA_BROKER TCP port 55556 did not become reachable within 10 s after podman compose up")
		}
		t.Log("DATA_BROKER TCP port is reachable.")
	} else {
		conn.Close()
		t.Log("DATA_BROKER TCP port 55556 is already reachable.")
	}

	_ = containerStarted

	// Verify gRPC connectivity by fetching metadata for a well-known standard signal.
	out := grpcurlTCP(t, "GetValue", `{"signal_id": {"path": "Vehicle.Speed"}}`)
	if out == "" {
		t.Fatalf("expected non-empty gRPC response from GetValue(Vehicle.Speed) but got empty output")
	}
	t.Logf("DATA_BROKER health check passed; GetValue response: %s", strings.TrimSpace(out))
}

// TestSmokeFullSignalInventory verifies that all 8 VSS signals (5 standard + 3 custom overlay)
// are present and accessible in the DATA_BROKER. Missing signals are reported by name.
//
// Test Spec: TS-02-SMOKE-2
// Requirements: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestSmokeFullSignalInventory(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	missing := []string{}

	for _, sig := range allExpectedSignals {
		t.Run(sig.path, func(t *testing.T) {
			reqJSON := `{"root": "` + sig.path + `"}`
			// Use grpcurlTCP but don't fatal on error — collect missing signals instead.
			out := grpcurlTCPReport(t, "ListMetadata", reqJSON)
			outLower := strings.ToLower(out)

			if !strings.Contains(out, sig.path) {
				t.Errorf("signal %q not found in ListMetadata response\noutput: %s", sig.path, out)
				missing = append(missing, sig.path)
				return
			}
			if !strings.Contains(outLower, sig.typeHint) {
				t.Errorf("signal %q found but type hint %q absent in response\noutput: %s",
					sig.path, sig.typeHint, out)
			}
		})
	}

	if len(missing) > 0 {
		t.Errorf("DATA_BROKER is missing %d signal(s): %s", len(missing), strings.Join(missing, ", "))
	} else {
		t.Logf("All %d expected VSS signals are present in the DATA_BROKER.", len(allExpectedSignals))
	}
}

// grpcurlTCPReport is like grpcurlTCP but logs the error and returns the output rather than
// calling t.Fatal, allowing the caller to handle the failure gracefully (e.g., collect missing
// signals without stopping the entire test).
func grpcurlTCPReport(t *testing.T, method, reqJSON string) string {
	t.Helper()
	args := []string{"-plaintext", "-d", reqJSON, tcpAddr, grpcService + "/" + method}
	cmd := exec.Command("grpcurl", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Logf("grpcurl TCP %s returned error (signal may be missing): %v\noutput: %s", method, err, out)
	}
	return string(out)
}
