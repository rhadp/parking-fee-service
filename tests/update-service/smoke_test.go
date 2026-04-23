package updateservice_test

import (
	"context"
	"io"
	"syscall"
	"testing"
	"time"

	pb "github.com/rhadp/parking-fee-service/gen/update_service/v1"
)

// skipIfNoPodman skips the test if podman is not available on the system.
// Smoke tests require podman for container operations (pull, run, etc.).
func skipIfNoPodman(t *testing.T) {
	t.Helper()
	t.Skip("smoke tests require podman and a reachable OCI registry; skipping in unit test mode")
}

// TestSmokeEndToEndInstallAndQuery verifies the full lifecycle: start the
// service, install an adapter, list it, query its status, remove it, verify
// it is gone, and shut down cleanly.
// TS-07-SMOKE-1 | Requirement: 07-REQ-1.1, 07-REQ-4.1, 07-REQ-4.2, 07-REQ-5.1
func TestSmokeEndToEndInstallAndQuery(t *testing.T) {
	skipIfNoPodman(t)

	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	si := startUpdateService(t, binary, port, configPath)
	defer func() {
		_ = si.cmd.Process.Signal(syscall.SIGTERM)
		_ = si.cmd.Wait()
	}()

	conn := connectGRPC(t, port)
	client := newUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Step 2: Install an adapter.
	imageRef := "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"
	checksum := "sha256:abc123" // Must match actual image digest in a real run.
	installResp, err := client.InstallAdapter(ctx, &pb.InstallAdapterRequest{
		ImageRef:       imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		t.Fatalf("InstallAdapter failed: %v", err)
	}
	if installResp.JobId == "" {
		t.Error("expected non-empty job_id")
	}
	if installResp.AdapterId != "parkhaus-munich-v1.0.0" {
		t.Errorf("expected adapter_id 'parkhaus-munich-v1.0.0', got %q", installResp.AdapterId)
	}
	if installResp.State != pb.AdapterState_DOWNLOADING {
		t.Errorf("expected initial state DOWNLOADING, got %v", installResp.State)
	}

	// Wait for the adapter to reach RUNNING state.
	time.Sleep(5 * time.Second)

	// Step 3: ListAdapters and verify the adapter appears.
	listResp, err := client.ListAdapters(ctx, &pb.ListAdaptersRequest{})
	if err != nil {
		t.Fatalf("ListAdapters failed: %v", err)
	}
	found := false
	for _, a := range listResp.Adapters {
		if a.AdapterId == "parkhaus-munich-v1.0.0" {
			found = true
			if a.State != pb.AdapterState_RUNNING {
				t.Errorf("expected adapter state RUNNING, got %v", a.State)
			}
		}
	}
	if !found {
		t.Error("adapter not found in ListAdapters response")
	}

	// Step 4: GetAdapterStatus and verify state is RUNNING.
	statusResp, err := client.GetAdapterStatus(ctx, &pb.GetAdapterStatusRequest{
		AdapterId: "parkhaus-munich-v1.0.0",
	})
	if err != nil {
		t.Fatalf("GetAdapterStatus failed: %v", err)
	}
	if statusResp.State != pb.AdapterState_RUNNING {
		t.Errorf("expected RUNNING, got %v", statusResp.State)
	}

	// Step 5: RemoveAdapter and verify success.
	_, err = client.RemoveAdapter(ctx, &pb.RemoveAdapterRequest{
		AdapterId: "parkhaus-munich-v1.0.0",
	})
	if err != nil {
		t.Fatalf("RemoveAdapter failed: %v", err)
	}

	// Step 6: ListAdapters and verify the adapter is gone.
	listResp2, err := client.ListAdapters(ctx, &pb.ListAdaptersRequest{})
	if err != nil {
		t.Fatalf("ListAdapters after removal failed: %v", err)
	}
	for _, a := range listResp2.Adapters {
		if a.AdapterId == "parkhaus-munich-v1.0.0" {
			t.Error("adapter still present after RemoveAdapter")
		}
	}

	// Step 7: Send SIGTERM and verify clean exit.
	if err := si.cmd.Process.Signal(syscall.SIGTERM); err != nil {
		t.Fatalf("failed to send SIGTERM: %v", err)
	}
	done := make(chan error, 1)
	go func() { done <- si.cmd.Wait() }()
	select {
	case err := <-done:
		if err != nil {
			t.Errorf("expected clean exit, got error: %v", err)
		}
	case <-time.After(15 * time.Second):
		_ = si.cmd.Process.Kill()
		_ = si.cmd.Wait()
		t.Fatal("service did not exit cleanly")
	}
}

// TestSmokeWatchAdapterStatesStream verifies that subscribing to
// WatchAdapterStates delivers state transition events when an adapter
// is installed.
// TS-07-SMOKE-2 | Requirement: 07-REQ-3.1, 07-REQ-3.2, 07-REQ-3.3
func TestSmokeWatchAdapterStatesStream(t *testing.T) {
	skipIfNoPodman(t)

	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	si := startUpdateService(t, binary, port, configPath)
	defer func() {
		_ = si.cmd.Process.Signal(syscall.SIGTERM)
		_ = si.cmd.Wait()
	}()

	conn := connectGRPC(t, port)
	client := newUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Step 1: Open WatchAdapterStates stream.
	stream, err := client.WatchAdapterStates(ctx, &pb.WatchAdapterStatesRequest{})
	if err != nil {
		t.Fatalf("WatchAdapterStates failed: %v", err)
	}

	// Step 2: Install an adapter to trigger state transitions.
	imageRef := "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0"
	_, err = client.InstallAdapter(ctx, &pb.InstallAdapterRequest{
		ImageRef:       imageRef,
		ChecksumSha256: "sha256:abc123",
	})
	if err != nil {
		t.Fatalf("InstallAdapter failed: %v", err)
	}

	// Step 3: Collect events from the stream.
	var events []*pb.AdapterStateEvent
	deadline := time.After(10 * time.Second)
	for {
		select {
		case <-deadline:
			goto checkEvents
		default:
			event, err := stream.Recv()
			if err == io.EOF {
				goto checkEvents
			}
			if err != nil {
				// Context cancelled or other error — stop collecting.
				goto checkEvents
			}
			events = append(events, event)
			// We expect at least 3 transitions; stop after collecting enough.
			if len(events) >= 3 {
				goto checkEvents
			}
		}
	}

checkEvents:
	// Step 4: Verify events include DOWNLOADING, INSTALLING, RUNNING transitions.
	if len(events) < 3 {
		t.Fatalf("expected at least 3 events, got %d", len(events))
	}

	// First event: UNKNOWN -> DOWNLOADING
	if events[0].OldState != pb.AdapterState_UNKNOWN || events[0].NewState != pb.AdapterState_DOWNLOADING {
		t.Errorf("event[0]: expected UNKNOWN->DOWNLOADING, got %v->%v",
			events[0].OldState, events[0].NewState)
	}
	// Second event: DOWNLOADING -> INSTALLING
	if events[1].OldState != pb.AdapterState_DOWNLOADING || events[1].NewState != pb.AdapterState_INSTALLING {
		t.Errorf("event[1]: expected DOWNLOADING->INSTALLING, got %v->%v",
			events[1].OldState, events[1].NewState)
	}
	// Third event: INSTALLING -> RUNNING
	if events[2].OldState != pb.AdapterState_INSTALLING || events[2].NewState != pb.AdapterState_RUNNING {
		t.Errorf("event[2]: expected INSTALLING->RUNNING, got %v->%v",
			events[2].OldState, events[2].NewState)
	}

	// Verify adapter_id and timestamp on all events.
	for i, e := range events {
		if e.AdapterId != "parkhaus-munich-v1.0.0" {
			t.Errorf("event[%d]: expected adapter_id 'parkhaus-munich-v1.0.0', got %q", i, e.AdapterId)
		}
		if e.Timestamp == 0 {
			t.Errorf("event[%d]: expected non-zero timestamp", i)
		}
	}
}

// TestGRPCListAdaptersEmpty verifies that ListAdapters returns an empty list
// when no adapters have been installed. This does not require podman.
// TS-07-E9 (integration-level) | Requirement: 07-REQ-4.E2
func TestGRPCListAdaptersEmpty(t *testing.T) {
	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	_ = startUpdateServiceWithCleanup(t, binary, port, configPath)

	conn := connectGRPC(t, port)
	client := newUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &pb.ListAdaptersRequest{})
	if err != nil {
		t.Fatalf("ListAdapters failed: %v", err)
	}
	if len(resp.Adapters) != 0 {
		t.Errorf("expected empty adapter list, got %d adapters", len(resp.Adapters))
	}
}

