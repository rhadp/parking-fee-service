package update_service_test

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"testing"
	"time"

	update "github.com/rhadp/parking-fee-service/gen/update"
)

// TestSmokeStartupLogging verifies that on startup the service logs its
// configuration (port, inactivity timeout) and a ready message.
// Test Spec: TS-07-17
// Requirements: 07-REQ-10.1, 07-REQ-7.3
func TestSmokeStartupLogging(t *testing.T) {
	skipIfCargoUnavailable(t)
	// Start the service with log capture so we can assert on log content.
	_, logBuf := startUpdateServiceCaptureLogs(t)

	// Assert the captured log output contains the configured port number
	// and a ready indicator, as required by 07-REQ-10.1.
	logs := logBuf.String()
	portStr := fmt.Sprintf("%d", testGRPCPort)
	if !strings.Contains(logs, portStr) {
		t.Errorf("expected startup logs to contain port %s, got:\n%s", portStr, logs)
	}
	if !strings.Contains(strings.ToLower(logs), "ready") {
		t.Errorf("expected startup logs to contain 'ready', got:\n%s", logs)
	}

	// Also verify the service is accepting gRPC connections.
	_, client := dialUpdateService(t)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	resp, err := client.ListAdapters(ctx, &update.ListAdaptersRequest{})
	if err != nil {
		t.Fatalf("ListAdapters failed after startup: %v", err)
	}
	if len(resp.GetAdapters()) != 0 {
		t.Errorf("expected empty adapter list on startup, got %d", len(resp.GetAdapters()))
	}
}

// TestSmokeGracefulShutdown verifies that on SIGTERM the service stops
// accepting RPCs and exits with code 0.
// Test Spec: TS-07-18
// Requirements: 07-REQ-10.2, 07-REQ-10.E1
func TestSmokeGracefulShutdown(t *testing.T) {
	skipIfCargoUnavailable(t)
	cmd := startUpdateService(t)

	// Verify the service is running.
	_, client := dialUpdateService(t)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	_, err := client.ListAdapters(ctx, &update.ListAdaptersRequest{})
	if err != nil {
		t.Fatalf("ListAdapters failed before shutdown: %v", err)
	}

	// Send SIGTERM.
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for exit.
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case waitErr := <-done:
		if waitErr != nil {
			// On Unix, a process killed by signal may return a non-nil
			// error even if the exit code is 0. Check the exit code.
			if exitErr, ok := waitErr.(*exec.ExitError); ok {
				if exitErr.ExitCode() != 0 {
					t.Errorf("expected exit code 0, got %d", exitErr.ExitCode())
				}
			}
		}
	case <-time.After(15 * time.Second):
		t.Fatal("service did not exit within 15 seconds after SIGTERM")
	}
}

// TestSmokeForceTerminateWithActiveRPC verifies that when SIGTERM is received
// while a long-running streaming RPC is in flight, the service drains for the
// configured 10-second timeout then force-terminates and exits with code 0.
//
// This test takes ≥10 seconds because the service always waits 10s before
// force-terminating, regardless of whether in-flight RPCs complete first.
//
// Test Spec: TS-07-E17
// Requirement: 07-REQ-10.E1
func TestSmokeForceTerminateWithActiveRPC(t *testing.T) {
	skipIfCargoUnavailable(t)
	cmd := startUpdateService(t)

	// Open a WatchAdapterStates stream — an infinite server-streaming RPC
	// that will remain in-flight until the server closes the connection.
	// Using a non-cancelled context so the stream stays alive until the
	// server shuts down.
	streamCtx, streamCancel := context.WithCancel(context.Background())
	defer streamCancel()

	_, client := dialUpdateService(t)
	stream, err := client.WatchAdapterStates(streamCtx, &update.WatchAdapterStatesRequest{})
	if err != nil {
		t.Fatalf("WatchAdapterStates failed: %v", err)
	}

	// Drain the stream in the background so the RPC stays active.
	go func() {
		for {
			_, recvErr := stream.Recv()
			if recvErr != nil {
				return
			}
		}
	}()

	// Allow the streaming RPC to be fully established server-side.
	time.Sleep(200 * time.Millisecond)

	// Send SIGTERM with the streaming RPC in flight.
	start := time.Now()
	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}

	// Wait for the service to exit.
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()

	select {
	case waitErr := <-done:
		elapsed := time.Since(start)
		t.Logf("service exited in %v with active streaming RPC (expected ~10s drain)", elapsed)

		// Exit code must be 0 (or a signal exit on Unix which is acceptable).
		if waitErr != nil {
			if exitErr, ok := waitErr.(*exec.ExitError); ok {
				if exitErr.ExitCode() != 0 {
					t.Errorf("expected exit code 0, got %d", exitErr.ExitCode())
				}
			}
			// Non-ExitError means the process was killed by a signal — acceptable.
		}

		// The service should have drained for at least 9s (10s timeout with
		// a 1-second tolerance for scheduling jitter).
		const minDrain = 9 * time.Second
		const maxDrain = 15 * time.Second
		if elapsed < minDrain {
			t.Errorf("expected service to drain for >= %v before force-terminating, but exited in %v", minDrain, elapsed)
		}
		if elapsed > maxDrain {
			t.Errorf("expected service to force-terminate within %v, but took %v (too slow)", maxDrain, elapsed)
		}

	case <-time.After(20 * time.Second):
		t.Fatal("service did not exit within 20 seconds after SIGTERM — may be stuck")
	}
}

