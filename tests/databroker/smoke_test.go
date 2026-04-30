package databroker_test

import (
	"net"
	"os/exec"
	"testing"
	"time"
)

// ensureDatabrokerRunning starts the kuksa-databroker container if it is not
// already reachable via TCP. It waits up to 30 seconds for the container to
// become ready. On success, it registers a t.Cleanup that runs compose down
// only if this function was the one that started the container (to avoid
// tearing down a container started by another test or by the developer).
// If podman is unavailable, the test is skipped.
func ensureDatabrokerRunning(t *testing.T) {
	t.Helper()

	// If TCP is already reachable, the container is running — nothing to do.
	conn, err := net.DialTimeout("tcp", tcpTarget, 2*time.Second)
	if err == nil {
		conn.Close()
		return
	}

	// Podman must be available to start the container.
	skipIfPodmanUnavailable(t)

	dir := deploymentsDir(t)

	// Start the container in detached mode.
	cmd := exec.Command("podman", "compose", "up", "-d", "kuksa-databroker")
	cmd.Dir = dir
	out, upErr := cmd.CombinedOutput()
	if upErr != nil {
		t.Fatalf("failed to start databroker container: %v\n%s", upErr, string(out))
	}

	// Register cleanup to tear down what we started.
	t.Cleanup(func() {
		composeDown(t)
	})

	// Wait for the TCP port to become reachable (up to 30s).
	deadline := time.Now().Add(30 * time.Second)
	for time.Now().Before(deadline) {
		c, dialErr := net.DialTimeout("tcp", tcpTarget, 1*time.Second)
		if dialErr == nil {
			c.Close()
			return
		}
		time.Sleep(1 * time.Second)
	}
	t.Fatalf("databroker TCP port %s not reachable after 30s", tcpTarget)
}

// TestSmokeHealthCheck verifies that the DATA_BROKER container starts and
// accepts TCP connections, and that a metadata query returns populated entries.
// Test Spec: TS-02-SMOKE-1
// Requirement: 02-REQ-1.1, 02-REQ-2.1
func TestSmokeHealthCheck(t *testing.T) {
	ensureDatabrokerRunning(t)

	_, client := dialTCP(t)

	// Query metadata for a known standard signal.
	md := listMetadataOrFail(t, client, "Vehicle.Speed")
	if len(md) == 0 {
		t.Fatal("health check failed: ListMetadata for Vehicle.Speed returned no entries")
	}

	// Verify the metadata entry is populated (not just an empty placeholder).
	entry := md[0]
	if entry.DataType == 0 {
		t.Error("health check: metadata entry for Vehicle.Speed has unset DataType")
	}
	t.Logf("health check passed: Vehicle.Speed metadata id=%d dataType=%v", entry.Id, entry.DataType)
}

// TestSmokeFullSignalInventory verifies that all 8 expected VSS signals (5
// standard + 3 custom) are present in the DATA_BROKER metadata after startup.
// Test Spec: TS-02-SMOKE-2
// Requirement: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestSmokeFullSignalInventory(t *testing.T) {
	ensureDatabrokerRunning(t)

	_, client := dialTCP(t)

	var missing []string
	foundCount := 0

	for _, sig := range allSignals {
		t.Run(sig.Path, func(t *testing.T) {
			md := listMetadataOrFail(t, client, sig.Path)
			if len(md) == 0 {
				t.Errorf("missing signal: %s", sig.Path)
				missing = append(missing, sig.Path)
				return
			}
			foundCount++
			t.Logf("found signal %s (dataType=%v)", sig.Path, md[0].DataType)
		})
	}

	// Verify the total count at the end. The subtests above already report
	// individual failures, but this gives a summary assertion.
	if len(missing) > 0 {
		t.Errorf("signal inventory incomplete: found %d/8, missing: %v",
			foundCount, missing)
	} else {
		t.Logf("full signal inventory: %d/%d signals present", len(allSignals), len(allSignals))
	}

	// Belt-and-suspenders: verify foundCount explicitly. The allSignals slice
	// should contain exactly 8 entries.
	if len(allSignals) != 8 {
		t.Errorf("allSignals has %d entries, expected 8 (test data issue)", len(allSignals))
	}
	// Log the inventory for debugging.
	for _, sig := range allSignals {
		t.Logf("  %s (%v)", sig.Path, sig.DataType)
	}
}
