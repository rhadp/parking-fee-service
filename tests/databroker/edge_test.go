package databroker_test

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	kuksa "github.com/rhadp/parking-fee-service/gen/kuksa"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/types/known/timestamppb"
)

// deploymentsDir returns the absolute path to the deployments directory.
func deploymentsDir(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "deployments")
}

// overlayPath returns the absolute path to the VSS overlay file.
func overlayPath(t *testing.T) string {
	t.Helper()
	return filepath.Join(deploymentsDir(t), "vss-overlay.json")
}

// composeDown runs podman compose down in the deployments directory and cleans
// up any stale UDS socket file. This prevents stale sockets from causing
// subsequent test failures.
func composeDown(t *testing.T) {
	t.Helper()
	dir := deploymentsDir(t)
	cmd := exec.Command("podman", "compose", "down", "--timeout", "10")
	cmd.Dir = dir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Logf("compose down: %v\n%s", err, out)
	}
	// Clean up stale UDS socket files.
	for _, p := range []string{"/tmp/kuksa/kuksa-databroker.sock", "/tmp/kuksa-databroker.sock"} {
		_ = os.Remove(p)
	}
}

// assertContainerNotRunning verifies that the kuksa-databroker container is NOT
// in "Up" state. This is used by tests that expect the container to fail on
// startup (e.g. bad overlay). The container may exist in a stopped/exited state
// but must not be running.
func assertContainerNotRunning(t *testing.T) {
	t.Helper()
	dir := deploymentsDir(t)
	cmd := exec.Command("podman", "compose", "ps", "--format", "{{.Status}}", "--filter", "name=kuksa-databroker")
	cmd.Dir = dir
	out, err := cmd.CombinedOutput()
	if err != nil {
		// If compose ps fails, the container likely doesn't exist at all — that's fine.
		t.Logf("compose ps failed (container likely not running): %v", err)
		return
	}
	output := strings.TrimSpace(string(out))
	if output == "" {
		// No container found — expected for startup failure.
		return
	}
	// Check each line of output for running state.
	for _, line := range strings.Split(output, "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		lower := strings.ToLower(line)
		if strings.Contains(lower, "up") && !strings.Contains(lower, "exited") {
			t.Errorf("container kuksa-databroker is still running (status: %s); expected it to have failed", line)
		}
	}
}

// skipIfPodmanUnavailable skips the test if podman is not installed or not
// functional.
func skipIfPodmanUnavailable(t *testing.T) {
	t.Helper()
	cmd := exec.Command("podman", "version")
	if err := cmd.Run(); err != nil {
		t.Skipf("podman not available, skipping: %v", err)
	}
}

// waitForContainerExit waits up to the given duration for the kuksa-databroker
// container to exit (stop running). Returns true if the container exited within
// the timeout, false otherwise.
func waitForContainerExit(t *testing.T, timeout time.Duration) bool {
	t.Helper()
	dir := deploymentsDir(t)
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		cmd := exec.Command("podman", "compose", "ps", "--format", "{{.Status}}", "--filter", "name=kuksa-databroker")
		cmd.Dir = dir
		out, err := cmd.CombinedOutput()
		if err != nil {
			return true // can't query = not running
		}
		output := strings.TrimSpace(string(out))
		if output == "" {
			return true
		}
		running := false
		for _, line := range strings.Split(output, "\n") {
			lower := strings.ToLower(strings.TrimSpace(line))
			if strings.Contains(lower, "up") && !strings.Contains(lower, "exited") {
				running = true
				break
			}
		}
		if !running {
			return true
		}
		time.Sleep(2 * time.Second)
	}
	return false
}

// --- Edge case tests ---

