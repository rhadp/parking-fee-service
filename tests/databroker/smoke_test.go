package databroker_test

// smoke_test.go — quick CI/CD smoke tests for the DATA_BROKER component.
//
// Smoke tests provide fast go/no-go verification that the DATA_BROKER starts
// correctly and exposes all required signals. They are designed to be run as
// the first gate in a CI pipeline before the full integration test suite.
//
// TestSmokeHealthCheck (TS-02-SMOKE-1):
//   - If the databroker TCP port is already reachable, verifies connectivity
//     and GetServerInfo without managing container lifecycle.
//   - If the port is NOT reachable, requires Podman, brings up the
//     `kuksa-databroker` service via `podman compose up -d`, waits up to 10s
//     for the port to become reachable, verifies GetServerInfo, and tears down
//     via t.Cleanup.
//   - Skips gracefully when neither the port is reachable nor Podman is
//     installed.
//
// TestSmokeFullSignalInventory (TS-02-SMOKE-2):
//   - Queries ListMetadata for all 8 expected signals.
//   - Reports any missing signals by name rather than failing on the first miss,
//     so that a single test run surfaces all gaps at once.
//
// Requirements: 02-REQ-1.1, 02-REQ-2.1, 02-REQ-5.1, 02-REQ-5.2,
//               02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3.

import (
	"net"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// ---- TS-02-SMOKE-1: databroker health check ----

// TestSmokeHealthCheck verifies that the DATA_BROKER starts and accepts TCP
// connections within 10 seconds (TS-02-SMOKE-1, 02-REQ-1.1, 02-REQ-2.1).
//
// If the databroker is already running, this test verifies connectivity
// without touching the container lifecycle.  If it is not running, the test
// attempts to start it via Podman Compose and tears it down in t.Cleanup.
func TestSmokeHealthCheck(t *testing.T) {
	alreadyRunning := isTCPReachable(tcpEndpoint, 1*time.Second)

	if !alreadyRunning {
		// Databroker is not running. We need Podman to start it.
		if _, err := exec.LookPath("podman"); err != nil {
			t.Skip("databroker not reachable and podman not installed; skipping smoke health check")
		}

		root := repoRoot(t)
		deploymentsDir := filepath.Join(root, "deployments")

		// Start the kuksa-databroker service detached.
		upCmd := exec.Command("podman", "compose", "up", "-d", "kuksa-databroker")
		upCmd.Dir = deploymentsDir
		if out, err := upCmd.CombinedOutput(); err != nil {
			t.Fatalf("podman compose up -d kuksa-databroker failed: %v\nOutput:\n%s", err, out)
		}
		t.Log("Started kuksa-databroker via podman compose")

		// Register cleanup to tear down the container when the test finishes.
		t.Cleanup(func() {
			downCmd := exec.Command("podman", "compose", "down")
			downCmd.Dir = deploymentsDir
			if out, err := downCmd.CombinedOutput(); err != nil {
				t.Logf("podman compose down failed (non-fatal): %v\nOutput:\n%s", err, out)
			} else {
				t.Log("Stopped kuksa-databroker via podman compose down")
			}
		})

		// Wait up to 10 seconds for the TCP port to become reachable.
		const deadline = 10 * time.Second
		const pollInterval = 250 * time.Millisecond
		start := time.Now()
		for {
			if isTCPReachable(tcpEndpoint, 1*time.Second) {
				break
			}
			if time.Since(start) >= deadline {
				t.Fatalf("databroker TCP port %s not reachable within %s after podman compose up",
					tcpEndpoint, deadline)
			}
			time.Sleep(pollInterval)
		}
		t.Logf("databroker reachable at %s after %s", tcpEndpoint, time.Since(start).Round(time.Millisecond))
	}

	// At this point the databroker is reachable. Verify grpcurl is available.
	requireGrpcurl(t)

	// Verify GetServerInfo returns a valid response with a version field.
	out := grpcurlTCP(t, "kuksa.val.v2.VAL/GetServerInfo", "{}")
	if strings.TrimSpace(out) == "" {
		t.Error("GetServerInfo returned empty response")
	}
	if !strings.Contains(out, "version") {
		t.Errorf("GetServerInfo response missing 'version' field, got:\n%s", out)
	}
	t.Logf("GetServerInfo response: %s", strings.TrimSpace(out))
}

// isTCPReachable returns true if a TCP connection to addr succeeds within timeout.
func isTCPReachable(addr string, timeout time.Duration) bool {
	conn, err := net.DialTimeout("tcp", addr, timeout)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

// ---- TS-02-SMOKE-2: full signal inventory check ----

// TestSmokeFullSignalInventory verifies that all 8 expected VSS signals are
// present and queryable in the DATA_BROKER (TS-02-SMOKE-2, 02-REQ-5.1,
// 02-REQ-5.2, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3).
//
// Rather than stopping at the first missing signal, this test collects all
// failures and reports them together so that a single run reveals all gaps.
func TestSmokeFullSignalInventory(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	var missing []string

	for _, sig := range allSignals {
		data := `{"root": "` + sig.path + `"}`
		out, err := grpcurlTCPRaw("kuksa.val.v2.VAL/ListMetadata", data)
		if err != nil {
			// Non-zero exit from grpcurl means the signal was not found or an
			// error occurred.  Record the signal as missing.
			missing = append(missing, sig.path)
			t.Logf("MISSING signal %q: %v\nOutput:\n%s", sig.path, err, out)
			continue
		}
		// Signal exists; also verify the expected data type is present.
		if !strings.Contains(out, sig.typeHint) {
			t.Errorf("signal %q present but wrong type: expected %q in metadata, got:\n%s",
				sig.path, sig.typeHint, out)
		}
	}

	if len(missing) > 0 {
		t.Errorf("the following signals are missing from the DATA_BROKER (%d/%d):\n  - %s",
			len(missing), len(allSignals), strings.Join(missing, "\n  - "))
	} else {
		t.Logf("all %d signals present in DATA_BROKER", len(allSignals))
	}
}
