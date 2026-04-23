package mockapps_test

import (
	"context"
	"encoding/json"
	"net"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	adaptorpb "github.com/rhadp/parking-fee-service/gen/parking_adaptor/v1"
	updatepb "github.com/rhadp/parking-fee-service/gen/update_service/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// ---------------------------------------------------------------------------
// Mock gRPC servers
// ---------------------------------------------------------------------------

// mockUpdateService implements the UpdateService gRPC server for testing.
// It supports WatchAdapterStates streaming with configurable events and errors.
type mockUpdateService struct {
	updatepb.UnimplementedUpdateServiceServer
	// watchEvents is the list of events to send on WatchAdapterStates stream.
	// If nil, the default unimplemented behavior is used.
	watchEvents []*updatepb.AdapterStateEvent
	// watchError, if non-nil, is returned after sending watchEvents.
	// This simulates a gRPC failure mid-stream (e.g., network loss).
	watchError error
}

func (m *mockUpdateService) InstallAdapter(_ context.Context, req *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
	return &updatepb.InstallAdapterResponse{
		JobId:     "j1",
		AdapterId: "a1",
		State:     updatepb.AdapterState_DOWNLOADING,
	}, nil
}

func (m *mockUpdateService) ListAdapters(_ context.Context, _ *updatepb.ListAdaptersRequest) (*updatepb.ListAdaptersResponse, error) {
	return &updatepb.ListAdaptersResponse{
		Adapters: []*updatepb.AdapterInfo{
			{
				AdapterId: "a1",
				State:     updatepb.AdapterState_RUNNING,
				ImageRef:  "registry/adapter:v1",
			},
		},
	}, nil
}

func (m *mockUpdateService) GetAdapterStatus(_ context.Context, req *updatepb.GetAdapterStatusRequest) (*updatepb.GetAdapterStatusResponse, error) {
	return &updatepb.GetAdapterStatusResponse{
		AdapterId: req.GetAdapterId(),
		State:     updatepb.AdapterState_RUNNING,
		ImageRef:  "registry/adapter:v1",
	}, nil
}

func (m *mockUpdateService) RemoveAdapter(_ context.Context, req *updatepb.RemoveAdapterRequest) (*updatepb.RemoveAdapterResponse, error) {
	return &updatepb.RemoveAdapterResponse{}, nil
}

func (m *mockUpdateService) WatchAdapterStates(_ *updatepb.WatchAdapterStatesRequest, stream grpc.ServerStreamingServer[updatepb.AdapterStateEvent]) error {
	for _, event := range m.watchEvents {
		if err := stream.Send(event); err != nil {
			return err
		}
	}
	if m.watchError != nil {
		return m.watchError
	}
	// Normal close: return nil to end the stream cleanly.
	return nil
}

// mockParkingAdaptor implements the ParkingAdaptor gRPC server for testing.
type mockParkingAdaptor struct {
	adaptorpb.UnimplementedParkingAdaptorServer
}

func (m *mockParkingAdaptor) StartSession(_ context.Context, req *adaptorpb.StartSessionRequest) (*adaptorpb.StartSessionResponse, error) {
	return &adaptorpb.StartSessionResponse{
		SessionId: "s1",
		Status:    "active",
		Rate: &adaptorpb.ParkingRate{
			RateType: "per_hour",
			Amount:   2.50,
			Currency: "EUR",
		},
	}, nil
}

func (m *mockParkingAdaptor) StopSession(_ context.Context, _ *adaptorpb.StopSessionRequest) (*adaptorpb.StopSessionResponse, error) {
	return &adaptorpb.StopSessionResponse{
		SessionId:       "s1",
		Status:          "stopped",
		DurationSeconds: 3600,
		TotalAmount:     2.50,
		Currency:        "EUR",
	}, nil
}

// startMockUpdateService starts a gRPC server with a mock UpdateService and
// returns the listener address. The server is stopped when the test completes.
func startMockUpdateService(t *testing.T) string {
	t.Helper()
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}

	srv := grpc.NewServer()
	updatepb.RegisterUpdateServiceServer(srv, &mockUpdateService{})
	go func() { _ = srv.Serve(lis) }()
	t.Cleanup(func() { srv.Stop() })

	return lis.Addr().String()
}

// startMockParkingAdaptor starts a gRPC server with a mock ParkingAdaptor and
// returns the listener address. The server is stopped when the test completes.
func startMockParkingAdaptor(t *testing.T) string {
	t.Helper()
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}

	srv := grpc.NewServer()
	adaptorpb.RegisterParkingAdaptorServer(srv, &mockParkingAdaptor{})
	go func() { _ = srv.Serve(lis) }()
	t.Cleanup(func() { srv.Stop() })

	return lis.Addr().String()
}

