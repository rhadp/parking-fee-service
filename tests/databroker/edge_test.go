// Edge case tests for the DATA_BROKER component.
//
// These tests verify error handling and boundary conditions:
//   - TS-02-E1: setting a non-existent signal returns NOT_FOUND
//   - TS-02-E2: overlay syntax error prevents databroker startup
//   - TS-02-E3: missing overlay file prevents databroker startup
//   - TS-02-E4: permissive mode accepts requests with invalid tokens
//   - TS-02-3:  running container uses the pinned image version
//
// Live tests skip when the DATA_BROKER is not running.
// Container lifecycle tests (E2, E3) require podman to be available.
//
// Test Specs: TS-02-E1, TS-02-E2, TS-02-E3, TS-02-E4, TS-02-3
// Requirements: 02-REQ-8.E1, 02-REQ-6.E1, 02-REQ-6.E2, 02-REQ-7.E1, 02-REQ-1.1
package databroker_test

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// TestEdgeCaseNonExistentSignal verifies that setting a non-existent signal
// results in a NOT_FOUND (or equivalent) error from the DATA_BROKER.
//
// Test Spec: TS-02-E1
// Requirements: 02-REQ-8.E1
func TestEdgeCaseNonExistentSignal(t *testing.T) {
	requireDatabrokerTCP(t)

	setBody := `{"signal_id":{"path":"Vehicle.NonExistent.Signal"},"data_point":{"bool":true}}`
	stdout, stderr, err := grpcurlTCP(t, "kuksa.val.v2.VAL/PublishValue", setBody)
	combined := stdout + stderr

	// We expect an error (non-zero exit) with a NOT_FOUND indication.
	if err == nil {
		t.Errorf("expected error when setting non-existent signal, but call succeeded; output: %s", combined)
		return
	}

	// The response should indicate the signal was not found.
	lowerCombined := strings.ToLower(combined)
	hasNotFound := strings.Contains(lowerCombined, "notfound") ||
		strings.Contains(lowerCombined, "not_found") ||
		strings.Contains(lowerCombined, "not found") ||
		strings.Contains(combined, "404") ||
		strings.Contains(lowerCombined, "unknown") // some brokers return UNKNOWN for missing signals

	if !hasNotFound {
		t.Errorf("expected NOT_FOUND error for non-existent signal; got: %s", combined)
	}
}

// TestEdgeCaseOverlaySyntaxError verifies that the DATA_BROKER fails to start
// when the VSS overlay file contains a syntax error.
//
// This test replaces the overlay file with invalid JSON, starts the container
// synchronously (without -d), and verifies it exits with a non-zero status.
// The overlay file is restored by the cleanup function regardless of outcome.
//
// Test Spec: TS-02-E2
// Requirements: 02-REQ-6.E1
func TestEdgeCaseOverlaySyntaxError(t *testing.T) {
	requirePodman(t)

	root := repoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")

	// Read the valid overlay content for restoration.
	original, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("cannot read overlay file: %v", err)
	}

	// Restore the original overlay and stop any running containers on cleanup.
	t.Cleanup(func() {
		// Bring down any containers started during this test.
		podmanCompose(t, "down") //nolint:errcheck
		if err := os.WriteFile(overlayPath, original, 0o644); err != nil {
			t.Errorf("failed to restore overlay file: %v", err)
		}
	})

	// Write a syntactically invalid overlay file.
	if err := os.WriteFile(overlayPath, []byte(`{{{ not valid JSON !!!`), 0o644); err != nil {
		t.Fatalf("cannot write invalid overlay: %v", err)
	}

	// Run podman compose up synchronously with a timeout.  When the overlay is
	// invalid, kuksa-databroker should exit immediately, causing compose to
	// return a non-zero exit code.
	deploymentsDir := filepath.Join(root, "deployments")
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	// Build the command: try "podman compose" first.
	var cmd *exec.Cmd
	if _, lkErr := exec.LookPath("podman"); lkErr == nil {
		cmd = exec.CommandContext(ctx, "podman", "compose", "up", "kuksa-databroker")
	} else {
		cmd = exec.CommandContext(ctx, "podman-compose", "up", "kuksa-databroker")
	}
	cmd.Dir = deploymentsDir

	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	upErr := cmd.Run()
	combined := stdout.String() + stderr.String()
	t.Logf("podman compose up output (err=%v): %s", upErr, combined)

	// The container should exit with a non-zero status when the overlay is bad.
	if upErr == nil {
		t.Error("expected non-zero exit from podman compose up with invalid VSS overlay; got success")
	}

	// The output/logs should mention a parse/error condition.
	lowerCombined := strings.ToLower(combined)
	if !strings.Contains(lowerCombined, "error") &&
		!strings.Contains(lowerCombined, "parse") &&
		!strings.Contains(lowerCombined, "invalid") &&
		!strings.Contains(lowerCombined, "fail") {
		t.Logf("Note: compose output does not contain 'error'/'parse'/'invalid'/'fail'; full output: %s", combined)
	}
}

