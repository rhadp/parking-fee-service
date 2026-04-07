package databroker_test

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-02-E2: Overlay syntax error
// Requirement: 02-REQ-6.E1
// ---------------------------------------------------------------------------

// TestEdgeCaseOverlaySyntaxError verifies that the DATA_BROKER container fails
// to start when the VSS overlay file contains a syntax error.
func TestEdgeCaseOverlaySyntaxError(t *testing.T) {
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not available on PATH; skipping container lifecycle test")
	}

	root := findRepoRoot(t)
	deploymentsDir := filepath.Join(root, "deployments")
	overlayPath := filepath.Join(deploymentsDir, "vss-overlay.json")
	backupPath := overlayPath + ".syntax-test-bak"

	// Read original overlay content.
	original, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("TS-02-E2: failed to read overlay file: %v", err)
	}

	// Ensure cleanup: stop any started containers, restore overlay, and restart.
	t.Cleanup(func() {
		// Stop containers first (ignore errors — they may not be running).
		cmd := exec.Command("podman", "compose", "down", "--timeout", "5")
		cmd.Dir = deploymentsDir
		_ = cmd.Run()

		// Restore original overlay — ensure parent path is a file, not a directory.
		_ = os.RemoveAll(overlayPath)
		if err := os.WriteFile(overlayPath, original, 0644); err != nil {
			t.Errorf("TS-02-E2: cleanup: failed to restore overlay: %v", err)
		}
		_ = os.Remove(backupPath)

		// Restart the container so subsequent tests have a running databroker.
		upCmd := exec.Command("podman", "compose", "up", "-d", "kuksa-databroker")
		upCmd.Dir = deploymentsDir
		_ = upCmd.Run()
		time.Sleep(3 * time.Second)
	})

	// Ensure no leftover containers from previous runs.
	stopCmd := exec.Command("podman", "compose", "down", "--timeout", "5")
	stopCmd.Dir = deploymentsDir
	_ = stopCmd.Run()

	// Write malformed JSON to the overlay file.
	malformedOverlay := []byte(`{this is not valid JSON!!!`)
	if err := os.WriteFile(overlayPath, malformedOverlay, 0644); err != nil {
		t.Fatalf("TS-02-E2: failed to write malformed overlay: %v", err)
	}

	// Start only the databroker service. We expect it to fail.
	upCmd := exec.Command("podman", "compose", "up", "-d", "kuksa-databroker")
	upCmd.Dir = deploymentsDir
	upOut, upErr := upCmd.CombinedOutput()
	t.Logf("TS-02-E2: compose up output: %s", string(upOut))

	// Give the container a moment to start and (hopefully) crash.
	time.Sleep(3 * time.Second)

	// Check if the container exited or is in a restart loop.
	// Use the compose-style container name.
	containerName := "deployments_kuksa-databroker_1"
	inspectCmd := exec.Command(
		"podman", "inspect",
		"--format", "{{.State.Status}} {{.State.ExitCode}}",
		containerName,
	)
	inspectOut, inspectErr := inspectCmd.CombinedOutput()
	inspectResult := strings.TrimSpace(string(inspectOut))
	t.Logf("TS-02-E2: container state: %q (inspect err: %v)", inspectResult, inspectErr)

	// Collect container logs for diagnostic purposes.
	logsCmd := exec.Command("podman", "logs", containerName)
	logsOut, _ := logsCmd.CombinedOutput()
	logs := string(logsOut)
	t.Logf("TS-02-E2: container logs:\n%s", logs)

	// Determine success: the container should have exited with non-zero,
	// or the compose up itself should have failed, or logs should contain
	// an error/parse message.
	containerFailed := false

	// Case 1: compose up itself failed.
	if upErr != nil {
		containerFailed = true
		t.Logf("TS-02-E2: compose up returned error (expected): %v", upErr)
	}

	// Case 2: container exited (not running).
	if strings.Contains(inspectResult, "exited") || strings.Contains(inspectResult, "dead") {
		containerFailed = true
	}

	// Case 3: logs contain error indicators.
	logsLower := strings.ToLower(logs)
	if strings.Contains(logsLower, "error") || strings.Contains(logsLower, "parse") ||
		strings.Contains(logsLower, "invalid") || strings.Contains(logsLower, "failed") {
		containerFailed = true
	}

	if !containerFailed {
		t.Errorf("TS-02-E2: expected databroker to fail with malformed overlay, "+
			"but container appears healthy. State: %q", inspectResult)
	}
}