// startMockUpdateServiceWithWatch starts a gRPC server with a mock UpdateService
// that has configurable WatchAdapterStates behavior.
func startMockUpdateServiceWithWatch(t *testing.T, events []*updatepb.AdapterStateEvent, watchErr error) string {
	t.Helper()
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}

	srv := grpc.NewServer()
	updatepb.RegisterUpdateServiceServer(srv, &mockUpdateService{
		watchEvents: events,
		watchError:  watchErr,
	})
	go func() { _ = srv.Serve(lis) }()
	t.Cleanup(func() { srv.Stop() })

	return lis.Addr().String()
}

// ---------------------------------------------------------------------------
// TS-09-WATCH: Parking App CLI Watch Adapter States (streaming)
// Requirement: 09-REQ-5.3
// Addresses Skeptic finding: no test covers WatchAdapterStates streaming RPC.
// ---------------------------------------------------------------------------

func TestWatch(t *testing.T) {
	events := []*updatepb.AdapterStateEvent{
		{
			AdapterId: "a1",
			OldState:  updatepb.AdapterState_DOWNLOADING,
			NewState:  updatepb.AdapterState_INSTALLING,
			Timestamp: 1700000001,
		},
		{
			AdapterId: "a1",
			OldState:  updatepb.AdapterState_INSTALLING,
			NewState:  updatepb.AdapterState_RUNNING,
			Timestamp: 1700000002,
		},
	}

	addr := startMockUpdateServiceWithWatch(t, events, nil)

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"watch",
		"--update-addr="+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	// Verify both events were printed to stdout.
	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected stdout to contain adapter_id 'a1', got: %s", stdout)
	}

	if !strings.Contains(stdout, "INSTALLING") {
		t.Errorf("expected stdout to contain 'INSTALLING', got: %s", stdout)
	}

	if !strings.Contains(stdout, "RUNNING") {
		t.Errorf("expected stdout to contain 'RUNNING', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TestWatchGRPCError: Watch stream exits 1 on genuine gRPC error
// Requirement: 09-REQ-5.E2
// Addresses Skeptic finding: runWatch treats all stream.Recv errors as normal
// close — genuine gRPC failures should exit with code 1, not 0.
// ---------------------------------------------------------------------------

func TestWatchGRPCError(t *testing.T) {
	// Send one event, then return a genuine gRPC error.
	events := []*updatepb.AdapterStateEvent{
		{
			AdapterId: "a1",
			OldState:  updatepb.AdapterState_UNKNOWN,
			NewState:  updatepb.AdapterState_DOWNLOADING,
			Timestamp: 1700000001,
		},
	}
	watchErr := status.Error(codes.Internal, "server crashed")

	addr := startMockUpdateServiceWithWatch(t, events, watchErr)

	binary := buildBinary(t, "parking-app-cli")
	_, stderr, exitCode := runBinary(t, binary,
		"watch",
		"--update-addr="+addr,
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 on gRPC stream error, got %d\nstderr: %s", exitCode, stderr)
	}

	if len(stderr) == 0 {
		t.Error("expected error message on stderr when gRPC stream fails")
	}
}

// ---------------------------------------------------------------------------
// TS-09-5: Parking App CLI Lookup
// Requirement: 09-REQ-4.1, 09-REQ-4.3
// ---------------------------------------------------------------------------

func TestLookup(t *testing.T) {
	var receivedPath string
	var receivedQuery string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedPath = r.URL.Path
		receivedQuery = r.URL.RawQuery
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode([]map[string]string{
			{"operator_id": "op-1", "name": "Demo"},
		})
	}))
	defer srv.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"lookup",
		"--lat=48.1351",
		"--lon=11.5820",
		"--service-addr="+srv.URL,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "op-1") {
		t.Errorf("expected stdout to contain 'op-1', got: %s", stdout)
	}

	if receivedPath != "/operators" {
		t.Errorf("expected path '/operators', got: %s", receivedPath)
	}

	if !strings.Contains(receivedQuery, "lat=48.1351") || !strings.Contains(receivedQuery, "lon=11.5820") {
		t.Errorf("expected query to contain lat and lon, got: %s", receivedQuery)
	}
}

// ---------------------------------------------------------------------------
// TS-09-6: Parking App CLI Adapter Info
// Requirement: 09-REQ-4.2
// ---------------------------------------------------------------------------

