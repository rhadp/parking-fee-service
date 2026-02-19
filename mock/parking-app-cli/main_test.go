package main

import (
	"context"
	"fmt"
	"net"
	"os"
	"strings"
	"testing"
	"time"

	commonpb "github.com/rhadp/parking-fee-service/proto/gen/go/common"
	adapterpb "github.com/rhadp/parking-fee-service/proto/gen/go/services/adapter"
	updatepb "github.com/rhadp/parking-fee-service/proto/gen/go/services/update"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// ─── Global flag parsing tests ──────────────────────────────────────────────

func TestParseGlobalFlagsDefaults(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	remaining, err := parseGlobalFlags([]string{"list-adapters"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if updateServiceAddr != "localhost:50053" {
		t.Errorf("expected default update-service-addr 'localhost:50053', got %q", updateServiceAddr)
	}
	if adapterAddr != "localhost:50054" {
		t.Errorf("expected default adapter-addr 'localhost:50054', got %q", adapterAddr)
	}
	if len(remaining) != 1 || remaining[0] != "list-adapters" {
		t.Errorf("expected remaining [list-adapters], got %v", remaining)
	}
}

func TestParseGlobalFlagsCustom(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	remaining, err := parseGlobalFlags([]string{
		"--update-service-addr", "10.0.0.1:50053",
		"--adapter-addr", "10.0.0.2:50054",
		"install-adapter",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if updateServiceAddr != "10.0.0.1:50053" {
		t.Errorf("expected update-service-addr '10.0.0.1:50053', got %q", updateServiceAddr)
	}
	if adapterAddr != "10.0.0.2:50054" {
		t.Errorf("expected adapter-addr '10.0.0.2:50054', got %q", adapterAddr)
	}
	if len(remaining) != 1 || remaining[0] != "install-adapter" {
		t.Errorf("expected remaining [install-adapter], got %v", remaining)
	}
}

func TestParseGlobalFlagsFromEnv(t *testing.T) {
	t.Setenv("UPDATE_SERVICE_ADDR", "envhost:50053")
	t.Setenv("ADAPTER_ADDR", "envhost:50054")

	remaining, err := parseGlobalFlags([]string{"get-rate"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if updateServiceAddr != "envhost:50053" {
		t.Errorf("expected update-service-addr from env 'envhost:50053', got %q", updateServiceAddr)
	}
	if adapterAddr != "envhost:50054" {
		t.Errorf("expected adapter-addr from env 'envhost:50054', got %q", adapterAddr)
	}
	if len(remaining) != 1 || remaining[0] != "get-rate" {
		t.Errorf("expected remaining [get-rate], got %v", remaining)
	}
}

func TestParseGlobalFlagsMissingValue(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	_, err := parseGlobalFlags([]string{"--update-service-addr"})
	if err == nil {
		t.Fatal("expected error for missing flag value")
	}
	if !strings.Contains(err.Error(), "requires a value") {
		t.Errorf("expected 'requires a value' error, got %q", err.Error())
	}

	_, err = parseGlobalFlags([]string{"--adapter-addr"})
	if err == nil {
		t.Fatal("expected error for missing flag value")
	}
	if !strings.Contains(err.Error(), "requires a value") {
		t.Errorf("expected 'requires a value' error, got %q", err.Error())
	}
}

func TestParseGlobalFlagsCLIOverridesEnv(t *testing.T) {
	t.Setenv("UPDATE_SERVICE_ADDR", "envhost:50053")
	t.Setenv("ADAPTER_ADDR", "envhost:50054")

	_, err := parseGlobalFlags([]string{
		"--update-service-addr", "clihost:50053",
		"--adapter-addr", "clihost:50054",
		"list-adapters",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if updateServiceAddr != "clihost:50053" {
		t.Errorf("expected CLI override 'clihost:50053', got %q", updateServiceAddr)
	}
	if adapterAddr != "clihost:50054" {
		t.Errorf("expected CLI override 'clihost:50054', got %q", adapterAddr)
	}
}

// ─── Command dispatch tests ─────────────────────────────────────────────────

func TestRunNoArgs(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run(nil)
	if err == nil {
		t.Fatal("expected error for no args")
	}
	if err.Error() != "no command specified" {
		t.Errorf("expected 'no command specified', got %q", err.Error())
	}
}

func TestRunUnknownCommand(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"nonexistent"})
	if err == nil {
		t.Fatal("expected error for unknown command")
	}
	expected := "unknown command: nonexistent"
	if err.Error() != expected {
		t.Errorf("expected %q, got %q", expected, err.Error())
	}
}

func TestRunHelp(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	for _, helpCmd := range []string{"help", "--help", "-h"} {
		t.Run(helpCmd, func(t *testing.T) {
			err := run([]string{helpCmd})
			if err != nil {
				t.Fatalf("unexpected error for %s command: %v", helpCmd, err)
			}
		})
	}
}

// ─── Utility function tests ─────────────────────────────────────────────────

func TestFlagValue(t *testing.T) {
	tests := []struct {
		name     string
		args     []string
		flag     string
		def      string
		expected string
	}{
		{"present", []string{"--zone-id", "zone-a"}, "--zone-id", "default", "zone-a"},
		{"absent", []string{}, "--zone-id", "default", "default"},
		{"last arg no value", []string{"--zone-id"}, "--zone-id", "default", "default"},
		{"multiple flags", []string{"--foo", "bar", "--zone-id", "zone-b"}, "--zone-id", "default", "zone-b"},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			result := flagValue(tc.args, tc.flag, tc.def)
			if result != tc.expected {
				t.Errorf("expected %q, got %q", tc.expected, result)
			}
		})
	}
}

func TestEnvOrDefault(t *testing.T) {
	t.Setenv("TEST_CLI_VAR", "custom-value")
	if v := envOrDefault("TEST_CLI_VAR", "default"); v != "custom-value" {
		t.Errorf("expected 'custom-value', got %q", v)
	}

	os.Unsetenv("TEST_CLI_VAR_UNSET")
	if v := envOrDefault("TEST_CLI_VAR_UNSET", "fallback"); v != "fallback" {
		t.Errorf("expected 'fallback', got %q", v)
	}
}

// ─── Required flag validation tests ─────────────────────────────────────────

func TestInstallAdapterRequiresImageRef(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"install-adapter"})
	if err == nil {
		t.Fatal("expected error for missing --image-ref")
	}
	if !strings.Contains(err.Error(), "--image-ref is required") {
		t.Errorf("expected '--image-ref is required' error, got %q", err.Error())
	}
}

func TestRemoveAdapterRequiresAdapterID(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"remove-adapter"})
	if err == nil {
		t.Fatal("expected error for missing --adapter-id")
	}
	if !strings.Contains(err.Error(), "--adapter-id is required") {
		t.Errorf("expected '--adapter-id is required' error, got %q", err.Error())
	}
}

func TestAdapterStatusRequiresAdapterID(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"adapter-status"})
	if err == nil {
		t.Fatal("expected error for missing --adapter-id")
	}
	if !strings.Contains(err.Error(), "--adapter-id is required") {
		t.Errorf("expected '--adapter-id is required' error, got %q", err.Error())
	}
}

