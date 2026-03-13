package cmd

import (
	"context"
	"fmt"
	"net"
	"strings"
	"testing"

	adaptorpb "github.com/parking-fee-service/proto"
	updatepb "github.com/parking-fee-service/proto/update_service/v1"
	"google.golang.org/grpc"
)

// mockUpdateService implements the UpdateService gRPC server for testing.
type mockUpdateService struct {
	updatepb.UnimplementedUpdateServiceServer

	installCalled   bool
	listCalled      bool
	removeCalled    bool
	getStatusCalled bool
	watchCalled     bool

	capturedImageRef  string
	capturedChecksum  string
	capturedAdapterID string
}

func (m *mockUpdateService) InstallAdapter(_ context.Context, req *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
	m.installCalled = true
	m.capturedImageRef = req.GetImageRef()
	m.capturedChecksum = req.GetChecksumSha256()
	return &updatepb.InstallAdapterResponse{
		JobId:     "job-001",
		AdapterId: "adapter-001",
		State:     updatepb.AdapterState_ADAPTER_STATE_DOWNLOADING,
	}, nil
}

func (m *mockUpdateService) WatchAdapterStates(_ *updatepb.WatchAdapterStatesRequest, stream updatepb.UpdateService_WatchAdapterStatesServer) error {
	m.watchCalled = true
	// Send 2 events then close
	for i := 0; i < 2; i++ {
		err := stream.Send(&updatepb.AdapterStateEvent{
			AdapterId: fmt.Sprintf("adapter-%03d", i),
			OldState:  updatepb.AdapterState_ADAPTER_STATE_DOWNLOADING,
			NewState:  updatepb.AdapterState_ADAPTER_STATE_RUNNING,
			Timestamp: int64(1700000000 + i),
		})
		if err != nil {
			return err
		}
	}
	return nil
}

func (m *mockUpdateService) ListAdapters(_ context.Context, _ *updatepb.ListAdaptersRequest) (*updatepb.ListAdaptersResponse, error) {
	m.listCalled = true
	return &updatepb.ListAdaptersResponse{
		Adapters: []*updatepb.AdapterInfo{
			{
				AdapterId: "adapter-001",
				ImageRef:  "ghcr.io/demo:v1",
				State:     updatepb.AdapterState_ADAPTER_STATE_RUNNING,
			},
		},
	}, nil
}

func (m *mockUpdateService) RemoveAdapter(_ context.Context, req *updatepb.RemoveAdapterRequest) (*updatepb.RemoveAdapterResponse, error) {
	m.removeCalled = true
	m.capturedAdapterID = req.GetAdapterId()
	return &updatepb.RemoveAdapterResponse{
		Success: true,
	}, nil
}

func (m *mockUpdateService) GetAdapterStatus(_ context.Context, req *updatepb.GetAdapterStatusRequest) (*updatepb.GetAdapterStatusResponse, error) {
	m.getStatusCalled = true
	m.capturedAdapterID = req.GetAdapterId()
	return &updatepb.GetAdapterStatusResponse{
		AdapterId:    req.GetAdapterId(),
		ImageRef:     "ghcr.io/demo:v1",
		State:        updatepb.AdapterState_ADAPTER_STATE_RUNNING,
		ErrorMessage: "",
	}, nil
}

// mockParkingAdaptor implements the ParkingAdaptor gRPC server for testing.
type mockParkingAdaptor struct {
	adaptorpb.UnimplementedParkingAdaptorServer

	startSessionCalled bool
	stopSessionCalled  bool
	capturedZoneID     string
}

func (m *mockParkingAdaptor) StartSession(_ context.Context, req *adaptorpb.StartSessionRequest) (*adaptorpb.StartSessionResponse, error) {
	m.startSessionCalled = true
	m.capturedZoneID = req.GetZoneId()
	return &adaptorpb.StartSessionResponse{
		SessionId: "sess-001",
		Status:    "active",
	}, nil
}

func (m *mockParkingAdaptor) StopSession(_ context.Context, _ *adaptorpb.StopSessionRequest) (*adaptorpb.StopSessionResponse, error) {
	m.stopSessionCalled = true
	return &adaptorpb.StopSessionResponse{
		SessionId:       "sess-001",
		DurationSeconds: 3600,
		Fee:             2.50,
		Status:          "stopped",
	}, nil
}

// startMockUpdateServer starts a mock UpdateService gRPC server on a random port.
func startMockUpdateServer(t *testing.T) (*mockUpdateService, string, func()) {
	t.Helper()
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}
	srv := grpc.NewServer()
	mock := &mockUpdateService{}
	updatepb.RegisterUpdateServiceServer(srv, mock)
	go srv.Serve(lis)
	return mock, lis.Addr().String(), func() { srv.Stop() }
}

// startMockAdaptorServer starts a mock ParkingAdaptor gRPC server on a random port.
func startMockAdaptorServer(t *testing.T) (*mockParkingAdaptor, string, func()) {
	t.Helper()
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}
	srv := grpc.NewServer()
	mock := &mockParkingAdaptor{}
	adaptorpb.RegisterParkingAdaptorServer(srv, mock)
	go srv.Serve(lis)
	return mock, lis.Addr().String(), func() { srv.Stop() }
}

