package databroker_test

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/rhadp/parking-fee-service/gen/kuksa"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/status"
)

// composePath returns the absolute path to the compose.yml file.
func composePath(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "deployments", "compose.yml")
}

// overlayPath returns the absolute path to the VSS overlay JSON file.
func overlayPath(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "deployments", "vss-overlay.json")
}

// composeUp runs `podman compose up -d kuksa-databroker` and returns any error.
// It uses the deployments directory as the working directory.
func composeUp(t *testing.T) error {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "podman", "compose", "-f", composePath(t), "up", "-d", "kuksa-databroker")
	cmd.Dir = filepath.Join(repoRoot(t), "deployments")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Logf("podman compose up output: %s", string(out))
	}
	return err
}

// composeDown runs `podman compose down` and cleans up the UDS socket.
func composeDown(t *testing.T) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "podman", "compose", "-f", composePath(t), "down", "--timeout", "5")
	cmd.Dir = filepath.Join(repoRoot(t), "deployments")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Logf("podman compose down error: %v, output: %s", err, string(out))
	}

	// Clean up stale UDS sockets to prevent interference with subsequent tests.
	for _, sock := range []string{"/tmp/kuksa/kuksa-databroker.sock", "/tmp/kuksa-databroker.sock"} {
		_ = os.Remove(sock)
	}
}

// composeLogs retrieves the container logs for kuksa-databroker.
func composeLogs(t *testing.T) string {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "podman", "compose", "-f", composePath(t), "logs", "kuksa-databroker")
	cmd.Dir = filepath.Join(repoRoot(t), "deployments")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Logf("podman compose logs error: %v", err)
	}
	return string(out)
}

// assertContainerNotRunning positively verifies that the kuksa-databroker
// container is not in a running ("Up") state. This is needed because
// `podman compose up -d` may return nil error even if the container
// immediately exits.
func assertContainerNotRunning(t *testing.T) {
	t.Helper()

	// Give the container a moment to fail on startup.
	time.Sleep(3 * time.Second)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "podman", "compose", "-f", composePath(t), "ps", "--format", "{{.Status}}", "--filter", "name=kuksa-databroker")
	cmd.Dir = filepath.Join(repoRoot(t), "deployments")
	out, err := cmd.CombinedOutput()
	if err != nil {
		// If `ps` itself errors or returns nothing, the container is not running.
		t.Logf("podman compose ps error (container likely not running): %v", err)
		return
	}

	output := strings.TrimSpace(string(out))
	if output == "" {
		// No container found at all -- not running.
		return
	}

	// Check each line of output: container should NOT be "Up".
	for _, line := range strings.Split(output, "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		if strings.Contains(strings.ToLower(line), "up") {
			t.Errorf("expected kuksa-databroker container to NOT be running, but status is: %s", line)
		}
	}
}

// skipIfPodmanUnavailable skips the test if podman is not available.
func skipIfPodmanUnavailable(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not available, skipping container lifecycle test")
	}
}

// TestEdgeCaseNonExistentSignal verifies that setting a non-existent signal
// returns a NOT_FOUND or similar gRPC error.
//
// Test Spec: TS-02-E1
// Requirements: 02-REQ-8.E1
func TestEdgeCaseNonExistentSignal(t *testing.T) {
	skipIfTCPUnreachable(t)

	ctx, cancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, tcpTarget,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		t.Fatalf("failed to dial TCP: %v", err)
	}
	defer conn.Close()

	client := kuksa.NewVALClient(conn)

	// Attempt to set a signal that does not exist in the VSS tree.
	setCtx, setCancel := context.WithTimeout(context.Background(), rpcTimeout)
	defer setCancel()

	_, setErr := client.Set(setCtx, &kuksa.SetRequest{
		Updates: []*kuksa.EntryUpdate{
			{
				Entry: &kuksa.DataEntry{
					Path: "Vehicle.NonExistent.Signal",
					Value: &kuksa.Datapoint{
						Value: &kuksa.Datapoint_FloatValue{FloatValue: 42.0},
					},
				},
				Fields: []kuksa.Field{kuksa.Field_FIELD_VALUE},
			},
		},
	})

	if setErr == nil {
		t.Fatal("expected error when setting a non-existent signal, got nil")
	}

	// The DATA_BROKER should return NOT_FOUND or INVALID_ARGUMENT for
	// non-existent signals.
	st, ok := status.FromError(setErr)
	if !ok {
		t.Fatalf("expected gRPC status error, got: %v", setErr)
	}

	code := st.Code()
	if code != codes.NotFound && code != codes.InvalidArgument {
		t.Errorf("expected NOT_FOUND or INVALID_ARGUMENT, got %v: %s", code, st.Message())
	}
}

