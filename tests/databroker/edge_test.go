package databroker_test

// edge_test.go — edge case tests for error scenarios.
//
// These tests verify that the DATA_BROKER handles error conditions correctly:
// - Setting a non-existent signal returns NOT_FOUND.
// - Starting the broker with an invalid overlay fails.
// - Starting the broker with a missing overlay fails.
// - The running container uses the pinned image version.
//
// Container lifecycle tests (E2, E3, ImageVersion) use `podman run --rm` with
// a dedicated test port (55599) to avoid conflicts with a running instance.
// They skip gracefully when Podman is not installed.
//
// Tests: TS-02-E1, TS-02-E2, TS-02-E3, TS-02-3 (live).
// Requirements: 02-REQ-8.E1, 02-REQ-6.E1, 02-REQ-6.E2, 02-REQ-1.1.

import (
	"context"
	"os"
	"os/exec"
	"strings"
	"testing"
	"time"
)

// requirePodman skips the test if the podman binary is not in PATH.
func requirePodman(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not installed; skipping container lifecycle test")
	}
}

// pinnedImage is the exact image tag required by 02-REQ-1.1.
const pinnedImage = "ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0"

// testPort is an alternate TCP port used for test containers to avoid
// conflicting with the running DATA_BROKER instance on port 55555.
const testPort = "55599"

// ---- TS-02-E1: non-existent signal ----

// TestEdgeCaseNonExistentSignal verifies that setting a signal that does not
// exist in the VSS tree returns a NOT_FOUND error (TS-02-E1, 02-REQ-8.E1).
func TestEdgeCaseNonExistentSignal(t *testing.T) {
	requireTCPReachable(t)
	requireGrpcurl(t)

	out, err := grpcurlTCPRaw(
		"kuksa.val.v2.VAL/PublishValue",
		`{"signal_id": {"path": "Vehicle.NonExistent.Signal"}, "data_point": {"value": {"float": 42}}}`,
	)

	if err == nil {
		t.Errorf("expected grpcurl to exit non-zero for non-existent signal; got:\n%s", out)
		return
	}

	// The error response should mention NOT_FOUND / NotFound.
	lc := strings.ToLower(out)
	if !strings.Contains(lc, "not_found") && !strings.Contains(lc, "notfound") {
		t.Errorf("expected NOT_FOUND in error output for non-existent signal, got:\n%s", out)
	}
}

// ---- TS-02-E2: overlay syntax error ----

// TestEdgeCaseOverlaySyntaxError verifies that the DATA_BROKER fails to start
// when the VSS overlay file contains invalid JSON (TS-02-E2, 02-REQ-6.E1).
//
// Strategy: write invalid JSON to a temp file, mount it as the overlay, and
// run the pinned image directly via `podman run --rm`. The container should
// exit with a non-zero status code immediately.
func TestEdgeCaseOverlaySyntaxError(t *testing.T) {
	requirePodman(t)

	// Create a temp file with malformed JSON.
	tmpOverlay, err := os.CreateTemp("", "kuksa-overlay-bad-*.json")
	if err != nil {
		t.Fatalf("cannot create temp overlay file: %v", err)
	}
	defer os.Remove(tmpOverlay.Name())

	if _, err := tmpOverlay.WriteString(`{invalid json syntax`); err != nil {
		t.Fatalf("cannot write invalid JSON to temp overlay: %v", err)
	}
	tmpOverlay.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	// Run the databroker with the invalid overlay via a test port to avoid conflicts.
	cmd := exec.CommandContext(ctx, "podman", "run", "--rm",
		"-v", tmpOverlay.Name()+":/app/vss-overlay.json:ro",
		pinnedImage,
		"--address", "0.0.0.0",
		"--port", testPort,
		"--vss", "/vss_release_4.0.json,/app/vss-overlay.json",
	)
	out, runErr := cmd.CombinedOutput()

	if runErr == nil {
		t.Errorf("expected databroker to fail with invalid overlay JSON; got exit 0.\nOutput:\n%s", out)
	} else {
		t.Logf("Container exited non-zero as expected: %v\nOutput:\n%s", runErr, out)
	}
}

// ---- TS-02-E3: missing overlay file ----

// TestEdgeCaseMissingOverlay verifies that the DATA_BROKER fails to start when
// the overlay file path does not exist inside the container (TS-02-E3, 02-REQ-6.E2).
//
// Strategy: run the pinned image with `--vss /vss_release_4.0.json,/app/vss-overlay.json`
// but WITHOUT mounting an overlay file. Since /app/vss-overlay.json is absent in
// the container, the broker should exit non-zero with a file-not-found error.
func TestEdgeCaseMissingOverlay(t *testing.T) {
	requirePodman(t)

	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	// No overlay volume mount → /app/vss-overlay.json is absent inside the container.
	cmd := exec.CommandContext(ctx, "podman", "run", "--rm",
		pinnedImage,
		"--address", "0.0.0.0",
		"--port", testPort,
		"--vss", "/vss_release_4.0.json,/app/vss-overlay.json",
	)
	out, runErr := cmd.CombinedOutput()

	if runErr == nil {
		t.Errorf("expected databroker to fail with missing overlay file; got exit 0.\nOutput:\n%s", out)
	} else {
		t.Logf("Container exited non-zero as expected: %v\nOutput:\n%s", runErr, out)
	}
}

// ---- TS-02-3 (live): pinned image version ----

// TestImageVersion verifies that the running DATA_BROKER container reports the
// pinned image version 0.5.0 via `podman ps` (TS-02-3 live, 02-REQ-1.1, 02-REQ-1.2).
//
// This is the live companion to the static TestComposePinnedImage check in
// compose_test.go, which only verifies compose.yml content.
func TestImageVersion(t *testing.T) {
	requireTCPReachable(t)
	requirePodman(t)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// Look for a running container whose name contains "kuksa-databroker".
	out, err := exec.CommandContext(ctx, "podman", "ps",
		"--format", "{{.Image}}",
		"--filter", "name=kuksa-databroker",
	).CombinedOutput()
	if err != nil {
		t.Skipf("podman ps failed (Podman VM may not be running): %v", err)
	}

	imageRef := strings.TrimSpace(string(out))
	if imageRef == "" {
		t.Skip("no kuksa-databroker container found running; skipping image version check")
	}

	// The container may show one or more image references (one per line).
	// Accept if any line contains the pinned tag.
	found := false
	for _, line := range strings.Split(imageRef, "\n") {
		if strings.Contains(line, "kuksa-databroker:0.5.0") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("running kuksa-databroker container image %q does not contain kuksa-databroker:0.5.0", imageRef)
	}

	// Log the found image reference for debugging.
	if !found {
		t.Logf("Image reference found: %q", imageRef)
	}
}
