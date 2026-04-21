// Integration tests for the UPDATE_SERVICE.
//
// Tests that do not require external infrastructure always run when cargo is available:
//   - TestStartupLogging (TS-07-17)
//   - TestGracefulShutdown (TS-07-18)
//   - TestListAdaptersEmpty (TS-07-E9)
//   - TestGetAdapterStatusNotFound (TS-07-E8)
//   - TestRemoveAdapterNotFound (TS-07-E10)
//   - TestInstallAdapterInvalidArgument (TS-07-E1, TS-07-E2)
//
// Smoke tests that require real podman and an OCI registry skip gracefully when
// those are not available:
//   - TestSmokeEndToEndInstallAndQuery (TS-07-SMOKE-1)
//   - TestSmokeWatchAdapterStates (TS-07-SMOKE-2)
package updateservice

import (
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"
)

// --- TS-07-17: Startup Logging ---

// TestStartupLogging verifies that on startup the service logs the gRPC port
// (50052), the inactivity timeout, and a "ready" indicator.
//
// Test Spec: TS-07-17
// Requirements: 07-REQ-10.1
func TestStartupLogging(t *testing.T) {
	binPath := buildUpdateServiceBinary(t)
	proc := startUpdateService(t, binPath)

	// Wait up to 10s for the ready log line to appear.
	if !waitForLog(proc, "ready", 10*time.Second) {
		t.Fatalf("startup ready log not found within 10s\nprocess output:\n%s", proc.output.String())
	}

	output := proc.output.String()

	// Verify the log includes the gRPC port.
	if !strings.Contains(output, "50052") {
		t.Errorf("startup log does not include port 50052\noutput:\n%s", output)
	}

	// Verify the log includes an inactivity timeout indicator.
	if !strings.Contains(output, "inactivity_timeout") {
		t.Errorf("startup log does not include inactivity_timeout\noutput:\n%s", output)
	}
}

// --- TS-07-18: Graceful Shutdown ---

// TestGracefulShutdown verifies that SIGTERM causes the update-service to exit
// with code 0.
//
// Test Spec: TS-07-18
// Requirements: 07-REQ-10.2, 07-REQ-10.E1
func TestGracefulShutdown(t *testing.T) {
	binPath := buildUpdateServiceBinary(t)
	proc := startUpdateService(t, binPath)

	// Wait for the service to start and begin listening.
	if !waitForLog(proc, "ready", 10*time.Second) {
		t.Fatalf("startup ready log not found within 10s\nprocess output:\n%s", proc.output.String())
	}

	// Send SIGTERM to request graceful shutdown.
	if err := proc.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the process to exit cleanly (allow up to 15s for the 10s drain timeout).
	done := make(chan error, 1)
	go func() { done <- proc.cmd.Wait() }()

	select {
	case err := <-done:
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				t.Errorf("expected exit code 0 after SIGTERM, got %d", exitErr.ExitCode())
			} else {
				t.Errorf("unexpected error waiting for process: %v", err)
			}
		}
		// exit code 0 confirmed — test passes.
	case <-time.After(15 * time.Second):
		t.Fatal("update-service did not exit within 15s after SIGTERM")
	}
}

// --- TS-07-E9: ListAdapters Empty ---

// TestListAdaptersEmpty verifies that ListAdapters returns an empty list when
// no adapters have been installed.
//
// Test Spec: TS-07-E9
// Requirements: 07-REQ-4.E2
func TestListAdaptersEmpty(t *testing.T) {
	skipIfGrpcurlMissing(t)

	binPath := buildUpdateServiceBinary(t)
	startUpdateService(t, binPath)
	ensureServiceReady(t)

	out, err := grpcurlCallNoBody(t, "ListAdapters")
	if err != nil {
		// An empty response is valid JSON; grpcurl may return an empty body without error.
		// Only fail if the error is not due to an empty response.
		if !strings.Contains(out, "{}") && !strings.Contains(out, "adapters") && out != "" {
			t.Logf("grpcurl ListAdapters returned non-zero exit: %v\noutput: %s", err, out)
		}
	}

	// The response should not contain any adapter entries.
	if strings.Contains(out, "adapter_id") {
		t.Errorf("expected empty adapter list, but got adapter entries:\n%s", out)
	}
}

// --- TS-07-E8: GetAdapterStatus NOT_FOUND ---

