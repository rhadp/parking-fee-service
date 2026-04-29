package mockapps_test

import (
	"context"
	"fmt"
	"net"
	"strings"
	"testing"

	adaptorpb "github.com/rhadp/parking-fee-service/gen/adapter"
	updatepb "github.com/rhadp/parking-fee-service/gen/update"

	"google.golang.org/grpc"
)

// ---------------------------------------------------------------------------
// Stub UpdateService gRPC server
// ---------------------------------------------------------------------------

type stubUpdateService struct {
	updatepb.UnimplementedUpdateServiceServer
}

func (s *stubUpdateService) InstallAdapter(_ context.Context, req *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
	return &updatepb.InstallAdapterResponse{
		JobId:     "j1",
		AdapterId: "a1",
		State:     updatepb.AdapterState_DOWNLOADING,
	}, nil
}

func (s *stubUpdateService) ListAdapters(_ context.Context, _ *updatepb.ListAdaptersRequest) (*updatepb.ListAdaptersResponse, error) {
	return &updatepb.ListAdaptersResponse{
		Adapters: []*updatepb.AdapterInfo{
			{AdapterId: "a1", ImageRef: "registry/adapter:v1", State: updatepb.AdapterState_RUNNING},
		},
	}, nil
}

func (s *stubUpdateService) GetAdapterStatus(_ context.Context, req *updatepb.GetAdapterStatusRequest) (*updatepb.GetAdapterStatusResponse, error) {
	return &updatepb.GetAdapterStatusResponse{
		Adapter: &updatepb.AdapterInfo{
			AdapterId: req.GetAdapterId(),
			ImageRef:  "registry/adapter:v1",
			State:     updatepb.AdapterState_RUNNING,
		},
	}, nil
}

func (s *stubUpdateService) RemoveAdapter(_ context.Context, req *updatepb.RemoveAdapterRequest) (*updatepb.RemoveAdapterResponse, error) {
	return &updatepb.RemoveAdapterResponse{
		AdapterId: req.GetAdapterId(),
		State:     updatepb.AdapterState_OFFLOADING,
	}, nil
}

// startStubUpdateService starts a stub UpdateService gRPC server on a random port.
func startStubUpdateService(t *testing.T) string {
	t.Helper()

	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}

	srv := grpc.NewServer()
	updatepb.RegisterUpdateServiceServer(srv, &stubUpdateService{})

	go func() {
		if err := srv.Serve(lis); err != nil {
			// Expected when GracefulStop is called during cleanup.
		}
	}()

	t.Cleanup(func() { srv.GracefulStop() })

	return lis.Addr().String()
}

// ---------------------------------------------------------------------------
// Stub ParkingAdaptorService gRPC server
// ---------------------------------------------------------------------------

type stubAdaptorService struct {
	adaptorpb.UnimplementedParkingOperatorAdaptorServiceServer
}

func (s *stubAdaptorService) StartSession(_ context.Context, req *adaptorpb.StartSessionRequest) (*adaptorpb.StartSessionResponse, error) {
	return &adaptorpb.StartSessionResponse{
		Session: &adaptorpb.SessionStatus{
			SessionId: "s1",
			Active:    true,
			ZoneId:    req.GetZoneId(),
			StartTime: 1700000000,
		},
	}, nil
}

func (s *stubAdaptorService) StopSession(_ context.Context, _ *adaptorpb.StopSessionRequest) (*adaptorpb.StopSessionResponse, error) {
	return &adaptorpb.StopSessionResponse{
		Session: &adaptorpb.SessionStatus{
			SessionId: "s1",
			Active:    false,
			ZoneId:    "zone-demo-1",
		},
	}, nil
}

// startStubAdaptorService starts a stub ParkingAdaptorService gRPC server on a random port.
func startStubAdaptorService(t *testing.T) string {
	t.Helper()

	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}

	srv := grpc.NewServer()
	adaptorpb.RegisterParkingOperatorAdaptorServiceServer(srv, &stubAdaptorService{})

	go func() {
		if err := srv.Serve(lis); err != nil {
			// Expected when GracefulStop is called during cleanup.
		}
	}()

	t.Cleanup(func() { srv.GracefulStop() })

	return lis.Addr().String()
}

// ---------------------------------------------------------------------------
// TS-09-7 (with mock gRPC): Parking App CLI Install Adapter
// Requirement: 09-REQ-5.1, 09-REQ-5.6
// ---------------------------------------------------------------------------