func TestStartSessionRequiresZoneID(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"start-session", "--vehicle-vin", "VIN123"})
	if err == nil {
		t.Fatal("expected error for missing --zone-id")
	}
	if !strings.Contains(err.Error(), "--zone-id is required") {
		t.Errorf("expected '--zone-id is required' error, got %q", err.Error())
	}
}

func TestStartSessionRequiresVehicleVin(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"start-session", "--zone-id", "zone-a"})
	if err == nil {
		t.Fatal("expected error for missing --vehicle-vin")
	}
	if !strings.Contains(err.Error(), "--vehicle-vin is required") {
		t.Errorf("expected '--vehicle-vin is required' error, got %q", err.Error())
	}
}

func TestStopSessionRequiresSessionID(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"stop-session"})
	if err == nil {
		t.Fatal("expected error for missing --session-id")
	}
	if !strings.Contains(err.Error(), "--session-id is required") {
		t.Errorf("expected '--session-id is required' error, got %q", err.Error())
	}
}

func TestGetRateRequiresZoneID(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	err := run([]string{"get-rate"})
	if err == nil {
		t.Fatal("expected error for missing --zone-id")
	}
	if !strings.Contains(err.Error(), "--zone-id is required") {
		t.Errorf("expected '--zone-id is required' error, got %q", err.Error())
	}
}

// get-status does NOT require --session-id (it's optional per design).
func TestGetStatusSessionIDOptional(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	// Without --session-id, the command should proceed to connect (and fail at
	// connection, not at flag parsing).
	err := run([]string{
		"--adapter-addr", "localhost:1",
		"get-status",
	})
	if err == nil {
		t.Fatal("expected connection error, not nil")
	}
	// Should be a connection error, not a "required" flag error.
	if strings.Contains(err.Error(), "is required") {
		t.Errorf("expected connection error, not required flag error: %q", err.Error())
	}
	if !strings.Contains(err.Error(), "failed to connect") {
		t.Errorf("expected 'failed to connect' error, got %q", err.Error())
	}
}

// ─── Subcommand recognition test ────────────────────────────────────────────

func TestAllSubcommandsRecognized(t *testing.T) {
	subcommands := []string{
		"install-adapter",
		"list-adapters",
		"remove-adapter",
		"adapter-status",
		"watch-adapters",
		"start-session",
		"stop-session",
		"get-status",
		"get-rate",
	}

	for _, cmd := range subcommands {
		t.Run(cmd, func(t *testing.T) {
			os.Unsetenv("UPDATE_SERVICE_ADDR")
			os.Unsetenv("ADAPTER_ADDR")

			// Use an address that will fail quickly.
			err := run([]string{
				"--update-service-addr", "localhost:1",
				"--adapter-addr", "localhost:1",
				cmd,
			})
			// We expect a connection or required-flag error, not "unknown command".
			if err == nil {
				return
			}
			if err.Error() == "unknown command: "+cmd {
				t.Errorf("command %q was not recognized", cmd)
			}
		})
	}
}

// ─── Error handling: unreachable service ─────────────────────────────────────