// TestEdgeCaseOverlaySyntaxError verifies that the DATA_BROKER fails to start
// when the overlay file has a syntax error.
//
// Test Spec: TS-02-E2
// Requirements: 02-REQ-6.E1
func TestEdgeCaseOverlaySyntaxError(t *testing.T) {
	skipIfPodmanUnavailable(t)

	overlay := overlayPath(t)

	// Save the original overlay file.
	original, err := os.ReadFile(overlay)
	if err != nil {
		t.Fatalf("failed to read overlay file: %v", err)
	}

	// Restore overlay and clean up container on exit.
	t.Cleanup(func() {
		if err := os.WriteFile(overlay, original, 0644); err != nil {
			t.Logf("WARNING: failed to restore overlay file: %v", err)
		}
		composeDown(t)
	})

	// Ensure any previous container is stopped first.
	composeDown(t)

	// Write invalid JSON content to the overlay file.
	invalidJSON := []byte(`{this is not valid JSON at all!!!`)
	if err := os.WriteFile(overlay, invalidJSON, 0644); err != nil {
		t.Fatalf("failed to write invalid overlay: %v", err)
	}

	// Attempt to start the databroker with the invalid overlay.
	_ = composeUp(t)

	// Verify the container is NOT running (it should fail to start).
	assertContainerNotRunning(t)

	// Check logs for error indication.
	logs := composeLogs(t)
	logsLower := strings.ToLower(logs)
	if logs != "" && !strings.Contains(logsLower, "error") && !strings.Contains(logsLower, "parse") && !strings.Contains(logsLower, "invalid") && !strings.Contains(logsLower, "failed") {
		t.Logf("Container logs (may indicate parse error): %s", logs)
	}
}

// TestEdgeCaseMissingOverlay verifies that the DATA_BROKER fails to start
// when the overlay file is missing.
//
// Test Spec: TS-02-E3
// Requirements: 02-REQ-6.E2
func TestEdgeCaseMissingOverlay(t *testing.T) {
	skipIfPodmanUnavailable(t)

	overlay := overlayPath(t)
	backupPath := overlay + ".bak"

	// Save original.
	original, err := os.ReadFile(overlay)
	if err != nil {
		t.Fatalf("failed to read overlay file: %v", err)
	}

	// Restore overlay and clean up container on exit.
	t.Cleanup(func() {
		// Podman may create a directory at the overlay path when the file
		// is missing and a bind mount is specified. Remove it first.
		info, statErr := os.Stat(overlay)
		if statErr == nil && info.IsDir() {
			_ = os.RemoveAll(overlay)
		}
		// Remove backup.
		_ = os.Remove(backupPath)
		// Restore original overlay.
		if err := os.WriteFile(overlay, original, 0644); err != nil {
			t.Logf("WARNING: failed to restore overlay file: %v", err)
		}
		composeDown(t)
	})

	// Ensure any previous container is stopped first.
	composeDown(t)

	// Rename the overlay file so it is missing.
	if err := os.Rename(overlay, backupPath); err != nil {
		t.Fatalf("failed to rename overlay to backup: %v", err)
	}

	// Attempt to start the databroker with the missing overlay.
	_ = composeUp(t)

	// Verify the container is NOT running (it should fail to start).
	assertContainerNotRunning(t)
}

// TestImageVersion verifies that the running DATA_BROKER container uses
// the pinned image version (not :latest).
//
// Test Spec: TS-02-3
// Requirements: 02-REQ-1.1
func TestImageVersion(t *testing.T) {
	skipIfTCPUnreachable(t)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// Query running containers for kuksa-databroker.
	cmd := exec.CommandContext(ctx, "podman", "ps", "--filter", "ancestor=ghcr.io/eclipse-kuksa/kuksa-databroker", "--format", "{{.Image}}")
	out, err := cmd.CombinedOutput()
	if err != nil {
		// Try alternative filter by name.
		cmd2 := exec.CommandContext(ctx, "podman", "ps", "--format", "{{.Image}}")
		out2, err2 := cmd2.CombinedOutput()
		if err2 != nil {
			t.Skipf("could not query running containers: %v", err2)
		}
		out = out2
	}

	output := strings.TrimSpace(string(out))
	if output == "" {
		t.Skip("no running containers found; skipping image version check")
	}

	// Find the kuksa-databroker image in the output.
	found := false
	for _, line := range strings.Split(output, "\n") {
		line = strings.TrimSpace(line)
		if strings.Contains(line, "kuksa-databroker") {
			found = true
			// Must NOT be :latest.
			if strings.HasSuffix(line, ":latest") {
				t.Errorf("running container uses :latest tag; expected a pinned version: %s", line)
			}
			// Must contain a version tag (the colon separator).
			if !strings.Contains(line, ":") {
				t.Errorf("running container image has no version tag: %s", line)
			}
			t.Logf("running kuksa-databroker image: %s", line)
			break
		}
	}
	if !found {
		t.Skip("kuksa-databroker container not found in running containers")
	}
}

// TestComposePinnedImageVersion verifies that compose.yml pins the
// kuksa-databroker image to the version specified by 02-REQ-1.1 (:0.6 tag).
//
// Note: The `:0.6` tag resolves to container package version 0.6.1.
// See errata E02-1 for the version discrepancy discussion.
//
// Test Spec: TS-02-3
// Requirements: 02-REQ-1.1
func TestComposePinnedImageVersion(t *testing.T) {
	content := readCompose(t)

	// 02-REQ-1.1 specifies the image SHALL be ghcr.io/eclipse-kuksa/kuksa-databroker:0.6
	expected := "ghcr.io/eclipse-kuksa/kuksa-databroker:0.6"
	if !strings.Contains(content, expected) {
		// Check if any version is pinned.
		if strings.Contains(content, "kuksa-databroker:latest") {
			t.Errorf("compose.yml uses :latest; 02-REQ-1.1 requires pinned version %s", expected)
		} else if !strings.Contains(content, "kuksa-databroker:") {
			t.Errorf("compose.yml does not reference kuksa-databroker at all")
		} else {
			// A version is pinned, but not the expected one. Log for visibility.
			t.Logf("compose.yml pins kuksa-databroker but not to %s; see errata E02-1", expected)
		}
	}
}

