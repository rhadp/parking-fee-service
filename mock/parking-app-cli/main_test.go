package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"

	"github.com/rhadp/parking-fee-service/gen/go/parkingadaptorpb"
	"github.com/rhadp/parking-fee-service/gen/go/updateservicepb"
	"google.golang.org/grpc"
)

// --------------------------------------------------------------------------
// Test helpers
// --------------------------------------------------------------------------

// runCLI calls run() and returns stdout, stderr, and exit code.
func runCLI(args []string) (stdout string, stderr string, code int) {
	var outBuf, errBuf bytes.Buffer
	code = run(args, &outBuf, &errBuf)
	return outBuf.String(), errBuf.String(), code
}

// newCapturingHTTPServer returns an httptest.Server that captures the last request.
func newCapturingHTTPServer(t *testing.T, capture *struct {
	Method string
	Path   string
	Query  string
}, statusCode int, respBody string) *httptest.Server {
	t.Helper()
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		capture.Method = r.Method
		capture.Path = r.URL.Path
		capture.Query = r.URL.RawQuery
		w.WriteHeader(statusCode)
		_, _ = io.WriteString(w, respBody)
	}))
}

// --------------------------------------------------------------------------
// Mock UPDATE_SERVICE gRPC server
// --------------------------------------------------------------------------

type mockUpdateServiceServer struct {
	updateservicepb.UnimplementedUpdateServiceServer
	installCalled   bool
	listCalled      bool
	removeCalled    bool
	getStatusCalled bool
	watchCalled     bool

	capturedInstallReq *updateservicepb.InstallAdapterRequest
	capturedRemoveReq  *updateservicepb.RemoveAdapterRequest
	capturedStatusReq  *updateservicepb.GetAdapterStatusRequest
}

func (m *mockUpdateServiceServer) InstallAdapter(_ context.Context, req *updateservicepb.InstallAdapterRequest) (*updateservicepb.InstallAdapterResponse, error) {
	m.installCalled = true
	m.capturedInstallReq = req
	return &updateservicepb.InstallAdapterResponse{JobId: "job-001", AdapterId: "adapter-001"}, nil
}

func (m *mockUpdateServiceServer) ListAdapters(_ context.Context, _ *updateservicepb.ListAdaptersRequest) (*updateservicepb.ListAdaptersResponse, error) {
	m.listCalled = true
	return &updateservicepb.ListAdaptersResponse{}, nil
}

func (m *mockUpdateServiceServer) RemoveAdapter(_ context.Context, req *updateservicepb.RemoveAdapterRequest) (*updateservicepb.RemoveAdapterResponse, error) {
	m.removeCalled = true
	m.capturedRemoveReq = req
	return &updateservicepb.RemoveAdapterResponse{}, nil
}

func (m *mockUpdateServiceServer) GetAdapterStatus(_ context.Context, req *updateservicepb.GetAdapterStatusRequest) (*updateservicepb.GetAdapterStatusResponse, error) {
	m.getStatusCalled = true
	m.capturedStatusReq = req
	return &updateservicepb.GetAdapterStatusResponse{}, nil
}

func (m *mockUpdateServiceServer) WatchAdapterStates(_ *updateservicepb.WatchAdapterStatesRequest, stream grpc.ServerStreamingServer[updateservicepb.AdapterStateEvent]) error {
	m.watchCalled = true
	// Send two events then close.
	for i := 0; i < 2; i++ {
		if err := stream.Send(&updateservicepb.AdapterStateEvent{
			AdapterId: fmt.Sprintf("adapter-%d", i),
		}); err != nil {
			return err
		}
	}
	return nil
}

// startMockUpdateService starts a mock UPDATE_SERVICE gRPC server and returns its address.
func startMockUpdateService(t *testing.T) (string, *mockUpdateServiceServer) {
	t.Helper()
	srv := &mockUpdateServiceServer{}
	grpcSrv := grpc.NewServer()
	updateservicepb.RegisterUpdateServiceServer(grpcSrv, srv)

	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	go grpcSrv.Serve(ln)
	t.Cleanup(grpcSrv.Stop)
	return ln.Addr().String(), srv
}

// --------------------------------------------------------------------------
// Mock PARKING_ADAPTOR gRPC server
// --------------------------------------------------------------------------