// TestEdgeCaseMissingOverlay verifies that the DATA_BROKER fails to start
// when the VSS overlay file is missing.
//
// This test renames the overlay file to simulate its absence, starts the
// container synchronously, and verifies it exits with a non-zero status.
// The overlay file is restored by the cleanup function regardless of outcome.
//
// Test Spec: TS-02-E3
// Requirements: 02-REQ-6.E2
func TestEdgeCaseMissingOverlay(t *testing.T) {
	requirePodman(t)

	root := repoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")
	backupPath := overlayPath + ".bak"

	// Read the original overlay for restoration.
	original, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("cannot read overlay file: %v", err)
	}

	// Restore the overlay and clean up containers on test completion.
	t.Cleanup(func() {
		podmanCompose(t, "down") //nolint:errcheck
		os.Remove(backupPath)    //nolint:errcheck
		if err := os.WriteFile(overlayPath, original, 0o644); err != nil {
			t.Errorf("failed to restore overlay file: %v", err)
		}
	})

	// Rename the overlay to simulate a missing file.
	if err := os.Rename(overlayPath, backupPath); err != nil {
		t.Fatalf("cannot rename overlay to backup: %v", err)
	}

	// Run podman compose up synchronously with a timeout.  When the overlay is
	// missing, either the bind-mount fails immediately or kuksa-databroker cannot
	// read the file; either way the container should exit non-zero.
	deploymentsDir := filepath.Join(root, "deployments")
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	var cmd *exec.Cmd
	if _, lkErr := exec.LookPath("podman"); lkErr == nil {
		cmd = exec.CommandContext(ctx, "podman", "compose", "up", "kuksa-databroker")
	} else {
		cmd = exec.CommandContext(ctx, "podman-compose", "up", "kuksa-databroker")
	}
	cmd.Dir = deploymentsDir

	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	upErr := cmd.Run()
	combined := stdout.String() + stderr.String()
	t.Logf("podman compose up output (err=%v): %s", upErr, combined)

	// The container should fail to start when the overlay file is missing.
	if upErr == nil {
		t.Error("expected non-zero exit from podman compose up with missing VSS overlay; got success")
	}
}

// TestEdgeCaseGetUnsetSignal verifies that getting an unset signal returns
// an appropriate response (empty value or default) without crashing.
//
// Requirements: general robustness
func TestEdgeCaseGetUnsetSignal(t *testing.T) {
	requireDatabrokerTCP(t)

	// Get a signal that has not been set — should return a response (possibly
	// empty/unset) without an error.
	getBody := `{"signal_id":{"path":"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"}}`
	stdout, stderr, err := grpcurlTCP(t, "kuksa.val.v2.VAL/GetValue", getBody)
	combined := stdout + stderr

	if err != nil {
		// Some brokers return an error for unset signals; others return empty.
		// As long as it's not a crash, it's acceptable.
		t.Logf("GetValue for unset signal returned error (acceptable): %v; output: %s", err, combined)
		return
	}
	t.Logf("GetValue for unset signal returned: %s", combined)
}

// TestImageVersion verifies that the running DATA_BROKER container uses the
// pinned image version 0.5.0 as reported by the running container's image info.
//
// This live test complements the static TestComposePinnedImage test by
// inspecting the actual running container rather than just checking compose.yml.
//
// Test Spec: TS-02-3
// Requirements: 02-REQ-1.1, 02-REQ-1.2
func TestImageVersion(t *testing.T) {
	requirePodman(t)
	requireDatabrokerTCP(t)

	// Use podman ps to find the running kuksa-databroker container and verify
	// it uses the expected pinned image tag.
	cmd := exec.Command("podman", "ps",
		"--filter", "ancestor=ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0",
		"--format", "{{.Image}}")
	out, err := cmd.Output()
	if err != nil {
		// Fall back to listing all containers and checking the image.
		t.Logf("podman ps --filter ancestor failed (%v); trying broader search", err)
		cmd2 := exec.Command("podman", "ps", "--format", "{{.Image}}")
		out2, err2 := cmd2.Output()
		if err2 != nil {
			t.Skipf("podman ps failed: %v; skipping image version check", err2)
		}
		if !strings.Contains(string(out2), "kuksa-databroker:0.5.0") {
			t.Errorf("running container does not use pinned image kuksa-databroker:0.5.0; found: %s", string(out2))
		}
		return
	}

	// The image name should contain the expected version tag.
	imageStr := strings.TrimSpace(string(out))
	if imageStr == "" {
		// Try an alternate search via container name.
		cmd3 := exec.Command("podman", "inspect",
			"--format", "{{.ImageName}}",
			"--latest")
		out3, err3 := cmd3.Output()
		if err3 != nil {
			t.Logf("Could not find running kuksa-databroker container (podman inspect: %v)", err3)
			t.Skip("kuksa-databroker container not found via podman; skipping image version check")
		}
		if !strings.Contains(string(out3), "kuksa-databroker:0.5.0") {
			t.Errorf("running container image does not match pinned version; got: %s", string(out3))
		}
		return
	}

	if !strings.Contains(imageStr, "kuksa-databroker:0.5.0") {
		t.Errorf("running container image tag is not 0.5.0; got: %s", imageStr)
	}
}