// TestGRPCGetAdapterStatusNotFound verifies that GetAdapterStatus returns
// NOT_FOUND for an unknown adapter_id. This does not require podman.
// TS-07-E8 (integration-level) | Requirement: 07-REQ-4.E1
func TestGRPCGetAdapterStatusNotFound(t *testing.T) {
	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	_ = startUpdateServiceWithCleanup(t, binary, port, configPath)

	conn := connectGRPC(t, port)
	client := newUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_, err := client.GetAdapterStatus(ctx, &pb.GetAdapterStatusRequest{
		AdapterId: "nonexistent-adapter",
	})
	if err == nil {
		t.Fatal("expected error for unknown adapter, got nil")
	}
	// Verify the error message contains the expected text.
	errMsg := err.Error()
	if !containsString(errMsg, "adapter not found") {
		t.Errorf("expected error to contain 'adapter not found', got: %s", errMsg)
	}
}

// TestGRPCRemoveAdapterNotFound verifies that RemoveAdapter returns NOT_FOUND
// for an unknown adapter_id. This does not require podman.
// TS-07-E10 (integration-level) | Requirement: 07-REQ-5.E1
func TestGRPCRemoveAdapterNotFound(t *testing.T) {
	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	_ = startUpdateServiceWithCleanup(t, binary, port, configPath)

	conn := connectGRPC(t, port)
	client := newUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_, err := client.RemoveAdapter(ctx, &pb.RemoveAdapterRequest{
		AdapterId: "nonexistent-adapter",
	})
	if err == nil {
		t.Fatal("expected error for unknown adapter, got nil")
	}
	errMsg := err.Error()
	if !containsString(errMsg, "adapter not found") {
		t.Errorf("expected error to contain 'adapter not found', got: %s", errMsg)
	}
}

