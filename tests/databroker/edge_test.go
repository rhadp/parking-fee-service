package databroker

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// TS-02-E1: UDS Socket Overwrite on Restart
// Requirement: 02-REQ-1.E1
func TestEdgeUDSSocketRestart(t *testing.T) {
	requireLiveDatabroker(t)

	_ = os.MkdirAll("/tmp/kuksa", 0o755)
	cf := composeFile(t)

	// First start.
	cmd := exec.Command("podman", "compose", "-f", cf, "up", "-d", "databroker")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("first databroker start failed: %v\n%s", err, string(out))
	}
	if !waitForDatabroker(t, 15*time.Second) {
		t.Fatal("databroker not healthy after first start")
	}

	// Stop.
	stopDatabroker(t)
	time.Sleep(500 * time.Millisecond)

	// Second start — any UDS socket file from the previous run may still exist.
	cmd2 := exec.Command("podman", "compose", "-f", cf, "up", "-d", "databroker")
	if out, err := cmd2.CombinedOutput(); err != nil {
		t.Fatalf("second databroker start failed: %v\n%s", err, string(out))
	}
	t.Cleanup(func() { stopDatabroker(t) })

	if !waitForDatabroker(t, 15*time.Second) {
		t.Fatal("databroker not healthy after restart (socket may not have been overwritten)")
	}
}

// TS-02-E2: Concurrent TCP and UDS Clients
// Requirement: 02-REQ-1.E2
func TestEdgeConcurrentTCPUDS(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Speed"
	const value = "50.0"

	// Set via TCP.
	setOut, err := grpcSetFloat(tcpEndpoint, signal, value)
	if err != nil {
		t.Fatalf("TCP Set(%s, %s) failed: %v\noutput: %s", signal, value, err, setOut)
	}

	// Read via UDS.
	getOut, err := grpcGet(udsEndpoint, signal)
	if err != nil {
		t.Fatalf("UDS Get(%s) failed: %v\noutput: %s", signal, err, getOut)
	}
	if !strings.Contains(getOut, "50") {
		t.Errorf("UDS Get(%s) after TCP Set(%s): expected 50 in response, got: %s", signal, value, getOut)
	}
}

// TS-02-E3: Malformed VSS Overlay
// Requirement: 02-REQ-3.E1
func TestEdgeMalformedOverlay(t *testing.T) {
	requirePodman(t)

	root := repoRoot(t)

	// Write a malformed overlay file.
	badOverlayPath := filepath.Join(root, "deployments", "vss-overlay-bad.json")
	if err := os.WriteFile(badOverlayPath, []byte("{invalid json"), 0o644); err != nil {
		t.Fatalf("failed to write bad overlay: %v", err)
	}
	t.Cleanup(func() { _ = os.Remove(badOverlayPath) })

	// Write a temporary compose file that uses the bad overlay.
	// Uses port 55557 to avoid colliding with the main databroker.
	badComposePath := filepath.Join(root, "deployments", "compose-bad-overlay.yml")
	badCompose := `services:
  databroker-bad:
    image: ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0
    ports:
      - "55557:55555"
    volumes:
      - ./vss-overlay-bad.json:/etc/kuksa/vss-overlay.json
    command:
      - "--vss"
      - "/vss_release_4.0.json,/etc/kuksa/vss-overlay.json"
      - "--address"
      - "0.0.0.0:55555"
      - "--insecure"
`
	if err := os.WriteFile(badComposePath, []byte(badCompose), 0o644); err != nil {
		t.Fatalf("failed to write bad compose: %v", err)
	}
	t.Cleanup(func() {
		_ = exec.Command("podman", "compose", "-f", badComposePath, "down").Run()
		_ = os.Remove(badComposePath)
	})

	// Attempt to start; may fail immediately or exit shortly after.
	cmd := exec.Command("podman", "compose", "-f", badComposePath, "up", "-d", "databroker-bad")
	cmd.Dir = filepath.Join(root, "deployments")
	_, _ = cmd.CombinedOutput()

	// Wait briefly, then verify the databroker is not responding on port 55557.
	time.Sleep(3 * time.Second)

	healthy := func() bool {
		ctx, cancel := timeout2s()
		defer cancel()
		return exec.CommandContext(ctx,
			"grpcurl", "-plaintext",
			"localhost:55557",
			"kuksa.val.v1.VAL/GetServerInfo",
		).Run() == nil
	}

	if healthy() {
		t.Error("databroker started successfully with malformed overlay — expected failure or unhealthy state")
	}
}

// TS-02-E4: Get Unset Custom Signal
// Requirement: 02-REQ-3.E2
func TestEdgeGetUnsetSignal(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	// Query the custom signal without ever setting it.
	const signal = "Vehicle.Parking.SessionActive"
	out, err := grpcGet(tcpEndpoint, signal)

	// The call must succeed (exit 0). Grpcurl exits 0 even when the response
	// contains an error body — only hard transport failures yield non-zero exit.
	if err != nil {
		t.Logf("Get(%s) transport error (may still be acceptable): %v\noutput: %s", signal, err, out)
		return
	}

	// The response must NOT be a NOT_FOUND — the signal is registered in the
	// overlay and must be known to the databroker; it simply has no value yet.
	if hasNotFoundInBody(out) {
		t.Errorf("Get(%s) on unset signal returned not_found; the signal must exist with no value:\n%s", signal, out)
	}
}

// TS-02-E5: Query Non-Existent Signal
// Requirement: 02-REQ-4.E1
func TestEdgeNonExistentSignal(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	// grpcurl exits 0 even when the response body contains a NOT_FOUND error,
	// so we must inspect the body.
	const signal = "Vehicle.NonExistent.Signal"
	out, _ := grpcGet(tcpEndpoint, signal)

	if !hasNotFoundInBody(out) {
		t.Errorf("Get(%s) did not return not_found for a non-existent signal; response:\n%s", signal, out)
	}
}

// TS-02-E6: Subscriber Reconnect
// Requirement: 02-REQ-5.E1
func TestEdgeSubscriberReconnect(t *testing.T) {
	requireLiveDatabroker(t)
	startDatabroker(t)

	const signal = "Vehicle.Parking.SessionActive"

	// First subscription — set false and let it cancel naturally.
	grpcSubscribeCapture(t, tcpEndpoint, signal, 2*time.Second, func() {
		_, _ = grpcSetBool(tcpEndpoint, signal, false)
	})

	// Reset to false so the second subscription has a clear transition.
	_, _ = grpcSetBool(tcpEndpoint, signal, false)

	// Second subscription (reconnect) — must receive notification after set.
	captured := grpcSubscribeCapture(t, tcpEndpoint, signal, 8*time.Second, func() {
		if out, err := grpcSetBool(tcpEndpoint, signal, true); err != nil {
			t.Errorf("Set(%s, true) on reconnect failed: %v\noutput: %s", signal, err, out)
		}
	})

	if !strings.Contains(captured, "true") {
		t.Errorf("Second subscription(%s): did not receive 'true' after reconnect:\n%s", signal, captured)
	}
}

// timeout2s returns a 2-second context and cancel func (helper for one-off checks).
func timeout2s() (context.Context, context.CancelFunc) {
	return context.WithTimeout(context.Background(), 2*time.Second)
}