func TestInstallWithMockGRPC(t *testing.T) {
	addr := startStubUpdateService(t)
	bin := parkingAppBinary(t)

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"install",
		"--image-ref=registry/adapter:v1",
		"--checksum=sha256:abc",
		"--update-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	if !strings.Contains(stdout, "j1") {
		t.Errorf("expected 'j1' (job_id) in stdout, got: %s", stdout)
	}
	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected 'a1' (adapter_id) in stdout, got: %s", stdout)
	}
	if !strings.Contains(stdout, "DOWNLOADING") {
		t.Errorf("expected 'DOWNLOADING' (state) in stdout, got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-8 (with mock gRPC): Parking App CLI List Adapters
// Requirement: 09-REQ-5.2
// ---------------------------------------------------------------------------

func TestListWithMockGRPC(t *testing.T) {
	addr := startStubUpdateService(t)
	bin := parkingAppBinary(t)

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"list",
		"--update-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected 'a1' (adapter_id) in stdout, got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-18 (with mock gRPC): Parking App CLI Get Adapter Status
// Requirement: 09-REQ-5.4
// ---------------------------------------------------------------------------

func TestAdapterStatusWithMockGRPC(t *testing.T) {
	addr := startStubUpdateService(t)
	bin := parkingAppBinary(t)

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"status",
		"--adapter-id=a1",
		"--update-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected 'a1' (adapter_id) in stdout, got: %s", stdout)
	}
	if !strings.Contains(stdout, "RUNNING") {
		t.Errorf("expected 'RUNNING' (state) in stdout, got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-19 (with mock gRPC): Parking App CLI Remove Adapter
// Requirement: 09-REQ-5.5
// ---------------------------------------------------------------------------

func TestRemoveWithMockGRPC(t *testing.T) {
	addr := startStubUpdateService(t)
	bin := parkingAppBinary(t)

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"remove",
		"--adapter-id=a1",
		"--update-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected 'a1' (adapter_id) in stdout, got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-9 (with mock gRPC): Parking App CLI Start Session Override
// Requirement: 09-REQ-6.1, 09-REQ-6.3
// ---------------------------------------------------------------------------

func TestStartSessionWithMockGRPC(t *testing.T) {
	addr := startStubAdaptorService(t)
	bin := parkingAppBinary(t)

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"start-session",
		"--zone-id=zone-demo-1",
		"--adaptor-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	if !strings.Contains(stdout, "s1") {
		t.Errorf("expected 's1' (session_id) in stdout, got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-10 (with mock gRPC): Parking App CLI Stop Session Override
// Requirement: 09-REQ-6.2
// ---------------------------------------------------------------------------

func TestStopSessionWithMockGRPC(t *testing.T) {
	addr := startStubAdaptorService(t)
	bin := parkingAppBinary(t)

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"stop-session",
		"--adaptor-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	if !strings.Contains(stdout, "s1") {
		t.Errorf("expected 's1' (session_id) in stdout, got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E10 (enhanced): gRPC error for all parking-app-cli gRPC subcommands
// Requirement: 09-REQ-5.E2, 09-REQ-6.E1
// ---------------------------------------------------------------------------

func TestParkingAppGRPCErrors(t *testing.T) {
	bin := parkingAppBinary(t)
	env := baseEnv()

	// Use a non-listening port to force connection failure.
	freePort := getFreePort(t)
	deadAddr := fmt.Sprintf("localhost:%d", freePort)

	tests := []struct {
		name string
		args []string
	}{
		{"install", []string{"install", "--image-ref=x", "--checksum=y", "--update-addr=" + deadAddr}},
		{"list", []string{"list", "--update-addr=" + deadAddr}},
		{"status", []string{"status", "--adapter-id=a1", "--update-addr=" + deadAddr}},
		{"remove", []string{"remove", "--adapter-id=a1", "--update-addr=" + deadAddr}},
		{"start-session", []string{"start-session", "--zone-id=z1", "--adaptor-addr=" + deadAddr}},
		{"stop-session", []string{"stop-session", "--adaptor-addr=" + deadAddr}},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			_, stderr, exitCode := runBinary(t, bin, tc.args, env)

			if exitCode != 1 {
				t.Errorf("expected exit 1 on gRPC error, got %d", exitCode)
			}
			if len(stderr) == 0 {
				t.Error("expected stderr to contain error message")
			}
		})
	}
}