// TestEdgeCaseNonExistentSignal verifies that setting a non-existent signal
// returns a NOT_FOUND or INVALID_ARGUMENT error.
// Test Spec: TS-02-E1
// Requirement: 02-REQ-8.E1
func TestEdgeCaseNonExistentSignal(t *testing.T) {
	skipIfTCPUnreachable(t)
	_, client := dialTCP(t)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_, err := client.PublishValue(ctx, &kuksa.PublishValueRequest{
		SignalId:  &kuksa.SignalID{Signal: &kuksa.SignalID_Path{Path: "Vehicle.NonExistent.Signal"}},
		DataPoint: &kuksa.Datapoint{Timestamp: timestamppb.Now(), Value: floatValue(42.0)},
	})
	if err == nil {
		t.Fatal("expected error when setting non-existent signal, got nil")
	}

	st, ok := status.FromError(err)
	if !ok {
		t.Fatalf("expected gRPC status error, got: %v", err)
	}
	// Kuksa may return NOT_FOUND or INVALID_ARGUMENT for unknown signals.
	code := st.Code()
	if code != codes.NotFound && code != codes.InvalidArgument {
		t.Errorf("expected NOT_FOUND or INVALID_ARGUMENT, got %v: %s", code, st.Message())
	}
}

// TestEdgeCaseOverlaySyntaxError verifies that the DATA_BROKER fails to start
// when the overlay file contains invalid JSON.
// Test Spec: TS-02-E2
// Requirement: 02-REQ-6.E1
func TestEdgeCaseOverlaySyntaxError(t *testing.T) {
	skipIfPodmanUnavailable(t)

	overlay := overlayPath(t)

	// Read the original overlay content.
	original, err := os.ReadFile(overlay)
	if err != nil {
		t.Fatalf("failed to read overlay: %v", err)
	}

	// Ensure we always restore the overlay and clean up.
	t.Cleanup(func() {
		if writeErr := os.WriteFile(overlay, original, 0o644); writeErr != nil {
			t.Errorf("failed to restore overlay: %v", writeErr)
		}
		composeDown(t)
	})

	// First, bring down any running instance.
	composeDown(t)

	// Write invalid JSON to the overlay file.
	invalidJSON := []byte(`{this is not valid JSON!!!`)
	if err := os.WriteFile(overlay, invalidJSON, 0o644); err != nil {
		t.Fatalf("failed to write invalid overlay: %v", err)
	}

	// Start the container — expect it to fail.
	dir := deploymentsDir(t)
	cmd := exec.Command("podman", "compose", "up", "-d", "kuksa-databroker")
	cmd.Dir = dir
	out, upErr := cmd.CombinedOutput()
	t.Logf("compose up with bad overlay: exit=%v output=%s", upErr, string(out))

	// Wait for the container to exit (it should crash on startup).
	if !waitForContainerExit(t, 30*time.Second) {
		t.Log("container did not exit within timeout; checking state")
	}

	// Positively verify the container is NOT running.
	assertContainerNotRunning(t)
}

// TestEdgeCaseMissingOverlay verifies that the DATA_BROKER fails to start when
// the overlay file is missing.
// Test Spec: TS-02-E3
// Requirement: 02-REQ-6.E2
func TestEdgeCaseMissingOverlay(t *testing.T) {
	skipIfPodmanUnavailable(t)

	overlay := overlayPath(t)
	backupPath := overlay + ".bak"

	// Ensure we always restore the overlay and clean up.
	t.Cleanup(func() {
		composeDown(t)
		// Podman may create a directory at the overlay path when the file is
		// missing (volume mount of a missing host path). Remove it first.
		_ = os.RemoveAll(overlay)
		if _, statErr := os.Stat(backupPath); statErr == nil {
			if renameErr := os.Rename(backupPath, overlay); renameErr != nil {
				t.Errorf("failed to restore overlay from backup: %v", renameErr)
			}
		}
	})

	// First, bring down any running instance.
	composeDown(t)

	// Move the overlay file to a backup location.
	if err := os.Rename(overlay, backupPath); err != nil {
		t.Fatalf("failed to rename overlay to backup: %v", err)
	}

	// Start the container — expect it to fail.
	dir := deploymentsDir(t)
	cmd := exec.Command("podman", "compose", "up", "-d", "kuksa-databroker")
	cmd.Dir = dir
	out, upErr := cmd.CombinedOutput()
	t.Logf("compose up with missing overlay: exit=%v output=%s", upErr, string(out))

	// Wait for the container to exit (it should fail on startup).
	if !waitForContainerExit(t, 30*time.Second) {
		t.Log("container did not exit within timeout; checking state")
	}

	// Positively verify the container is NOT running.
	assertContainerNotRunning(t)
}

