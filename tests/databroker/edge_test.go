// Edge case integration tests for the DATA_BROKER.
// These tests cover error scenarios: non-existent signals, overlay errors, and
// container lifecycle failures. Tests that require container lifecycle manipulation
// (overlay syntax error, missing overlay) skip when Podman is unavailable.
// Task group 4 strengthens these tests with full implementation details.
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

// TestEdgeCaseNonExistentSignal verifies that publishing a value for a signal that
// does not exist in the VSS tree returns a gRPC error (NOT_FOUND or similar).
// Test Spec: TS-02-E1
// Requirement: 02-REQ-8.E1
func TestEdgeCaseNonExistentSignal(t *testing.T) {
	skipIfGrpcurlMissing(t)
	skipIfTCPNotReachable(t)

	out := grpcurlTCPExpectError(t, "PublishValue",
		`{"signal_id": {"path": "Vehicle.NonExistent.Signal"}, "value": {"int32_value": 42}}`)

	// The error output should indicate the signal was not found or is invalid.
	outLower := strings.ToLower(out)
	if !strings.Contains(outLower, "not found") &&
		!strings.Contains(outLower, "unknown") &&
		!strings.Contains(outLower, "invalid") &&
		!strings.Contains(outLower, "error") {
		t.Errorf("expected error message for non-existent signal, got: %s", out)
	}
}

// TestEdgeCaseOverlaySyntaxError verifies that the DATA_BROKER container fails to start
// when the VSS overlay file contains a JSON syntax error.
// Test Spec: TS-02-E2
// Requirement: 02-REQ-6.E1
func TestEdgeCaseOverlaySyntaxError(t *testing.T) {
	skipIfPodmanNotRunning(t)

	root := findRepoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")

	// Read and back up the original overlay.
	original, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("failed to read overlay file: %v", err)
	}

	// Write an invalid JSON overlay.
	if err := os.WriteFile(overlayPath, []byte(`{ INVALID JSON `), 0o644); err != nil {
		t.Fatalf("failed to write invalid overlay: %v", err)
	}
	t.Cleanup(func() {
		// Restore original overlay regardless of test outcome.
		if err := os.WriteFile(overlayPath, original, 0o644); err != nil {
			t.Errorf("failed to restore overlay: %v", err)
		}
		// Tear down any containers that may have started.
		_, _ = runPodmanCompose(t, "down") //nolint:errcheck
	})

	// Run `podman compose up kuksa-databroker` synchronously with a 20s timeout.
	// The container must fail to start due to the invalid overlay (exit non-zero).
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	out, startErr := runPodmanComposeCtx(ctx, t, "up", "kuksa-databroker")
	if startErr == nil {
		t.Fatalf("expected non-zero exit when overlay has syntax error, but compose up succeeded\noutput: %s", out)
	}
	t.Logf("compose up correctly failed with invalid overlay; error: %v\noutput: %s", startErr, out)

	// TS-02-E2 requires log verification: 'logs contain parse error'.
	// Check both compose output and container logs for error/parse indicators.
	outLower := strings.ToLower(out)
	hasErrorInOutput := strings.Contains(outLower, "error") || strings.Contains(outLower, "parse") ||
		strings.Contains(outLower, "invalid") || strings.Contains(outLower, "json")

	if !hasErrorInOutput {
		// Try fetching container logs separately in case compose output is minimal.
		logsCtx, logsCancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer logsCancel()
		logs, _ := runPodmanComposeCtx(logsCtx, t, "logs", "kuksa-databroker")
		logsLower := strings.ToLower(logs)
		hasErrorInLogs := strings.Contains(logsLower, "error") || strings.Contains(logsLower, "parse") ||
			strings.Contains(logsLower, "invalid") || strings.Contains(logsLower, "json")
		if !hasErrorInLogs {
			t.Errorf("expected parse/error keywords in compose output or container logs\ncompose output: %s\nlogs: %s", out, logs)
		} else {
			t.Logf("parse error found in container logs: %s", logs)
		}
	}
}

// TestEdgeCaseMissingOverlay verifies that the DATA_BROKER container fails to start
// when the VSS overlay file is missing.
// Test Spec: TS-02-E3
// Requirement: 02-REQ-6.E2
func TestEdgeCaseMissingOverlay(t *testing.T) {
	skipIfPodmanNotRunning(t)

	root := findRepoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")
	backupPath := overlayPath + ".bak"

	// Read and back up the original overlay.
	original, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("failed to read overlay file: %v", err)
	}

	// Rename the overlay so it is missing from the expected mount path.
	if err := os.Rename(overlayPath, backupPath); err != nil {
		t.Fatalf("failed to rename overlay: %v", err)
	}
	t.Cleanup(func() {
		// Restore the overlay from backup.
		if err := os.WriteFile(overlayPath, original, 0o644); err != nil {
			t.Errorf("failed to restore overlay: %v", err)
		}
		os.Remove(backupPath) //nolint:errcheck
		// Tear down any containers that may have started.
		_, _ = runPodmanCompose(t, "down") //nolint:errcheck
	})

	// Run `podman compose up kuksa-databroker` synchronously with a 20s timeout.
	// Without the overlay file the container should fail to start (exit non-zero).
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	out, startErr := runPodmanComposeCtx(ctx, t, "up", "kuksa-databroker")
	if startErr == nil {
		t.Fatalf("expected non-zero exit when overlay file is missing, but compose up succeeded\noutput: %s", out)
	}
	t.Logf("compose up correctly failed with missing overlay; error: %v\noutput: %s", startErr, out)
}

// TestImageVersion verifies that the running DATA_BROKER container uses the pinned
// kuksa-databroker image version. This live test complements the static
// TestComposePinnedImage check on compose.yml.
// Test Spec: TS-02-3
// Requirement: 02-REQ-1.1
func TestImageVersion(t *testing.T) {
	skipIfPodmanMissing(t)
	skipIfTCPNotReachable(t)

	// Query `podman ps` for the running kuksa-databroker container and its image.
	cmd := exec.Command("podman", "ps", "--format", "{{.Image}}", "--filter", "name=kuksa-databroker")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("podman ps failed: %v\noutput: %s", err, out)
	}

	imageRef := strings.TrimSpace(string(out))
	if imageRef == "" {
		t.Fatal("no running container matching 'kuksa-databroker' found via podman ps")
	}

	// Accept either the spec-mandated version (0.6.1) or the errata version (0.5.0).
	const wantV1 = "kuksa-databroker:0.6.1"
	const wantV2 = "kuksa-databroker:0.5.0"
	if !strings.Contains(imageRef, wantV1) && !strings.Contains(imageRef, wantV2) {
		t.Fatalf("running container image %q does not match expected pinned version\n"+
			"  expected one of: %q or %q", imageRef, wantV1, wantV2)
	}
	t.Logf("confirmed pinned image: %s", imageRef)
}