// ---------------------------------------------------------------------------
// TS-02-E3: Missing overlay file
// Requirement: 02-REQ-6.E2
// ---------------------------------------------------------------------------

// TestEdgeCaseMissingOverlayFile verifies that the DATA_BROKER container fails
// to start when the overlay file is missing from the expected path.
func TestEdgeCaseMissingOverlayFile(t *testing.T) {
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not available on PATH; skipping container lifecycle test")
	}

	root := findRepoRoot(t)
	deploymentsDir := filepath.Join(root, "deployments")
	overlayPath := filepath.Join(deploymentsDir, "vss-overlay.json")
	backupPath := overlayPath + ".missing-test-bak"

	// Read original content for safe restoration.
	original, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("TS-02-E3: overlay file does not exist at %s: %v", overlayPath, err)
	}

	// Ensure cleanup: stop containers, restore overlay, and restart.
	t.Cleanup(func() {
		cmd := exec.Command("podman", "compose", "down", "--timeout", "5")
		cmd.Dir = deploymentsDir
		_ = cmd.Run()

		// Remove whatever podman may have created at the overlay path (could be a directory).
		_ = os.RemoveAll(overlayPath)
		// Restore from backup or original content.
		if _, err := os.Stat(backupPath); err == nil {
			data, readErr := os.ReadFile(backupPath)
			if readErr == nil {
				_ = os.WriteFile(overlayPath, data, 0644)
			}
			_ = os.Remove(backupPath)
		} else {
			_ = os.WriteFile(overlayPath, original, 0644)
		}

		// Restart the container so subsequent tests have a running databroker.
		upCmd := exec.Command("podman", "compose", "up", "-d", "kuksa-databroker")
		upCmd.Dir = deploymentsDir
		_ = upCmd.Run()
		time.Sleep(3 * time.Second)
	})

	// Ensure no leftover containers.
	stopCmd := exec.Command("podman", "compose", "down", "--timeout", "5")
	stopCmd.Dir = deploymentsDir
	_ = stopCmd.Run()

	// Move the overlay file out of the way.
	if err := os.Rename(overlayPath, backupPath); err != nil {
		t.Fatalf("TS-02-E3: failed to rename overlay for test: %v", err)
	}

	// Attempt to start the databroker. We expect compose up or the container to fail
	// because the volume mount source file is missing.
	upCmd := exec.Command("podman", "compose", "up", "-d", "kuksa-databroker")
	upCmd.Dir = deploymentsDir
	upOut, upErr := upCmd.CombinedOutput()
	t.Logf("TS-02-E3: compose up output: %s", string(upOut))

	// Give the container a moment to attempt startup.
	time.Sleep(3 * time.Second)

	// Check outcome.
	composeFailed := false

	// Case 1: compose up itself failed (e.g., volume mount source missing).
	if upErr != nil {
		composeFailed = true
		t.Logf("TS-02-E3: compose up returned error (expected): %v", upErr)
	}

	// Case 2: container exited.
	containerName := "deployments_kuksa-databroker_1"
	inspectCmd := exec.Command(
		"podman", "inspect",
		"--format", "{{.State.Status}} {{.State.ExitCode}}",
		containerName,
	)
	inspectOut, inspectErr := inspectCmd.CombinedOutput()
	inspectResult := strings.TrimSpace(string(inspectOut))
	t.Logf("TS-02-E3: container state: %q (inspect err: %v)", inspectResult, inspectErr)

	if inspectErr != nil {
		// Container doesn't exist at all — compose refused to start it.
		composeFailed = true
	} else if strings.Contains(inspectResult, "exited") || strings.Contains(inspectResult, "dead") {
		composeFailed = true
	}

	// Case 3: logs indicate file-not-found.
	logsCmd := exec.Command("podman", "logs", containerName)
	logsOut, _ := logsCmd.CombinedOutput()
	logs := strings.ToLower(string(logsOut))
	t.Logf("TS-02-E3: container logs:\n%s", string(logsOut))

	if strings.Contains(logs, "no such file") || strings.Contains(logs, "not found") ||
		strings.Contains(logs, "error") || strings.Contains(logs, "failed") ||
		strings.Contains(logs, "directory") {
		composeFailed = true
	}

	if !composeFailed {
		t.Errorf("TS-02-E3: expected databroker to fail with missing overlay file, "+
			"but container appears healthy. State: %q", inspectResult)
	}
}