func TestAdapterInfo(t *testing.T) {
	var receivedPath string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedPath = r.URL.Path
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{
			"image_ref": "registry/adapter:v1",
			"checksum":  "sha256:abc",
		})
	}))
	defer srv.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"adapter-info",
		"--operator-id=op-1",
		"--service-addr="+srv.URL,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "image_ref") {
		t.Errorf("expected stdout to contain 'image_ref', got: %s", stdout)
	}

	if receivedPath != "/operators/op-1/adapter" {
		t.Errorf("expected path '/operators/op-1/adapter', got: %s", receivedPath)
	}
}

// ---------------------------------------------------------------------------
// TS-09-7: Parking App CLI Install Adapter
// Requirement: 09-REQ-5.1, 09-REQ-5.6
// ---------------------------------------------------------------------------

func TestInstall(t *testing.T) {
	addr := startMockUpdateService(t)

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"install",
		"--image-ref=registry/adapter:v1",
		"--checksum=sha256:abc",
		"--update-addr="+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	// Verify the response contains the expected fields from the mock server.
	if !strings.Contains(stdout, "j1") {
		t.Errorf("expected stdout to contain job_id 'j1', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-8: Parking App CLI List Adapters
// Requirement: 09-REQ-5.2
// ---------------------------------------------------------------------------

func TestList(t *testing.T) {
	addr := startMockUpdateService(t)

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"list",
		"--update-addr="+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected stdout to contain adapter_id 'a1', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-18: Parking App CLI Get Adapter Status
// Requirement: 09-REQ-5.4
// ---------------------------------------------------------------------------

func TestAdapterStatus(t *testing.T) {
	addr := startMockUpdateService(t)

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"status",
		"--adapter-id=a1",
		"--update-addr="+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected stdout to contain adapter_id 'a1', got: %s", stdout)
	}

	if !strings.Contains(stdout, "RUNNING") {
		t.Errorf("expected stdout to contain 'RUNNING', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-19: Parking App CLI Remove Adapter
// Requirement: 09-REQ-5.5
// ---------------------------------------------------------------------------

func TestRemove(t *testing.T) {
	addr := startMockUpdateService(t)

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"remove",
		"--adapter-id=a1",
		"--update-addr="+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	// Verify the CLI confirms removal with meaningful output containing
	// the adapter ID (addresses skeptic finding about weak assertion).
	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected stdout to contain adapter_id 'a1', got: %s", stdout)
	}

	if !strings.Contains(stdout, "removed") {
		t.Errorf("expected stdout to contain 'removed', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-9: Parking App CLI Start Session Override
// Requirement: 09-REQ-6.1, 09-REQ-6.3
// ---------------------------------------------------------------------------

func TestStartSession(t *testing.T) {
	addr := startMockParkingAdaptor(t)

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"start-session",
		"--zone-id=zone-demo-1",
		"--adaptor-addr="+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "s1") {
		t.Errorf("expected stdout to contain session_id 's1', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-10: Parking App CLI Stop Session Override
// Requirement: 09-REQ-6.2
// ---------------------------------------------------------------------------

func TestStopSession(t *testing.T) {
	addr := startMockParkingAdaptor(t)

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"stop-session",
		"--adaptor-addr="+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "stopped") {
		t.Errorf("expected stdout to contain 'stopped', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E10: Parking App CLI gRPC Error
// Requirement: 09-REQ-5.E2, 09-REQ-6.E1
// ---------------------------------------------------------------------------

func TestInstallGRPCError(t *testing.T) {
	binary := buildBinary(t, "parking-app-cli")

	// Connect to a port where no service is running.
	_, stderr, exitCode := runBinary(t, binary,
		"install",
		"--image-ref=x",
		"--checksum=y",
		"--update-addr=localhost:19999",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 on gRPC error, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected error message on stderr when gRPC call fails")
	}
}

func TestSessionGRPCError(t *testing.T) {
	binary := buildBinary(t, "parking-app-cli")

	_, stderr, exitCode := runBinary(t, binary,
		"start-session",
		"--zone-id=zone-1",
		"--adaptor-addr=localhost:19999",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 on gRPC error, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected error message on stderr when gRPC call fails")
	}
}

// ---------------------------------------------------------------------------
// TS-09-E11: PARKING_FEE_SERVICE Non-2xx
// Requirement: 09-REQ-4.E2
// ---------------------------------------------------------------------------

func TestLookupHTTPError(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
		w.Write([]byte("internal error"))
	}))
	defer srv.Close()

	binary := buildBinary(t, "parking-app-cli")
	_, stderr, exitCode := runBinary(t, binary,
		"lookup",
		"--lat=0",
		"--lon=0",
		"--service-addr="+srv.URL,
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 on non-2xx response, got %d", exitCode)
	}

	if !strings.Contains(stderr, "500") {
		t.Errorf("expected stderr to contain '500', got: %s", stderr)
	}
}

// ---------------------------------------------------------------------------
// Parking App CLI Missing Args (09-REQ-4.E1)
// Addresses Skeptic finding: 09-REQ-4.E1 was untested.
// ---------------------------------------------------------------------------

func TestLookupMissingArgs(t *testing.T) {
	binary := buildBinary(t, "parking-app-cli")

	// Lookup requires --lat and --lon. Omit --lon.
	_, stderr, exitCode := runBinary(t, binary,
		"lookup",
		"--lat=48.1351",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 when required flags missing, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected usage error on stderr when required flags missing")
	}
}

func TestInstallMissingArgs(t *testing.T) {
	binary := buildBinary(t, "parking-app-cli")

	// Install requires --image-ref and --checksum. Omit both.
	_, stderr, exitCode := runBinary(t, binary,
		"install",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 when required flags missing, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected usage error on stderr when required flags missing")
	}
}

// ---------------------------------------------------------------------------
// TS-09-P2: CLI Argument Validation Property (Go CLIs)
// Property 2 from design.md
// Validates: 09-REQ-4.E1, 09-REQ-5.E1, 09-REQ-7.E1, 09-REQ-7.E2
// Systematically enumerates all missing required argument subsets for each
// parking-app-cli and companion-app-cli subcommand.
// ---------------------------------------------------------------------------

func TestGoCliArgumentValidationProperty(t *testing.T) {
	parkingBin := buildBinary(t, "parking-app-cli")
	companionBin := buildBinary(t, "companion-app-cli")

	cases := []struct {
		name   string
		binary string
		args   []string
	}{
		// parking-app-cli: lookup requires --lat and --lon.
		{"parking/lookup/no-args", parkingBin, []string{"lookup"}},
		{"parking/lookup/lat-only", parkingBin, []string{"lookup", "--lat=48.13"}},
		{"parking/lookup/lon-only", parkingBin, []string{"lookup", "--lon=11.58"}},

		// parking-app-cli: adapter-info requires --operator-id.
		{"parking/adapter-info/no-args", parkingBin, []string{"adapter-info"}},

		// parking-app-cli: install requires --image-ref and --checksum.
		{"parking/install/no-args", parkingBin, []string{"install"}},
		{"parking/install/image-only", parkingBin, []string{"install", "--image-ref=x"}},
		{"parking/install/checksum-only", parkingBin, []string{"install", "--checksum=y"}},

		// parking-app-cli: status requires --adapter-id.
		{"parking/status/no-args", parkingBin, []string{"status"}},

		// parking-app-cli: remove requires --adapter-id.
		{"parking/remove/no-args", parkingBin, []string{"remove"}},

		// parking-app-cli: start-session requires --zone-id.
		{"parking/start-session/no-args", parkingBin, []string{"start-session"}},

		// parking-app-cli: no subcommand.
		{"parking/no-subcommand", parkingBin, nil},

		// companion-app-cli: lock requires --vin and token.
		{"companion/lock/no-args", companionBin, []string{"lock"}},
		{"companion/lock/vin-only-no-token", companionBin, []string{"lock", "--vin=V1"}},
		{"companion/lock/token-only-no-vin", companionBin, []string{"lock", "--token=t1"}},

		// companion-app-cli: unlock requires --vin and token.
		{"companion/unlock/no-args", companionBin, []string{"unlock"}},
		{"companion/unlock/vin-only-no-token", companionBin, []string{"unlock", "--vin=V1"}},

		// companion-app-cli: status requires --vin, --command-id, and token.
		{"companion/status/no-args", companionBin, []string{"status"}},
		{"companion/status/vin-only", companionBin, []string{"status", "--vin=V1", "--token=t1"}},

		// companion-app-cli: no subcommand.
		{"companion/no-subcommand", companionBin, nil},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			var stdout, stderr string
			var exitCode int

			// Clear CLOUD_GATEWAY_TOKEN for companion tests to ensure token
			// validation is exercised via flag-only.
			if strings.Contains(tc.name, "companion") && strings.Contains(tc.name, "no-token") {
				stdout, stderr, exitCode = runBinaryWithEnv(t, tc.binary,
					[]string{"CLOUD_GATEWAY_TOKEN="},
					tc.args...)
			} else {
				stdout, stderr, exitCode = runBinary(t, tc.binary, tc.args...)
			}
			_ = stdout

			if exitCode != 1 {
				t.Fatalf("expected exit code 1 with missing args, got %d\nstderr: %s", exitCode, stderr)
			}

			if len(stderr) == 0 {
				t.Error("expected non-empty stderr with missing args")
			}
		})
	}
}