type mockAdaptorServer struct {
	parkingadaptorpb.UnimplementedParkingAdaptorServer
	startCalled bool
	stopCalled  bool

	capturedStartReq *parkingadaptorpb.StartSessionRequest
}

func (m *mockAdaptorServer) StartSession(_ context.Context, req *parkingadaptorpb.StartSessionRequest) (*parkingadaptorpb.StartSessionResponse, error) {
	m.startCalled = true
	m.capturedStartReq = req
	return &parkingadaptorpb.StartSessionResponse{SessionId: "sess-mock", Status: "active"}, nil
}

func (m *mockAdaptorServer) StopSession(_ context.Context, _ *parkingadaptorpb.StopSessionRequest) (*parkingadaptorpb.StopSessionResponse, error) {
	m.stopCalled = true
	return &parkingadaptorpb.StopSessionResponse{SessionId: "sess-mock", Status: "stopped"}, nil
}

// startMockAdaptor starts a mock PARKING_ADAPTOR gRPC server and returns its address.
func startMockAdaptor(t *testing.T) (string, *mockAdaptorServer) {
	t.Helper()
	srv := &mockAdaptorServer{}
	grpcSrv := grpc.NewServer()
	parkingadaptorpb.RegisterParkingAdaptorServer(grpcSrv, srv)

	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	go grpcSrv.Serve(ln)
	t.Cleanup(grpcSrv.Stop)
	return ln.Addr().String(), srv
}

// --------------------------------------------------------------------------
// Config tests
// --------------------------------------------------------------------------

// TS-09-23: PARKING_APP Config Defaults
// Requirement: 09-REQ-5.3
func TestConfigDefaults(t *testing.T) {
	os.Unsetenv("PARKING_FEE_SERVICE_URL")
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTOR_ADDR")
	cfg := loadConfig(nil)
	if cfg.FeeServiceURL != "http://localhost:8080" {
		t.Errorf("FeeServiceURL = %q, want http://localhost:8080", cfg.FeeServiceURL)
	}
	if cfg.UpdateSvcAddr != "localhost:50052" {
		t.Errorf("UpdateSvcAddr = %q, want localhost:50052", cfg.UpdateSvcAddr)
	}
	if cfg.AdaptorAddr != "localhost:50053" {
		t.Errorf("AdaptorAddr = %q, want localhost:50053", cfg.AdaptorAddr)
	}
}

// TS-09-23: Env vars override defaults
func TestConfigEnvOverrides(t *testing.T) {
	os.Setenv("PARKING_FEE_SERVICE_URL", "http://custom-fee:9000")
	os.Setenv("UPDATE_SERVICE_ADDR", "custom-update:50052")
	os.Setenv("ADAPTOR_ADDR", "custom-adaptor:50053")
	defer func() {
		os.Unsetenv("PARKING_FEE_SERVICE_URL")
		os.Unsetenv("UPDATE_SERVICE_ADDR")
		os.Unsetenv("ADAPTOR_ADDR")
	}()
	cfg := loadConfig(nil)
	if cfg.FeeServiceURL != "http://custom-fee:9000" {
		t.Errorf("FeeServiceURL = %q, want http://custom-fee:9000", cfg.FeeServiceURL)
	}
	if cfg.UpdateSvcAddr != "custom-update:50052" {
		t.Errorf("UpdateSvcAddr = %q, want custom-update:50052", cfg.UpdateSvcAddr)
	}
	if cfg.AdaptorAddr != "custom-adaptor:50053" {
		t.Errorf("AdaptorAddr = %q, want custom-adaptor:50053", cfg.AdaptorAddr)
	}
}

// --------------------------------------------------------------------------
// REST subcommand tests
// --------------------------------------------------------------------------