// TS-09-14: install calls UPDATE_SERVICE InstallAdapter gRPC.
func TestInstall_gRPC(t *testing.T) {
	mock, addr, cleanup := startMockUpdateServer(t)
	defer cleanup()

	err := RunInstall([]string{"--image-ref=ghcr.io/demo:v1", "--checksum=abc123"}, addr)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !mock.installCalled {
		t.Error("InstallAdapter was not called")
	}
	if mock.capturedImageRef != "ghcr.io/demo:v1" {
		t.Errorf("expected image_ref ghcr.io/demo:v1, got %q", mock.capturedImageRef)
	}
	if mock.capturedChecksum != "abc123" {
		t.Errorf("expected checksum abc123, got %q", mock.capturedChecksum)
	}
}

// TS-09-15: watch streams WatchAdapterStates events.
func TestWatch_gRPC(t *testing.T) {
	mock, addr, cleanup := startMockUpdateServer(t)
	defer cleanup()

	err := RunWatch(nil, addr)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !mock.watchCalled {
		t.Error("WatchAdapterStates was not called")
	}
}

// TS-09-16: list calls UPDATE_SERVICE ListAdapters.
func TestList_gRPC(t *testing.T) {
	mock, addr, cleanup := startMockUpdateServer(t)
	defer cleanup()

	err := RunList(nil, addr)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !mock.listCalled {
		t.Error("ListAdapters was not called")
	}
}

// TS-09-17: remove calls UPDATE_SERVICE RemoveAdapter with adapter_id.
func TestRemove_gRPC(t *testing.T) {
	mock, addr, cleanup := startMockUpdateServer(t)
	defer cleanup()

	err := RunRemove([]string{"--adapter-id=adapter-001"}, addr)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !mock.removeCalled {
		t.Error("RemoveAdapter was not called")
	}
	if mock.capturedAdapterID != "adapter-001" {
		t.Errorf("expected adapter_id adapter-001, got %q", mock.capturedAdapterID)
	}
}

// TS-09-18: status calls UPDATE_SERVICE GetAdapterStatus with adapter_id.
func TestAdapterStatus_gRPC(t *testing.T) {
	mock, addr, cleanup := startMockUpdateServer(t)
	defer cleanup()

	err := RunStatus([]string{"--adapter-id=adapter-001"}, addr)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !mock.getStatusCalled {
		t.Error("GetAdapterStatus was not called")
	}
	if mock.capturedAdapterID != "adapter-001" {
		t.Errorf("expected adapter_id adapter-001, got %q", mock.capturedAdapterID)
	}
}

// TS-09-19: start-session calls PARKING_OPERATOR_ADAPTOR StartSession with zone_id.
func TestStartSession_gRPC(t *testing.T) {
	mock, addr, cleanup := startMockAdaptorServer(t)
	defer cleanup()

	err := RunStartSession([]string{"--zone-id=zone-demo-1"}, addr)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !mock.startSessionCalled {
		t.Error("StartSession was not called")
	}
	if mock.capturedZoneID != "zone-demo-1" {
		t.Errorf("expected zone_id zone-demo-1, got %q", mock.capturedZoneID)
	}
}

// TS-09-20: stop-session calls PARKING_OPERATOR_ADAPTOR StopSession.
func TestStopSession_gRPC(t *testing.T) {
	mock, addr, cleanup := startMockAdaptorServer(t)
	defer cleanup()

	err := RunStopSession(nil, addr)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !mock.stopSessionCalled {
		t.Error("StopSession was not called")
	}
}

// TS-09-E10: Upstream gRPC unreachable exits with error.
func TestInstall_UpstreamUnreachable(t *testing.T) {
	// Use a valid but non-listening address
	addr := "localhost:19998"
	// With gRPC lazy connect, we need to actually make the RPC call to get an error
	err := RunInstall([]string{"--image-ref=test:v1", "--checksum=abc"}, addr)
	if err == nil {
		t.Fatal("expected error when upstream is unreachable")
	}
}

// TS-09-P3: CLI subcommand dispatch property test.
func TestPropertySubcommandDispatch(t *testing.T) {
	known := []string{
		"lookup", "adapter-info", "install", "watch", "list",
		"remove", "status", "start-session", "stop-session",
	}

	// Known subcommands should not produce "unknown" error
	for _, subcmd := range known {
		err := Dispatch(subcmd, nil, "", "", "")
		if err != nil && strings.Contains(err.Error(), "unknown") {
			t.Errorf("known subcommand %q dispatched as unknown", subcmd)
		}
	}

	// Unknown subcommands should produce "unknown" error
	unknowns := []string{"foobar", "delete", "update", "create", "help", ""}
	for _, subcmd := range unknowns {
		err := Dispatch(subcmd, nil, "", "", "")
		if err == nil {
			t.Errorf("unknown subcommand %q should produce error", subcmd)
			continue
		}
		if !strings.Contains(err.Error(), "unknown") {
			t.Errorf("unknown subcommand %q error should mention 'unknown', got: %v", subcmd, err)
		}
	}
}