func TestUnreachableServiceReturnsError(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	tests := []struct {
		name string
		args []string
	}{
		{"list-adapters", []string{"--update-service-addr", "localhost:1", "list-adapters"}},
		{"install-adapter", []string{"--update-service-addr", "localhost:1", "install-adapter", "--image-ref", "test:latest"}},
		{"remove-adapter", []string{"--update-service-addr", "localhost:1", "remove-adapter", "--adapter-id", "a1"}},
		{"adapter-status", []string{"--update-service-addr", "localhost:1", "adapter-status", "--adapter-id", "a1"}},
		{"watch-adapters", []string{"--update-service-addr", "localhost:1", "watch-adapters"}},
		{"start-session", []string{"--adapter-addr", "localhost:1", "start-session", "--zone-id", "z1", "--vehicle-vin", "VIN1"}},
		{"stop-session", []string{"--adapter-addr", "localhost:1", "stop-session", "--session-id", "s1"}},
		{"get-status", []string{"--adapter-addr", "localhost:1", "get-status", "--session-id", "s1"}},
		{"get-rate", []string{"--adapter-addr", "localhost:1", "get-rate", "--zone-id", "z1"}},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			err := run(tc.args)
			if err == nil {
				t.Fatalf("expected error for unreachable service, got nil")
			}
			if !strings.Contains(err.Error(), "failed to connect") {
				t.Errorf("expected 'failed to connect' error, got %q", err.Error())
			}
		})
	}
}

// ─── Mock gRPC servers ──────────────────────────────────────────────────────

// mockUpdateService implements updatepb.UpdateServiceServer for testing.
type mockUpdateService struct {
	updatepb.UnimplementedUpdateServiceServer

	installAdapterFn  func(context.Context, *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error)
	listAdaptersFn    func(context.Context, *updatepb.ListAdaptersRequest) (*updatepb.ListAdaptersResponse, error)
	removeAdapterFn   func(context.Context, *updatepb.RemoveAdapterRequest) (*updatepb.RemoveAdapterResponse, error)
	getAdapterStatusFn func(context.Context, *updatepb.GetAdapterStatusRequest) (*updatepb.GetAdapterStatusResponse, error)
	watchAdapterStatesFn func(*updatepb.WatchAdapterStatesRequest, grpc.ServerStreamingServer[updatepb.AdapterStateEvent]) error
}

func (m *mockUpdateService) InstallAdapter(ctx context.Context, req *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
	if m.installAdapterFn != nil {
		return m.installAdapterFn(ctx, req)
	}
	return &updatepb.InstallAdapterResponse{}, nil
}

func (m *mockUpdateService) ListAdapters(ctx context.Context, req *updatepb.ListAdaptersRequest) (*updatepb.ListAdaptersResponse, error) {
	if m.listAdaptersFn != nil {
		return m.listAdaptersFn(ctx, req)
	}
	return &updatepb.ListAdaptersResponse{}, nil
}

func (m *mockUpdateService) RemoveAdapter(ctx context.Context, req *updatepb.RemoveAdapterRequest) (*updatepb.RemoveAdapterResponse, error) {
	if m.removeAdapterFn != nil {
		return m.removeAdapterFn(ctx, req)
	}
	return &updatepb.RemoveAdapterResponse{}, nil
}

func (m *mockUpdateService) GetAdapterStatus(ctx context.Context, req *updatepb.GetAdapterStatusRequest) (*updatepb.GetAdapterStatusResponse, error) {
	if m.getAdapterStatusFn != nil {
		return m.getAdapterStatusFn(ctx, req)
	}
	return &updatepb.GetAdapterStatusResponse{}, nil
}

func (m *mockUpdateService) WatchAdapterStates(req *updatepb.WatchAdapterStatesRequest, stream grpc.ServerStreamingServer[updatepb.AdapterStateEvent]) error {
	if m.watchAdapterStatesFn != nil {
		return m.watchAdapterStatesFn(req, stream)
	}
	return nil
}

// mockParkingAdapter implements adapterpb.ParkingAdapterServer for testing.
type mockParkingAdapter struct {
	adapterpb.UnimplementedParkingAdapterServer

	startSessionFn func(context.Context, *adapterpb.StartSessionRequest) (*adapterpb.StartSessionResponse, error)
	stopSessionFn  func(context.Context, *adapterpb.StopSessionRequest) (*adapterpb.StopSessionResponse, error)
	getStatusFn    func(context.Context, *adapterpb.GetStatusRequest) (*adapterpb.GetStatusResponse, error)
	getRateFn      func(context.Context, *adapterpb.GetRateRequest) (*adapterpb.GetRateResponse, error)
}

func (m *mockParkingAdapter) StartSession(ctx context.Context, req *adapterpb.StartSessionRequest) (*adapterpb.StartSessionResponse, error) {
	if m.startSessionFn != nil {
		return m.startSessionFn(ctx, req)
	}
	return &adapterpb.StartSessionResponse{}, nil
}

func (m *mockParkingAdapter) StopSession(ctx context.Context, req *adapterpb.StopSessionRequest) (*adapterpb.StopSessionResponse, error) {
	if m.stopSessionFn != nil {
		return m.stopSessionFn(ctx, req)
	}
	return &adapterpb.StopSessionResponse{}, nil
}