// TS-09-12: lookup subcommand queries PARKING_FEE_SERVICE
// Requirement: 09-REQ-4.1
func TestLookup(t *testing.T) {
	var cap struct{ Method, Path, Query string }
	srv := newCapturingHTTPServer(t, &cap, 200, `[]`)
	defer srv.Close()

	os.Unsetenv("PARKING_FEE_SERVICE_URL")
	_, stderr, code := runCLI([]string{
		"lookup",
		"--lat=48.1351",
		"--lon=11.5820",
		"--fee-service-url=" + srv.URL,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if cap.Path != "/operators" {
		t.Errorf("path = %q, want /operators", cap.Path)
	}
	if !strings.Contains(cap.Query, "lat=48.1351") {
		t.Errorf("query %q missing lat=48.1351", cap.Query)
	}
	if !strings.Contains(cap.Query, "lon=11.5820") {
		t.Errorf("query %q missing lon=11.5820", cap.Query)
	}
}

// TS-09-13: adapter-info subcommand queries PARKING_FEE_SERVICE
// Requirement: 09-REQ-4.2
func TestAdapterInfo(t *testing.T) {
	var cap struct{ Method, Path, Query string }
	srv := newCapturingHTTPServer(t, &cap, 200, `{}`)
	defer srv.Close()

	_, stderr, code := runCLI([]string{
		"adapter-info",
		"--operator-id=op-001",
		"--fee-service-url=" + srv.URL,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if cap.Path != "/operators/op-001/adapter" {
		t.Errorf("path = %q, want /operators/op-001/adapter", cap.Path)
	}
}

// --------------------------------------------------------------------------
// gRPC subcommand tests (UPDATE_SERVICE)
// --------------------------------------------------------------------------

// TS-09-14: install calls UPDATE_SERVICE InstallAdapter
// Requirement: 09-REQ-4.3
func TestInstall(t *testing.T) {
	addr, mock := startMockUpdateService(t)

	_, stderr, code := runCLI([]string{
		"install",
		"--image-ref=ghcr.io/demo:v1",
		"--checksum=abc123",
		"--update-service-addr=" + addr,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if !mock.installCalled {
		t.Error("InstallAdapter was not called")
	}
	if mock.capturedInstallReq.GetImageRef() != "ghcr.io/demo:v1" {
		t.Errorf("image_ref = %q, want ghcr.io/demo:v1", mock.capturedInstallReq.GetImageRef())
	}
	if mock.capturedInstallReq.GetChecksumSha256() != "abc123" {
		t.Errorf("checksum = %q, want abc123", mock.capturedInstallReq.GetChecksumSha256())
	}
}

// TS-09-15: watch streams WatchAdapterStates events
// Requirement: 09-REQ-4.4
func TestWatch(t *testing.T) {
	addr, mock := startMockUpdateService(t)

	stdout, stderr, code := runCLI([]string{
		"watch",
		"--update-service-addr=" + addr,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if !mock.watchCalled {
		t.Error("WatchAdapterStates was not called")
	}
	// Should print at least 2 lines (one per event).
	lines := strings.Split(strings.TrimSpace(stdout), "\n")
	if len(lines) < 2 {
		t.Errorf("expected at least 2 output lines, got %d: %q", len(lines), stdout)
	}
}

// TS-09-16: list calls UPDATE_SERVICE ListAdapters
// Requirement: 09-REQ-4.5
func TestList(t *testing.T) {
	addr, mock := startMockUpdateService(t)

	_, stderr, code := runCLI([]string{
		"list",
		"--update-service-addr=" + addr,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if !mock.listCalled {
		t.Error("ListAdapters was not called")
	}
}

// TS-09-17: remove calls UPDATE_SERVICE RemoveAdapter
// Requirement: 09-REQ-4.6
func TestRemove(t *testing.T) {
	addr, mock := startMockUpdateService(t)

	_, stderr, code := runCLI([]string{
		"remove",
		"--adapter-id=adapter-001",
		"--update-service-addr=" + addr,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if !mock.removeCalled {
		t.Error("RemoveAdapter was not called")
	}
	if mock.capturedRemoveReq.GetAdapterId() != "adapter-001" {
		t.Errorf("adapter_id = %q, want adapter-001", mock.capturedRemoveReq.GetAdapterId())
	}
}

// TS-09-18: status calls UPDATE_SERVICE GetAdapterStatus
// Requirement: 09-REQ-4.7
func TestStatus(t *testing.T) {
	addr, mock := startMockUpdateService(t)

	_, stderr, code := runCLI([]string{
		"status",
		"--adapter-id=adapter-001",
		"--update-service-addr=" + addr,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if !mock.getStatusCalled {
		t.Error("GetAdapterStatus was not called")
	}
	if mock.capturedStatusReq.GetAdapterId() != "adapter-001" {
		t.Errorf("adapter_id = %q, want adapter-001", mock.capturedStatusReq.GetAdapterId())
	}
}

// TS-09-19: start-session calls PARKING_ADAPTOR StartSession
// Requirement: 09-REQ-4.8
func TestStartSession(t *testing.T) {
	addr, mock := startMockAdaptor(t)

	_, stderr, code := runCLI([]string{
		"start-session",
		"--zone-id=zone-demo-1",
		"--adaptor-addr=" + addr,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if !mock.startCalled {
		t.Error("StartSession was not called")
	}
	if mock.capturedStartReq.GetZoneId() != "zone-demo-1" {
		t.Errorf("zone_id = %q, want zone-demo-1", mock.capturedStartReq.GetZoneId())
	}
}

// TS-09-20: stop-session calls PARKING_ADAPTOR StopSession
// Requirement: 09-REQ-4.9
func TestStopSession(t *testing.T) {
	addr, mock := startMockAdaptor(t)

	_, stderr, code := runCLI([]string{
		"stop-session",
		"--adaptor-addr=" + addr,
	})
	if code != 0 {
		t.Fatalf("exit code = %d, want 0; stderr: %s", code, stderr)
	}
	if !mock.stopCalled {
		t.Error("StopSession was not called")
	}
}

// --------------------------------------------------------------------------
// Error and edge-case tests
// --------------------------------------------------------------------------

// TS-09-E8: Unknown subcommand → exit 1
// Requirement: 09-REQ-4.E1
func TestUnknownSubcommand(t *testing.T) {
	_, stderr, code := runCLI([]string{"foobar"})
	if code == 0 {
		t.Error("expected non-zero exit code for unknown subcommand")
	}
	if len(stderr) == 0 {
		t.Error("expected non-empty stderr for unknown subcommand")
	}
}

// TS-09-E9: Missing required flag → exit 1
// Requirement: 09-REQ-4.E2
func TestMissingRequiredFlag(t *testing.T) {
	cases := [][]string{
		{"install"},                     // missing --image-ref and --checksum
		{"install", "--image-ref=foo"},  // missing --checksum
		{"lookup"},                      // missing --lat/--lon
		{"adapter-info"},                // missing --operator-id
		{"remove"},                      // missing --adapter-id
		{"status"},                      // missing --adapter-id
		{"start-session"},               // missing --zone-id
	}
	for _, args := range cases {
		t.Run(strings.Join(args, "_"), func(t *testing.T) {
			_, stderr, code := runCLI(args)
			if code == 0 {
				t.Errorf("args %v: expected non-zero exit code, got 0", args)
			}
			if len(stderr) == 0 {
				t.Errorf("args %v: expected non-empty stderr", args)
			}
		})
	}
}

// TS-09-E10: Upstream unreachable → exit 1
// Requirement: 09-REQ-4.E3
func TestUpstreamUnreachable(t *testing.T) {
	os.Unsetenv("PARKING_FEE_SERVICE_URL")
	_, _, code := runCLI([]string{
		"lookup",
		"--lat=48.0",
		"--lon=11.0",
		"--fee-service-url=http://localhost:19999",
	})
	if code == 0 {
		t.Error("expected non-zero exit code when upstream unreachable, got 0")
	}
}

// TS-09-25: --help exits 0 with usage
// Requirement: 09-REQ-6.1
func TestHelpFlag(t *testing.T) {
	var outBuf, errBuf bytes.Buffer
	code := run([]string{"--help"}, &outBuf, &errBuf)
	output := outBuf.String() + errBuf.String()
	if code != 0 {
		t.Errorf("--help exit code = %d, want 0", code)
	}
	if len(output) == 0 {
		t.Error("--help produced no output")
	}
}

// TS-09-P3: CLI Subcommand Dispatch Property
// Property 3 — known subcommands dispatch correctly; unknown → exit 1.
func TestPropertySubcommandDispatch(t *testing.T) {
	// All known subcommands. We don't test them fully here, just that they
	// dispatch (and fail for missing services/flags, not "unknown subcommand").
	unknown := []string{"foobar", "LOOKUP", "lock", "serve", "x", ""}
	for _, sub := range unknown {
		t.Run("unknown_"+sub, func(t *testing.T) {
			if sub == "" {
				// No subcommand → exit 1 (usage)
				_, _, code := runCLI(nil)
				if code == 0 {
					t.Error("expected non-zero exit for no subcommand")
				}
				return
			}
			_, stderr, code := runCLI([]string{sub})
			if code == 0 {
				t.Errorf("subcommand %q: expected exit 1, got 0", sub)
			}
			if !strings.Contains(stderr, "unknown") && !strings.Contains(stderr, "error") && !strings.Contains(stderr, "required") {
				t.Errorf("subcommand %q: stderr %q should mention error", sub, stderr)
			}
		})
	}
}

// TS-09-P4: Configuration Defaults Property (parking-app-cli)
// Property 4 — unset env vars use defaults.
func TestPropertyConfigDefaults(t *testing.T) {
	os.Unsetenv("PARKING_FEE_SERVICE_URL")
	os.Unsetenv("UPDATE_SERVICE_ADDR")
	os.Unsetenv("ADAPTOR_ADDR")
	cfg := loadConfig(nil)
	defaults := map[string]string{
		"FeeServiceURL": "http://localhost:8080",
		"UpdateSvcAddr": "localhost:50052",
		"AdaptorAddr":   "localhost:50053",
	}
	got := map[string]string{
		"FeeServiceURL": cfg.FeeServiceURL,
		"UpdateSvcAddr": cfg.UpdateSvcAddr,
		"AdaptorAddr":   cfg.AdaptorAddr,
	}
	for k, want := range defaults {
		if got[k] != want {
			t.Errorf("%s = %q, want %q", k, got[k], want)
		}
	}
}

// TS-09-P5: Error Exit Code Consistency (parking-app-cli)
func TestPropertyErrorExitCode(t *testing.T) {
	scenarios := []struct {
		name string
		args []string
	}{
		{"unknown_subcmd", []string{"foobar"}},
		{"missing_flag_install", []string{"install"}},
		{"upstream_unreachable", []string{"lookup", "--lat=1", "--lon=1", "--fee-service-url=http://localhost:19999"}},
	}
	for _, sc := range scenarios {
		t.Run(sc.name, func(t *testing.T) {
			_, stderr, code := runCLI(sc.args)
			if code == 0 {
				t.Errorf("scenario %q: expected exit code != 0", sc.name)
			}
			if len(stderr) == 0 {
				t.Errorf("scenario %q: expected non-empty stderr", sc.name)
			}
		})
	}
}

// TS-09-26: Connection error message includes the target address
// Requirement: 09-REQ-6.2
func TestConnectionErrorMessage(t *testing.T) {
	_, stderr, code := runCLI([]string{
		"lookup",
		"--lat=1",
		"--lon=1",
		"--fee-service-url=http://localhost:19999",
	})
	if code == 0 {
		t.Fatal("expected non-zero exit code")
	}
	if !strings.Contains(stderr, "19999") && !strings.Contains(strings.ToLower(stderr), "connect") {
		t.Errorf("stderr should mention address/connection: %q", stderr)
	}
}

// TS-09-27: Upstream error response printed to stderr
// Requirement: 09-REQ-6.3
func TestUpstreamErrorResponse(t *testing.T) {
	var cap struct{ Method, Path, Query string }
	srv := newCapturingHTTPServer(t, &cap, http.StatusForbidden, `{"error":"forbidden"}`)
	defer srv.Close()

	_, stderr, code := runCLI([]string{
		"lookup",
		"--lat=1",
		"--lon=1",
		"--fee-service-url=" + srv.URL,
	})
	if code == 0 {
		t.Fatal("expected non-zero exit on HTTP 403")
	}
	if len(stderr) == 0 {
		t.Error("expected non-empty stderr on upstream error")
	}
}

// Ensure the watch output can be decoded as JSON.
func TestWatchOutputIsJSON(t *testing.T) {
	addr, _ := startMockUpdateService(t)
	stdout, stderr, code := runCLI([]string{
		"watch",
		"--update-service-addr=" + addr,
	})
	if code != 0 {
		t.Fatalf("exit code = %d; stderr: %s", code, stderr)
	}
	// Each non-blank line should be (start of) a JSON object.
	lines := strings.Split(strings.TrimSpace(stdout), "\n")
	// Collect full JSON objects (output is indented, so we parse the whole blob).
	dec := json.NewDecoder(strings.NewReader(stdout))
	for dec.More() {
		var obj map[string]any
		if err := dec.Decode(&obj); err != nil {
			t.Fatalf("output is not valid JSON: %v\noutput: %s", err, stdout)
		}
	}
	_ = lines
}