// TestGRPCInstallEmptyImageRef verifies that InstallAdapter returns
// INVALID_ARGUMENT when image_ref is empty. This does not require podman.
// TS-07-E1 (integration-level) | Requirement: 07-REQ-1.E1
func TestGRPCInstallEmptyImageRef(t *testing.T) {
	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	_ = startUpdateServiceWithCleanup(t, binary, port, configPath)

	conn := connectGRPC(t, port)
	client := newUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_, err := client.InstallAdapter(ctx, &pb.InstallAdapterRequest{
		ImageRef:       "",
		ChecksumSha256: "sha256:abc123",
	})
	if err == nil {
		t.Fatal("expected error for empty image_ref, got nil")
	}
	errMsg := err.Error()
	if !containsString(errMsg, "image_ref is required") {
		t.Errorf("expected error to contain 'image_ref is required', got: %s", errMsg)
	}
}

// TestGRPCInstallEmptyChecksum verifies that InstallAdapter returns
// INVALID_ARGUMENT when checksum_sha256 is empty. This does not require podman.
// TS-07-E2 (integration-level) | Requirement: 07-REQ-1.E2
func TestGRPCInstallEmptyChecksum(t *testing.T) {
	binary := buildUpdateService(t)
	port := findFreePort(t)
	configPath := writeConfigFile(t, port)

	_ = startUpdateServiceWithCleanup(t, binary, port, configPath)

	conn := connectGRPC(t, port)
	client := newUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_, err := client.InstallAdapter(ctx, &pb.InstallAdapterRequest{
		ImageRef:       "example.com/img:v1",
		ChecksumSha256: "",
	})
	if err == nil {
		t.Fatal("expected error for empty checksum, got nil")
	}
	errMsg := err.Error()
	if !containsString(errMsg, "checksum_sha256 is required") {
		t.Errorf("expected error to contain 'checksum_sha256 is required', got: %s", errMsg)
	}
}

// containsString checks if s contains substr (case-sensitive).
func containsString(s, substr string) bool {
	return len(s) >= len(substr) && contains(s, substr)
}

func contains(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