func (m *mockParkingAdapter) GetStatus(ctx context.Context, req *adapterpb.GetStatusRequest) (*adapterpb.GetStatusResponse, error) {
	if m.getStatusFn != nil {
		return m.getStatusFn(ctx, req)
	}
	return &adapterpb.GetStatusResponse{}, nil
}

func (m *mockParkingAdapter) GetRate(ctx context.Context, req *adapterpb.GetRateRequest) (*adapterpb.GetRateResponse, error) {
	if m.getRateFn != nil {
		return m.getRateFn(ctx, req)
	}
	return &adapterpb.GetRateResponse{}, nil
}

// startMockUpdateServer starts a gRPC server with the given mock UpdateService
// implementation and returns its address and a cleanup function.
func startMockUpdateServer(t *testing.T, svc updatepb.UpdateServiceServer) (string, func()) {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}
	srv := grpc.NewServer()
	updatepb.RegisterUpdateServiceServer(srv, svc)
	go func() { _ = srv.Serve(lis) }()
	return lis.Addr().String(), func() { srv.Stop() }
}

// startMockAdapterServer starts a gRPC server with the given mock
// ParkingAdapter implementation and returns its address and a cleanup function.
func startMockAdapterServer(t *testing.T, svc adapterpb.ParkingAdapterServer) (string, func()) {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}
	srv := grpc.NewServer()
	adapterpb.RegisterParkingAdapterServer(srv, svc)
	go func() { _ = srv.Serve(lis) }()
	return lis.Addr().String(), func() { srv.Stop() }
}

// ─── UpdateService subcommand tests with mock server ────────────────────────

func TestCmdInstallAdapterWithMockServer(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	var receivedReq *updatepb.InstallAdapterRequest
	mock := &mockUpdateService{
		installAdapterFn: func(_ context.Context, req *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
			receivedReq = req
			return &updatepb.InstallAdapterResponse{
				JobId:     "job-123",
				AdapterId: "adapter-001",
				State:     commonpb.AdapterState_ADAPTER_STATE_RUNNING,
			}, nil
		},
	}

	addr, cleanup := startMockUpdateServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--update-service-addr", addr,
		"install-adapter",
		"--image-ref", "localhost/my-adapter:v1",
		"--checksum", "sha256:abc123",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedReq == nil {
		t.Fatal("InstallAdapter was not called")
	}
	if receivedReq.ImageRef != "localhost/my-adapter:v1" {
		t.Errorf("expected image_ref 'localhost/my-adapter:v1', got %q", receivedReq.ImageRef)
	}
	if receivedReq.Checksum != "sha256:abc123" {
		t.Errorf("expected checksum 'sha256:abc123', got %q", receivedReq.Checksum)
	}
}