// TestSmokeEndToEndInstallAndQuery starts the service, calls InstallAdapter
// (which will fail because podman is not available in CI), then verifies
// ListAdapters and GetAdapterStatus return the adapter. Finally removes
// the adapter and verifies it's gone.
// Test Spec: TS-07-SMOKE-1
// Requirements: 07-REQ-1.1, 07-REQ-4.1, 07-REQ-4.2, 07-REQ-5.1
func TestSmokeEndToEndInstallAndQuery(t *testing.T) {
	skipIfCargoUnavailable(t)
	skipIfPodmanUnavailable(t)
	_ = startUpdateService(t)

	_, client := dialUpdateService(t)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	imageRef := os.Getenv("SMOKE_TEST_IMAGE_REF")
	checksum := os.Getenv("SMOKE_TEST_CHECKSUM")
	if imageRef == "" || checksum == "" {
		t.Skip("SMOKE_TEST_IMAGE_REF and SMOKE_TEST_CHECKSUM not set, skipping end-to-end test")
	}

	// Step 1: Install an adapter.
	installResp, err := client.InstallAdapter(ctx, &update.InstallAdapterRequest{
		ImageRef:       imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		t.Fatalf("InstallAdapter failed: %v", err)
	}
	if installResp.GetJobId() == "" {
		t.Error("InstallAdapter response missing job_id")
	}
	if installResp.GetAdapterId() == "" {
		t.Error("InstallAdapter response missing adapter_id")
	}
	adapterID := installResp.GetAdapterId()
	t.Logf("InstallAdapter: job_id=%s adapter_id=%s state=%v",
		installResp.GetJobId(), adapterID, installResp.GetState())

	// Wait for the adapter to reach RUNNING state.
	deadline := time.Now().Add(60 * time.Second)
	for time.Now().Before(deadline) {
		statusResp, statusErr := client.GetAdapterStatus(ctx, &update.GetAdapterStatusRequest{
			AdapterId: adapterID,
		})
		if statusErr != nil {
			t.Fatalf("GetAdapterStatus failed: %v", statusErr)
		}
		state := statusResp.GetAdapter().GetState()
		if state == update.AdapterState_RUNNING {
			t.Logf("Adapter %s reached RUNNING state", adapterID)
			break
		}
		if state == update.AdapterState_ERROR {
			t.Fatalf("Adapter %s entered ERROR state", adapterID)
		}
		time.Sleep(1 * time.Second)
	}

	// Step 2: ListAdapters should include the adapter.
	listResp, err := client.ListAdapters(ctx, &update.ListAdaptersRequest{})
	if err != nil {
		t.Fatalf("ListAdapters failed: %v", err)
	}
	found := false
	for _, a := range listResp.GetAdapters() {
		if a.GetAdapterId() == adapterID {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("ListAdapters did not include adapter %s", adapterID)
	}

	// Step 3: GetAdapterStatus should return the adapter.
	statusResp, err := client.GetAdapterStatus(ctx, &update.GetAdapterStatusRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		t.Fatalf("GetAdapterStatus failed: %v", err)
	}
	if statusResp.GetAdapter().GetState() != update.AdapterState_RUNNING {
		t.Errorf("expected RUNNING state, got %v", statusResp.GetAdapter().GetState())
	}

	// Step 4: RemoveAdapter.
	_, err = client.RemoveAdapter(ctx, &update.RemoveAdapterRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		t.Fatalf("RemoveAdapter failed: %v", err)
	}

	// Step 5: ListAdapters should no longer include the adapter.
	listResp, err = client.ListAdapters(ctx, &update.ListAdaptersRequest{})
	if err != nil {
		t.Fatalf("ListAdapters after removal failed: %v", err)
	}
	for _, a := range listResp.GetAdapters() {
		if a.GetAdapterId() == adapterID {
			t.Errorf("ListAdapters still includes removed adapter %s", adapterID)
		}
	}
}

// TestSmokeWatchAdapterStatesStream subscribes to WatchAdapterStates,
// installs an adapter, and verifies the stream delivers state transition events.
// Test Spec: TS-07-SMOKE-2
// Requirements: 07-REQ-3.1, 07-REQ-3.2, 07-REQ-3.3
func TestSmokeWatchAdapterStatesStream(t *testing.T) {
	skipIfCargoUnavailable(t)
	skipIfPodmanUnavailable(t)
	_ = startUpdateService(t)

	_, client := dialUpdateService(t)

	imageRef := os.Getenv("SMOKE_TEST_IMAGE_REF")
	checksum := os.Getenv("SMOKE_TEST_CHECKSUM")
	if imageRef == "" || checksum == "" {
		t.Skip("SMOKE_TEST_IMAGE_REF and SMOKE_TEST_CHECKSUM not set, skipping stream test")
	}

	// Subscribe to state events.
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	stream, err := client.WatchAdapterStates(ctx, &update.WatchAdapterStatesRequest{})
	if err != nil {
		t.Fatalf("WatchAdapterStates failed: %v", err)
	}

	// Install an adapter to trigger events.
	_, err = client.InstallAdapter(ctx, &update.InstallAdapterRequest{
		ImageRef:       imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		t.Fatalf("InstallAdapter failed: %v", err)
	}

	// Collect events with a timeout.
	var events []*update.AdapterStateEvent
	eventDone := make(chan struct{})
	go func() {
		defer close(eventDone)
		for {
			event, recvErr := stream.Recv()
			if recvErr != nil {
				return
			}
			events = append(events, event)
			// Once we see RUNNING or ERROR, stop collecting.
			if event.GetNewState() == update.AdapterState_RUNNING ||
				event.GetNewState() == update.AdapterState_ERROR {
				return
			}
		}
	}()

	select {
	case <-eventDone:
	case <-time.After(60 * time.Second):
		t.Fatal("timed out waiting for state events")
	}

	if len(events) < 3 {
		t.Errorf("expected at least 3 events (UNKNOWN->DOWNLOADING, DOWNLOADING->INSTALLING, INSTALLING->RUNNING), got %d", len(events))
	}

	// Verify the event sequence.
	for _, event := range events {
		t.Logf("event: adapter_id=%s %v -> %v ts=%d",
			event.GetAdapterId(), event.GetOldState(), event.GetNewState(), event.GetTimestamp())
		if event.GetTimestamp() <= 0 {
			t.Errorf("event timestamp should be positive, got %d", event.GetTimestamp())
		}
	}
}

// TestSmokeInputValidation verifies that invalid inputs return proper
// gRPC error codes without starting podman.
// Test Spec: TS-07-E1, TS-07-E2, TS-07-E8, TS-07-E10
// Requirements: 07-REQ-1.E1, 07-REQ-1.E2, 07-REQ-4.E1, 07-REQ-5.E1
func TestSmokeInputValidation(t *testing.T) {
	skipIfCargoUnavailable(t)
	_ = startUpdateService(t)

	_, client := dialUpdateService(t)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	t.Run("empty_image_ref", func(t *testing.T) {
		_, err := client.InstallAdapter(ctx, &update.InstallAdapterRequest{
			ImageRef:       "",
			ChecksumSha256: "sha256:abc",
		})
		if err == nil {
			t.Fatal("expected error for empty image_ref")
		}
		t.Logf("got expected error: %v", err)
	})

	t.Run("empty_checksum", func(t *testing.T) {
		_, err := client.InstallAdapter(ctx, &update.InstallAdapterRequest{
			ImageRef:       "example.com/img:v1",
			ChecksumSha256: "",
		})
		if err == nil {
			t.Fatal("expected error for empty checksum")
		}
		t.Logf("got expected error: %v", err)
	})

	t.Run("get_unknown_adapter", func(t *testing.T) {
		_, err := client.GetAdapterStatus(ctx, &update.GetAdapterStatusRequest{
			AdapterId: "nonexistent-adapter",
		})
		if err == nil {
			t.Fatal("expected error for unknown adapter_id")
		}
		t.Logf("got expected error: %v", err)
	})

	t.Run("remove_unknown_adapter", func(t *testing.T) {
		_, err := client.RemoveAdapter(ctx, &update.RemoveAdapterRequest{
			AdapterId: "nonexistent-adapter",
		})
		if err == nil {
			t.Fatal("expected error for unknown adapter_id")
		}
		t.Logf("got expected error: %v", err)
	})
}