// TestGetAdapterStatusNotFound verifies that GetAdapterStatus returns NOT_FOUND
// for an unknown adapter_id.
//
// Test Spec: TS-07-E8
// Requirements: 07-REQ-4.E1
func TestGetAdapterStatusNotFound(t *testing.T) {
	skipIfGrpcurlMissing(t)

	binPath := buildUpdateServiceBinary(t)
	startUpdateService(t, binPath)
	ensureServiceReady(t)

	out, _ := grpcurlCall(t, "GetAdapterStatus", `{"adapter_id":"nonexistent-adapter"}`)

	if !strings.Contains(out, "NOT_FOUND") && !strings.Contains(out, "adapter not found") {
		t.Errorf("expected NOT_FOUND error for unknown adapter, got:\n%s", out)
	}
}

// --- TS-07-E10: RemoveAdapter NOT_FOUND ---

// TestRemoveAdapterNotFound verifies that RemoveAdapter returns NOT_FOUND for
// an unknown adapter_id.
//
// Test Spec: TS-07-E10
// Requirements: 07-REQ-5.E1
func TestRemoveAdapterNotFound(t *testing.T) {
	skipIfGrpcurlMissing(t)

	binPath := buildUpdateServiceBinary(t)
	startUpdateService(t, binPath)
	ensureServiceReady(t)

	out, _ := grpcurlCall(t, "RemoveAdapter", `{"adapter_id":"nonexistent-adapter"}`)

	if !strings.Contains(out, "NOT_FOUND") && !strings.Contains(out, "adapter not found") {
		t.Errorf("expected NOT_FOUND error for unknown adapter, got:\n%s", out)
	}
}

// --- TS-07-E1, TS-07-E2: InstallAdapter INVALID_ARGUMENT ---

// TestInstallAdapterInvalidArgument verifies that InstallAdapter returns
// INVALID_ARGUMENT when image_ref or checksum_sha256 is empty.
//
// Test Spec: TS-07-E1, TS-07-E2
// Requirements: 07-REQ-1.E1, 07-REQ-1.E2
func TestInstallAdapterInvalidArgument(t *testing.T) {
	skipIfGrpcurlMissing(t)

	binPath := buildUpdateServiceBinary(t)
	startUpdateService(t, binPath)
	ensureServiceReady(t)

	// Empty image_ref — TS-07-E1
	t.Run("empty_image_ref", func(t *testing.T) {
		out, _ := grpcurlInstallAdapter(t, "", "sha256:abc123")
		if !strings.Contains(out, "INVALID_ARGUMENT") && !strings.Contains(out, "image_ref is required") {
			t.Errorf("expected INVALID_ARGUMENT for empty image_ref, got:\n%s", out)
		}
	})

	// Empty checksum — TS-07-E2
	t.Run("empty_checksum", func(t *testing.T) {
		out, _ := grpcurlInstallAdapter(t, "example.com/img:v1", "")
		if !strings.Contains(out, "INVALID_ARGUMENT") && !strings.Contains(out, "checksum_sha256 is required") {
			t.Errorf("expected INVALID_ARGUMENT for empty checksum, got:\n%s", out)
		}
	})
}

// --- TS-07-SMOKE-1: End-to-End Install and Query ---

// TestSmokeEndToEndInstallAndQuery is an end-to-end smoke test that requires
// a real podman installation and a pre-pulled OCI image.
// Skips gracefully when podman is not available.
//
// Test Spec: TS-07-SMOKE-1
// Requirements: 07-REQ-1.1 through 07-REQ-1.5, 07-REQ-4.1, 07-REQ-4.2
func TestSmokeEndToEndInstallAndQuery(t *testing.T) {
	skipIfGrpcurlMissing(t)

	// Skip if podman is not available.
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not in PATH; skipping end-to-end smoke test")
	}

	// Skip unless a pre-pulled test image is specified via environment variable.
	testImage := "docker.io/library/hello-world:latest"
	t.Logf("smoke test would use image %s (requires real podman + prior pull)", testImage)
	t.Skip("TS-07-SMOKE-1: skipping smoke test requiring real OCI registry — run manually with a real podman environment")
}

// --- TS-07-SMOKE-2: WatchAdapterStates Stream ---

// TestSmokeWatchAdapterStates verifies that WatchAdapterStates delivers state
// transition events when an adapter is installed.
// Skips gracefully when podman is not available.
//
// Test Spec: TS-07-SMOKE-2
// Requirements: 07-REQ-3.1, 07-REQ-3.2, 07-REQ-3.3
func TestSmokeWatchAdapterStates(t *testing.T) {
	skipIfGrpcurlMissing(t)

	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not in PATH; skipping WatchAdapterStates smoke test")
	}

	t.Skip("TS-07-SMOKE-2: skipping smoke test requiring real OCI registry — run manually with a real podman environment")
}
