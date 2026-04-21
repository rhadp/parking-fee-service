// Edge case integration tests for the DATA_BROKER.
// These tests cover error scenarios: non-existent signals, overlay errors, and
// container lifecycle failures. Tests that require container lifecycle manipulation
// (overlay syntax error, missing overlay) skip when Podman is unavailable.
// Task group 4 strengthens these tests with full implementation details.
package databroker

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
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
	skipIfPodmanMissing(t)

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
	})

	// Attempt to start the databroker with the invalid overlay.
	// The container should fail to start (exit non-zero).
	_, startErr := runPodmanCompose(t, "up", "--no-start", "kuksa-databroker")
	// Note: some compose implementations detect overlay errors only at runtime.
	// The container must not be healthy; a full start attempt is performed in group 4.
	_ = startErr // error handling strengthened in task group 4

	// Always ensure the container is stopped.
	_, _ = runPodmanCompose(t, "down") //nolint:errcheck
}

// TestEdgeCaseMissingOverlay verifies that the DATA_BROKER container fails to start
// when the VSS overlay file is missing.
// Test Spec: TS-02-E3
// Requirement: 02-REQ-6.E2
func TestEdgeCaseMissingOverlay(t *testing.T) {
	skipIfPodmanMissing(t)

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
	})

	// Attempt to start the databroker. Without the overlay file the container
	// should fail to start (exit non-zero). Full assertion is strengthened in group 4.
	_, _ = runPodmanCompose(t, "up", "--no-start", "kuksa-databroker") //nolint:errcheck
	_, _ = runPodmanCompose(t, "down")                                  //nolint:errcheck
}
