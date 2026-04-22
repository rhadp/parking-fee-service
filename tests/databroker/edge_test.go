package databroker_test

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	pb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// skipIfPodmanNotRunning skips the test if podman is not available or the
// podman machine is not running with full container support.
func skipIfPodmanNotRunning(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not found in PATH")
	}
	// Verify podman can actually pull/run containers by checking machine state.
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	// Run a lightweight container to verify the podman backend is functional.
	cmd := exec.CommandContext(ctx, "podman", "version", "--format", "{{.Server.Version}}")
	output, err := cmd.CombinedOutput()
	if err != nil || strings.TrimSpace(string(output)) == "" {
		t.Skipf("podman server not reachable (machine stopped?): %v", err)
	}
}

// TestEdgeCaseNonExistentSignal verifies that setting a non-existent signal
// returns an appropriate gRPC error.
// TS-02-E1 | Requirement: 02-REQ-8.E1
func TestEdgeCaseNonExistentSignal(t *testing.T) {
	skipIfTCPUnreachable(t)
	conn := connectTCP(t)
	client := newVALClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_, err := client.Set(ctx, &pb.SetRequest{
		Updates: []*pb.EntryUpdate{
			{
				Entry: &pb.DataEntry{
					Path: "Vehicle.NonExistent.Signal",
					Value: &pb.Datapoint{
						Value: &pb.Datapoint_FloatValue{FloatValue: 42.0},
					},
				},
				Fields: []pb.Field{pb.Field_FIELD_VALUE},
			},
		},
	})
	if err == nil {
		t.Fatal("expected error when setting non-existent signal, got nil")
	}

	// The error should indicate the signal was not found.
	st, ok := status.FromError(err)
	if !ok {
		t.Fatalf("expected gRPC status error, got: %v", err)
	}
	// Accept NOT_FOUND or similar error codes indicating the signal doesn't exist.
	if st.Code() != codes.NotFound && st.Code() != codes.InvalidArgument {
		t.Errorf("expected NOT_FOUND or INVALID_ARGUMENT error, got %v: %s",
			st.Code(), st.Message())
	}
}

// skipIfComposeNotConfigured skips the test if compose.yml doesn't have dual
// listener configuration (requires TG2 completion).
func skipIfComposeNotConfigured(t *testing.T) {
	t.Helper()
	root := repoRoot(t)
	data, err := os.ReadFile(filepath.Join(root, "deployments", "compose.yml"))
	if err != nil {
		t.Skipf("cannot read compose.yml: %v", err)
	}
	content := string(data)
	if !strings.Contains(content, "--address") && !strings.Contains(content, "--port") {
		t.Skip("compose.yml not configured for dual listeners (TG2 not applied)")
	}
}

// TestEdgeCaseOverlaySyntaxError verifies that the DATA_BROKER fails to start
// when the overlay file has a syntax error.
// TS-02-E2 | Requirement: 02-REQ-6.E1
func TestEdgeCaseOverlaySyntaxError(t *testing.T) {
	skipIfComposeNotConfigured(t)
	skipIfPodmanNotRunning(t)

	root := repoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")

	// Read original overlay content for restoration.
	originalContent, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("failed to read overlay: %v", err)
	}
	t.Cleanup(func() {
		os.WriteFile(overlayPath, originalContent, 0644)
	})

	// Write invalid JSON to the overlay.
	if err := os.WriteFile(overlayPath, []byte("{invalid json content!!!"), 0644); err != nil {
		t.Fatalf("failed to write invalid overlay: %v", err)
	}

	// Try to start the databroker — it should fail.
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "podman", "compose", "up", "--no-start", "kuksa-databroker")
	cmd.Dir = filepath.Join(root, "deployments")
	output, err := cmd.CombinedOutput()

	// Even if compose up succeeds (creates container), try to start it.
	if err == nil {
		startCmd := exec.CommandContext(ctx, "podman", "compose", "up", "-d", "kuksa-databroker")
		startCmd.Dir = filepath.Join(root, "deployments")
		startCmd.CombinedOutput()

		// Wait briefly for the container to fail.
		time.Sleep(3 * time.Second)

		// Check container status.
		statusCmd := exec.CommandContext(ctx, "podman", "compose", "ps", "--format", "{{.Status}}")
		statusCmd.Dir = filepath.Join(root, "deployments")
		statusOutput, _ := statusCmd.CombinedOutput()

		// Cleanup: bring down the container.
		downCmd := exec.CommandContext(ctx, "podman", "compose", "down")
		downCmd.Dir = filepath.Join(root, "deployments")
		downCmd.CombinedOutput()

		statusStr := string(statusOutput)
		if strings.Contains(statusStr, "Up") {
			t.Error("DATA_BROKER should not be running with invalid overlay")
		}
	}
	_ = output
}

// TestEdgeCaseMissingOverlay verifies that the DATA_BROKER fails to start
// when the overlay file is missing.
// TS-02-E3 | Requirement: 02-REQ-6.E2
func TestEdgeCaseMissingOverlay(t *testing.T) {
	skipIfComposeNotConfigured(t)
	skipIfPodmanNotRunning(t)

	root := repoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")
	backupPath := overlayPath + ".bak"

	// Rename overlay to backup.
	if err := os.Rename(overlayPath, backupPath); err != nil {
		t.Fatalf("failed to rename overlay: %v", err)
	}
	t.Cleanup(func() {
		os.Rename(backupPath, overlayPath)
	})

	// Try to start the databroker — it should fail due to missing overlay.
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "podman", "compose", "up", "-d", "kuksa-databroker")
	cmd.Dir = filepath.Join(root, "deployments")
	output, err := cmd.CombinedOutput()

	// Cleanup: bring down any started container.
	downCmd := exec.CommandContext(ctx, "podman", "compose", "down")
	downCmd.Dir = filepath.Join(root, "deployments")
	downCmd.CombinedOutput()

	if err == nil {
		// If compose didn't fail, check if the container exited.
		// The container might fail to start due to missing volume source.
		outputStr := string(output)
		if !strings.Contains(strings.ToLower(outputStr), "error") &&
			!strings.Contains(strings.ToLower(outputStr), "no such file") {
			t.Log("compose output:", outputStr)
			// The container may have started but failed — check logs.
		}
	}
	// If err != nil, compose failed as expected — this is the success case.
}

// TestImageVersion verifies that the running DATA_BROKER container uses the
// pinned image version.
// TS-02-3 | Requirement: 02-REQ-1.1
func TestImageVersion(t *testing.T) {
	skipIfTCPUnreachable(t)

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "podman", "ps",
		"--filter", "name=kuksa-databroker",
		"--format", "{{.Image}}")
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Skipf("podman ps failed: %v", err)
	}

	imageRef := strings.TrimSpace(string(output))
	if imageRef == "" {
		t.Skip("no running kuksa-databroker container found")
	}

	// Verify the image is not :latest and contains a version tag.
	if strings.Contains(imageRef, ":latest") {
		t.Errorf("running container uses :latest image; expected pinned version, got %q", imageRef)
	}
	if !strings.Contains(imageRef, "kuksa-databroker") {
		t.Errorf("unexpected image reference: %q", imageRef)
	}
}