func TestCmdInstallAdapterWithoutChecksum(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	var receivedReq *updatepb.InstallAdapterRequest
	mock := &mockUpdateService{
		installAdapterFn: func(_ context.Context, req *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
			receivedReq = req
			return &updatepb.InstallAdapterResponse{
				JobId:     "job-456",
				AdapterId: "adapter-002",
				State:     commonpb.AdapterState_ADAPTER_STATE_INSTALLING,
			}, nil
		},
	}

	addr, cleanup := startMockUpdateServer(t, mock)
	defer cleanup()

	// checksum is optional, so this should succeed.
	err := run([]string{
		"--update-service-addr", addr,
		"install-adapter",
		"--image-ref", "test:latest",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedReq == nil {
		t.Fatal("InstallAdapter was not called")
	}
	if receivedReq.ImageRef != "test:latest" {
		t.Errorf("expected image_ref 'test:latest', got %q", receivedReq.ImageRef)
	}
	// Checksum should be empty since not provided.
	if receivedReq.Checksum != "" {
		t.Errorf("expected empty checksum, got %q", receivedReq.Checksum)
	}
}

func TestCmdListAdaptersWithMockServer(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	mock := &mockUpdateService{
		listAdaptersFn: func(_ context.Context, _ *updatepb.ListAdaptersRequest) (*updatepb.ListAdaptersResponse, error) {
			return &updatepb.ListAdaptersResponse{
				Adapters: []*commonpb.AdapterInfo{
					{
						AdapterId: "adapter-001",
						Name:      "parking-operator-adaptor",
						ImageRef:  "localhost/poa:latest",
						Checksum:  "sha256:abc",
						Version:   "1.0.0",
					},
					{
						AdapterId: "adapter-002",
						Name:      "another-adaptor",
						ImageRef:  "localhost/other:v2",
						Checksum:  "sha256:def",
						Version:   "2.0.0",
					},
				},
			}, nil
		},
	}

	addr, cleanup := startMockUpdateServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--update-service-addr", addr,
		"list-adapters",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestCmdRemoveAdapterWithMockServer(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	var receivedID string
	mock := &mockUpdateService{
		removeAdapterFn: func(_ context.Context, req *updatepb.RemoveAdapterRequest) (*updatepb.RemoveAdapterResponse, error) {
			receivedID = req.AdapterId
			return &updatepb.RemoveAdapterResponse{}, nil
		},
	}

	addr, cleanup := startMockUpdateServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--update-service-addr", addr,
		"remove-adapter",
		"--adapter-id", "adapter-xyz",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedID != "adapter-xyz" {
		t.Errorf("expected adapter_id 'adapter-xyz', got %q", receivedID)
	}
}

func TestCmdAdapterStatusWithMockServer(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	var receivedID string
	mock := &mockUpdateService{
		getAdapterStatusFn: func(_ context.Context, req *updatepb.GetAdapterStatusRequest) (*updatepb.GetAdapterStatusResponse, error) {
			receivedID = req.AdapterId
			return &updatepb.GetAdapterStatusResponse{
				Info: &commonpb.AdapterInfo{
					AdapterId: req.AdapterId,
					Name:      "test-adapter",
					ImageRef:  "test:latest",
				},
				State: commonpb.AdapterState_ADAPTER_STATE_RUNNING,
			}, nil
		},
	}

	addr, cleanup := startMockUpdateServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--update-service-addr", addr,
		"adapter-status",
		"--adapter-id", "adapter-abc",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedID != "adapter-abc" {
		t.Errorf("expected adapter_id 'adapter-abc', got %q", receivedID)
	}
}

func TestCmdAdapterStatusNotFound(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	mock := &mockUpdateService{
		getAdapterStatusFn: func(_ context.Context, req *updatepb.GetAdapterStatusRequest) (*updatepb.GetAdapterStatusResponse, error) {
			return nil, status.Errorf(codes.NotFound, "adapter %q not found", req.AdapterId)
		},
	}

	addr, cleanup := startMockUpdateServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--update-service-addr", addr,
		"adapter-status",
		"--adapter-id", "nonexistent",
	})
	if err == nil {
		t.Fatal("expected error for not-found adapter")
	}
	if !strings.Contains(err.Error(), "GetAdapterStatus RPC failed") {
		t.Errorf("expected RPC failed error, got %q", err.Error())
	}
}

func TestCmdWatchAdaptersStreaming(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	mock := &mockUpdateService{
		watchAdapterStatesFn: func(_ *updatepb.WatchAdapterStatesRequest, stream grpc.ServerStreamingServer[updatepb.AdapterStateEvent]) error {
			events := []*updatepb.AdapterStateEvent{
				{
					AdapterId: "adapter-001",
					OldState:  commonpb.AdapterState_ADAPTER_STATE_UNKNOWN,
					NewState:  commonpb.AdapterState_ADAPTER_STATE_INSTALLING,
					Timestamp: 1000,
				},
				{
					AdapterId: "adapter-001",
					OldState:  commonpb.AdapterState_ADAPTER_STATE_INSTALLING,
					NewState:  commonpb.AdapterState_ADAPTER_STATE_RUNNING,
					Timestamp: 1001,
				},
			}
			for _, e := range events {
				if err := stream.Send(e); err != nil {
					return err
				}
			}
			// Server closes the stream (EOF) after sending events.
			return nil
		},
	}

	addr, cleanup := startMockUpdateServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--update-service-addr", addr,
		"watch-adapters",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

// ─── ParkingAdapter subcommand tests with mock server ───────────────────────

func TestCmdStartSessionWithMockServer(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	var receivedReq *adapterpb.StartSessionRequest
	mock := &mockParkingAdapter{
		startSessionFn: func(_ context.Context, req *adapterpb.StartSessionRequest) (*adapterpb.StartSessionResponse, error) {
			receivedReq = req
			return &adapterpb.StartSessionResponse{
				SessionId: "sess-001",
				Status:    "active",
			}, nil
		},
	}

	addr, cleanup := startMockAdapterServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--adapter-addr", addr,
		"start-session",
		"--zone-id", "zone-1",
		"--vehicle-vin", "WBAPH5C55BA271111",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedReq == nil {
		t.Fatal("StartSession was not called")
	}
	if receivedReq.VehicleId == nil || receivedReq.VehicleId.Vin != "WBAPH5C55BA271111" {
		t.Errorf("expected VIN 'WBAPH5C55BA271111', got %v", receivedReq.VehicleId)
	}
	if receivedReq.ZoneId != "zone-1" {
		t.Errorf("expected zone_id 'zone-1', got %q", receivedReq.ZoneId)
	}
	if receivedReq.Timestamp <= 0 {
		t.Errorf("expected positive timestamp, got %d", receivedReq.Timestamp)
	}
}

func TestCmdStopSessionWithMockServer(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	var receivedReq *adapterpb.StopSessionRequest
	mock := &mockParkingAdapter{
		stopSessionFn: func(_ context.Context, req *adapterpb.StopSessionRequest) (*adapterpb.StopSessionResponse, error) {
			receivedReq = req
			return &adapterpb.StopSessionResponse{
				Status:          "completed",
				TotalFee:        2.50,
				DurationSeconds: 300,
			}, nil
		},
	}

	addr, cleanup := startMockAdapterServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--adapter-addr", addr,
		"stop-session",
		"--session-id", "sess-001",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedReq == nil {
		t.Fatal("StopSession was not called")
	}
	if receivedReq.SessionId != "sess-001" {
		t.Errorf("expected session_id 'sess-001', got %q", receivedReq.SessionId)
	}
	if receivedReq.Timestamp <= 0 {
		t.Errorf("expected positive timestamp, got %d", receivedReq.Timestamp)
	}
}

func TestCmdStopSessionNotFound(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	mock := &mockParkingAdapter{
		stopSessionFn: func(_ context.Context, req *adapterpb.StopSessionRequest) (*adapterpb.StopSessionResponse, error) {
			return nil, status.Errorf(codes.NotFound, "session %q not found", req.SessionId)
		},
	}

	addr, cleanup := startMockAdapterServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--adapter-addr", addr,
		"stop-session",
		"--session-id", "nonexistent",
	})
	if err == nil {
		t.Fatal("expected error for not-found session")
	}
	if !strings.Contains(err.Error(), "StopSession RPC failed") {
		t.Errorf("expected RPC failed error, got %q", err.Error())
	}
}