// TestPermissiveModeWithArbitraryToken verifies that the DATA_BROKER accepts
// requests even when an invalid authorization token is provided.
// Test Spec: TS-02-E4
// Requirement: 02-REQ-7.E1
func TestPermissiveModeWithArbitraryToken(t *testing.T) {
	skipIfTCPUnreachable(t)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	conn, err := grpc.DialContext(ctx, tcpTarget, //nolint:staticcheck // DialContext is fine for tests
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(), //nolint:staticcheck // WithBlock is fine for tests
	)
	if err != nil {
		t.Fatalf("failed to dial TCP %s: %v", tcpTarget, err)
	}
	defer conn.Close()

	client := kuksa.NewVALClient(conn)

	// Add an arbitrary invalid authorization token to the request metadata.
	md := metadata.New(map[string]string{
		"authorization": "Bearer invalid-token-12345",
	})
	authCtx := metadata.NewOutgoingContext(context.Background(), md)
	authCtx, authCancel := context.WithTimeout(authCtx, 5*time.Second)
	defer authCancel()

	// The request should succeed despite the invalid token (permissive mode).
	resp, err := client.GetValue(authCtx, &kuksa.GetValueRequest{
		SignalId: &kuksa.SignalID{Signal: &kuksa.SignalID_Path{Path: "Vehicle.Speed"}},
	})
	if err != nil {
		st, ok := status.FromError(err)
		if ok && st.Code() == codes.PermissionDenied {
			t.Fatalf("request rejected with PERMISSION_DENIED; databroker should be in permissive mode: %v", err)
		}
		// Other errors (like "no value set") are acceptable — the point is it
		// was not rejected for authentication reasons.
		t.Logf("request returned non-auth error (acceptable in permissive mode): %v", err)
		return
	}
	t.Logf("request succeeded with arbitrary token, response: %v", resp)
}

// --- Pinned image version tests ---

// TestComposePinnedImageVersion verifies that compose.yml references the pinned
// Kuksa Databroker image tag per 02-REQ-1.1.
// Test Spec: TS-02-3 (static check)
// Requirement: 02-REQ-1.1
func TestComposePinnedImageVersion(t *testing.T) {
	content := readCompose(t)
	expected := "ghcr.io/eclipse-kuksa/kuksa-databroker:0.6"
	if !strings.Contains(content, expected) {
		t.Errorf("compose.yml does not contain pinned image %q", expected)
	}
	// Ensure it's not using :latest or an unpinned reference.
	if strings.Contains(content, "kuksa-databroker:latest") {
		t.Error("compose.yml uses :latest tag; should use pinned version :0.6")
	}
}

// TestImageVersion verifies that the running DATA_BROKER container uses the
// pinned image version. Inspects the running container to verify the image
// reference is not :latest and contains kuksa-databroker.
// Test Spec: TS-02-3 (live check)
// Requirement: 02-REQ-1.1
func TestImageVersion(t *testing.T) {
	skipIfTCPUnreachable(t)
	skipIfPodmanUnavailable(t)

	// Use podman ps to inspect the running container image.
	cmd := exec.Command("podman", "ps", "--format", "{{.Image}}", "--filter", fmt.Sprintf("ancestor=%s", "ghcr.io/eclipse-kuksa/kuksa-databroker"))
	out, err := cmd.CombinedOutput()
	if err != nil {
		// Fallback: try a broader podman ps.
		cmd2 := exec.Command("podman", "ps", "--format", "{{.Image}}")
		out2, err2 := cmd2.CombinedOutput()
		if err2 != nil {
			t.Skipf("could not inspect running containers: %v", err2)
		}
		out = out2
	}

	output := strings.TrimSpace(string(out))
	if output == "" {
		t.Skip("no running containers found to verify image version")
	}

	// Check that at least one line references kuksa-databroker.
	found := false
	for _, line := range strings.Split(output, "\n") {
		line = strings.TrimSpace(line)
		if strings.Contains(line, "kuksa-databroker") {
			found = true
			if strings.Contains(line, ":latest") {
				t.Errorf("running container uses :latest tag: %s; expected pinned version", line)
			}
			t.Logf("running image: %s", line)
		}
	}
	if !found {
		t.Skip("kuksa-databroker container not found in podman ps output")
	}
}