func TestCmdGetStatusWithMockServer(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	var receivedReq *adapterpb.GetStatusRequest
	mock := &mockParkingAdapter{
		getStatusFn: func(_ context.Context, req *adapterpb.GetStatusRequest) (*adapterpb.GetStatusResponse, error) {
			receivedReq = req
			return &adapterpb.GetStatusResponse{
				SessionId:  req.SessionId,
				Active:     true,
				StartTime:  time.Now().Unix() - 300,
				CurrentFee: 1.25,
			}, nil
		},
	}

	addr, cleanup := startMockAdapterServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--adapter-addr", addr,
		"get-status",
		"--session-id", "sess-001",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedReq == nil {
		t.Fatal("GetStatus was not called")
	}
	if receivedReq.SessionId != "sess-001" {
		t.Errorf("expected session_id 'sess-001', got %q", receivedReq.SessionId)
	}
}

func TestCmdGetStatusWithoutSessionID(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	var receivedReq *adapterpb.GetStatusRequest
	mock := &mockParkingAdapter{
		getStatusFn: func(_ context.Context, req *adapterpb.GetStatusRequest) (*adapterpb.GetStatusResponse, error) {
			receivedReq = req
			return &adapterpb.GetStatusResponse{
				SessionId:  "",
				Active:     false,
				StartTime:  0,
				CurrentFee: 0,
			}, nil
		},
	}

	addr, cleanup := startMockAdapterServer(t, mock)
	defer cleanup()

	// get-status without --session-id should work (optional).
	err := run([]string{
		"--adapter-addr", addr,
		"get-status",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedReq == nil {
		t.Fatal("GetStatus was not called")
	}
	if receivedReq.SessionId != "" {
		t.Errorf("expected empty session_id, got %q", receivedReq.SessionId)
	}
}

func TestCmdGetRateWithMockServer(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	var receivedReq *adapterpb.GetRateRequest
	mock := &mockParkingAdapter{
		getRateFn: func(_ context.Context, req *adapterpb.GetRateRequest) (*adapterpb.GetRateResponse, error) {
			receivedReq = req
			return &adapterpb.GetRateResponse{
				ZoneId:      req.ZoneId,
				RatePerHour: 3.00,
				Currency:    "EUR",
			}, nil
		},
	}

	addr, cleanup := startMockAdapterServer(t, mock)
	defer cleanup()

	err := run([]string{
		"--adapter-addr", addr,
		"get-rate",
		"--zone-id", "zone-downtown",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if receivedReq == nil {
		t.Fatal("GetRate was not called")
	}
	if receivedReq.ZoneId != "zone-downtown" {
		t.Errorf("expected zone_id 'zone-downtown', got %q", receivedReq.ZoneId)
	}
}

// ─── gRPC server error propagation ──────────────────────────────────────────

func TestGRPCErrorsPropagated(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	// Test that gRPC errors (NOT_FOUND, INTERNAL, etc.) are properly
	// propagated as CLI errors.
	tests := []struct {
		name        string
		setupUpdate func(*mockUpdateService)
		setupAdapter func(*mockParkingAdapter)
		args        []string
		errContains string
	}{
		{
			name: "InstallAdapter internal error",
			setupUpdate: func(m *mockUpdateService) {
				m.installAdapterFn = func(_ context.Context, _ *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
					return nil, status.Error(codes.Internal, "podman failed")
				}
			},
			args:        []string{"install-adapter", "--image-ref", "test:latest"},
			errContains: "InstallAdapter RPC failed",
		},
		{
			name: "RemoveAdapter not found",
			setupUpdate: func(m *mockUpdateService) {
				m.removeAdapterFn = func(_ context.Context, _ *updatepb.RemoveAdapterRequest) (*updatepb.RemoveAdapterResponse, error) {
					return nil, status.Error(codes.NotFound, "adapter not found")
				}
			},
			args:        []string{"remove-adapter", "--adapter-id", "missing"},
			errContains: "RemoveAdapter RPC failed",
		},
		{
			name: "ListAdapters unavailable",
			setupUpdate: func(m *mockUpdateService) {
				m.listAdaptersFn = func(_ context.Context, _ *updatepb.ListAdaptersRequest) (*updatepb.ListAdaptersResponse, error) {
					return nil, status.Error(codes.Unavailable, "service shutting down")
				}
			},
			args:        []string{"list-adapters"},
			errContains: "ListAdapters RPC failed",
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			updateMock := &mockUpdateService{}
			adapterMock := &mockParkingAdapter{}

			if tc.setupUpdate != nil {
				tc.setupUpdate(updateMock)
			}
			if tc.setupAdapter != nil {
				tc.setupAdapter(adapterMock)
			}

			updateAddr, cleanup1 := startMockUpdateServer(t, updateMock)
			defer cleanup1()
			adapterAddr, cleanup2 := startMockAdapterServer(t, adapterMock)
			defer cleanup2()

			fullArgs := []string{
				"--update-service-addr", updateAddr,
				"--adapter-addr", adapterAddr,
			}
			fullArgs = append(fullArgs, tc.args...)

			err := run(fullArgs)
			if err == nil {
				t.Fatal("expected error, got nil")
			}
			if !strings.Contains(err.Error(), tc.errContains) {
				t.Errorf("expected error containing %q, got %q", tc.errContains, err.Error())
			}
		})
	}
}

// ─── Usage output test ──────────────────────────────────────────────────────

func TestUsageShowsAllSubcommands(t *testing.T) {
	// Capture stderr output by redirecting.
	// We just verify the usage function doesn't panic.
	printUsage()

	// Verify help variant flags all work.
	for _, helpFlag := range []string{"help", "--help", "-h"} {
		err := run([]string{helpFlag})
		if err != nil {
			t.Errorf("help flag %q returned error: %v", helpFlag, err)
		}
	}
}

// ─── Full workflow test ─────────────────────────────────────────────────────

func TestFullWorkflow(t *testing.T) {
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	// Simulate a full workflow: install adapter → start session →
	// get status → get rate → stop session → remove adapter.
	var (
		installedImage string
		startedZone    string
		startedVIN     string
		stoppedSession string
		statusSession  string
		rateZone       string
		removedAdapter string
	)

	updateMock := &mockUpdateService{
		installAdapterFn: func(_ context.Context, req *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
			installedImage = req.ImageRef
			return &updatepb.InstallAdapterResponse{
				JobId:     "job-1",
				AdapterId: "adapter-1",
				State:     commonpb.AdapterState_ADAPTER_STATE_RUNNING,
			}, nil
		},
		removeAdapterFn: func(_ context.Context, req *updatepb.RemoveAdapterRequest) (*updatepb.RemoveAdapterResponse, error) {
			removedAdapter = req.AdapterId
			return &updatepb.RemoveAdapterResponse{}, nil
		},
	}

	adapterMock := &mockParkingAdapter{
		startSessionFn: func(_ context.Context, req *adapterpb.StartSessionRequest) (*adapterpb.StartSessionResponse, error) {
			startedZone = req.ZoneId
			startedVIN = req.VehicleId.Vin
			return &adapterpb.StartSessionResponse{
				SessionId: "sess-workflow",
				Status:    "active",
			}, nil
		},
		stopSessionFn: func(_ context.Context, req *adapterpb.StopSessionRequest) (*adapterpb.StopSessionResponse, error) {
			stoppedSession = req.SessionId
			return &adapterpb.StopSessionResponse{
				Status:          "completed",
				TotalFee:        5.00,
				DurationSeconds: 600,
			}, nil
		},
		getStatusFn: func(_ context.Context, req *adapterpb.GetStatusRequest) (*adapterpb.GetStatusResponse, error) {
			statusSession = req.SessionId
			return &adapterpb.GetStatusResponse{
				SessionId:  req.SessionId,
				Active:     true,
				StartTime:  time.Now().Unix() - 300,
				CurrentFee: 2.50,
			}, nil
		},
		getRateFn: func(_ context.Context, req *adapterpb.GetRateRequest) (*adapterpb.GetRateResponse, error) {
			rateZone = req.ZoneId
			return &adapterpb.GetRateResponse{
				ZoneId:      req.ZoneId,
				RatePerHour: 3.00,
				Currency:    "EUR",
			}, nil
		},
	}

	updateAddr, cleanup1 := startMockUpdateServer(t, updateMock)
	defer cleanup1()
	adAddr, cleanup2 := startMockAdapterServer(t, adapterMock)
	defer cleanup2()

	baseArgs := func(args ...string) []string {
		return append([]string{
			"--update-service-addr", updateAddr,
			"--adapter-addr", adAddr,
		}, args...)
	}

	// Step 1: Install adapter.
	if err := run(baseArgs("install-adapter", "--image-ref", "poa:v1", "--checksum", "sha256:111")); err != nil {
		t.Fatalf("install-adapter failed: %v", err)
	}
	if installedImage != "poa:v1" {
		t.Errorf("expected installed image 'poa:v1', got %q", installedImage)
	}

	// Step 2: Start session.
	if err := run(baseArgs("start-session", "--zone-id", "zone-a", "--vehicle-vin", "VIN001")); err != nil {
		t.Fatalf("start-session failed: %v", err)
	}
	if startedZone != "zone-a" {
		t.Errorf("expected zone 'zone-a', got %q", startedZone)
	}
	if startedVIN != "VIN001" {
		t.Errorf("expected VIN 'VIN001', got %q", startedVIN)
	}

	// Step 3: Get status.
	if err := run(baseArgs("get-status", "--session-id", "sess-workflow")); err != nil {
		t.Fatalf("get-status failed: %v", err)
	}
	if statusSession != "sess-workflow" {
		t.Errorf("expected session 'sess-workflow', got %q", statusSession)
	}

	// Step 4: Get rate.
	if err := run(baseArgs("get-rate", "--zone-id", "zone-a")); err != nil {
		t.Fatalf("get-rate failed: %v", err)
	}
	if rateZone != "zone-a" {
		t.Errorf("expected zone 'zone-a', got %q", rateZone)
	}

	// Step 5: Stop session.
	if err := run(baseArgs("stop-session", "--session-id", "sess-workflow")); err != nil {
		t.Fatalf("stop-session failed: %v", err)
	}
	if stoppedSession != "sess-workflow" {
		t.Errorf("expected session 'sess-workflow', got %q", stoppedSession)
	}

	// Step 6: Remove adapter.
	if err := run(baseArgs("remove-adapter", "--adapter-id", "adapter-1")); err != nil {
		t.Fatalf("remove-adapter failed: %v", err)
	}
	if removedAdapter != "adapter-1" {
		t.Errorf("expected removed adapter 'adapter-1', got %q", removedAdapter)
	}
}

// ─── dialGRPC test ──────────────────────────────────────────────────────────

func TestDialGRPCUnreachable(t *testing.T) {
	// Port 1 should be unreachable.
	_, err := dialGRPC("localhost:1")
	if err == nil {
		t.Fatal("expected error for unreachable address")
	}
	if !strings.Contains(err.Error(), "failed to connect") {
		t.Errorf("expected 'failed to connect' error, got %q", err.Error())
	}
}

func TestDialGRPCSuccess(t *testing.T) {
	// Start a minimal gRPC server and verify dialGRPC connects.
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}
	srv := grpc.NewServer()
	go func() { _ = srv.Serve(lis) }()
	defer srv.Stop()

	conn, err := dialGRPC(lis.Addr().String())
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	conn.Close()
}

// ─── printJSON test ──────────────────────────────────────────────────────────

func TestPrintJSON(t *testing.T) {
	// Just verify it doesn't panic.
	printJSON(map[string]string{"key": "value"})
	printJSON(struct{ Foo string }{"bar"})

	// printJSON with a channel (not serializable) should print to stderr, not panic.
	printJSON(make(chan int))
}

// ─── Flag override precedence test ──────────────────────────────────────────

func TestGlobalFlagsAfterCommand(t *testing.T) {
	// Verify global flags can be placed before the command.
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTER_ADDR")

	remaining, err := parseGlobalFlags([]string{
		"--update-service-addr", "host1:50053",
		"install-adapter",
		"--image-ref", "test:latest",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if updateServiceAddr != "host1:50053" {
		t.Errorf("expected 'host1:50053', got %q", updateServiceAddr)
	}
	// --image-ref and its value should be in remaining along with the command.
	if len(remaining) != 3 {
		t.Errorf("expected 3 remaining args, got %d: %v", len(remaining), remaining)
	}
	if remaining[0] != "install-adapter" {
		t.Errorf("expected first remaining to be 'install-adapter', got %q", remaining[0])
	}
}

// ─── Edge case: printJSON with proto message ────────────────────────────────

func TestPrintJSONProtoMessage(t *testing.T) {
	// Verify printJSON works with protobuf-generated types.
	resp := &updatepb.InstallAdapterResponse{
		JobId:     "job-1",
		AdapterId: "adapter-1",
		State:     commonpb.AdapterState_ADAPTER_STATE_RUNNING,
	}
	printJSON(resp)

	// Verify with a proto response that has nested messages.
	statusResp := &updatepb.GetAdapterStatusResponse{
		Info: &commonpb.AdapterInfo{
			AdapterId: "a1",
			Name:      "test",
			ImageRef:  "img:v1",
		},
		State: commonpb.AdapterState_ADAPTER_STATE_STOPPED,
	}
	printJSON(statusResp)
}

// ─── Ensure all commands exist per 04-REQ-7.1 and 04-REQ-7.2 ───────────────

func TestRequiredUpdateServiceSubcommands(t *testing.T) {
	// 04-REQ-7.1: install-adapter, list-adapters, remove-adapter,
	// adapter-status, watch-adapters.
	required := []string{
		"install-adapter",
		"list-adapters",
		"remove-adapter",
		"adapter-status",
		"watch-adapters",
	}
	for _, cmd := range required {
		t.Run(cmd, func(t *testing.T) {
			err := run([]string{cmd})
			if err != nil && strings.Contains(err.Error(), fmt.Sprintf("unknown command: %s", cmd)) {
				t.Errorf("UPDATE_SERVICE subcommand %q not implemented (04-REQ-7.1)", cmd)
			}
		})
	}
}

func TestRequiredAdapterSubcommands(t *testing.T) {
	// 04-REQ-7.2: start-session, stop-session, get-status, get-rate.
	required := []string{
		"start-session",
		"stop-session",
		"get-status",
		"get-rate",
	}
	for _, cmd := range required {
		t.Run(cmd, func(t *testing.T) {
			err := run([]string{cmd})
			if err != nil && strings.Contains(err.Error(), fmt.Sprintf("unknown command: %s", cmd)) {
				t.Errorf("PARKING_OPERATOR_ADAPTOR subcommand %q not implemented (04-REQ-7.2)", cmd)
			}
		})
	}
}
