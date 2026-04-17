// Test helpers for integration tests.
//
// buildBinary builds a Go binary from the mock module using go build.
// startMockHTTPServer starts a local HTTP server that captures requests and
// returns a configured response — used to verify the correct HTTP calls are
// made by CLI tools.
// startTCPListener starts a gRPC server with mock implementations of
// UpdateService and AdapterService, suitable as a fake gRPC endpoint for
// parking-app-cli integration tests.
package integration

import (
	"context"
	"encoding/json"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"sync"
	"testing"

	adapterpb "github.com/sdv-demo/tests/mock-apps/pb/adapter"
	updatepb "github.com/sdv-demo/tests/mock-apps/pb/update"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/emptypb"
)

// buildBinary compiles the Go binary at the given import path (e.g.
// "github.com/sdv-demo/mock/companion-app-cli") and returns the path to the
// resulting executable.  The binary is placed in t.TempDir() and removed at
// test cleanup.
func buildBinary(t *testing.T, pkg string) string {
	t.Helper()
	dir := t.TempDir()
	out := filepath.Join(dir, "binary")
	cmd := exec.Command("go", "build", "-o", out, pkg)
	cmd.Env = os.Environ()
	if data, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("go build %s: %v\n%s", pkg, err, data)
	}
	return out
}

// capturedRequest holds the HTTP request details captured by the mock server.
type capturedRequest struct {
	Method  string
	URL     string
	Header  http.Header
	Body    []byte
	BodyMap map[string]any
}

// mockServer wraps httptest.Server and captures the last request.
type mockServer struct {
	*httptest.Server
	mu       sync.Mutex
	last     *capturedRequest
	status   int
	respBody string
}

// startMockHTTPServer starts a local HTTP server that records the incoming
// request and responds with the given status code and JSON body.
func startMockHTTPServer(t *testing.T, status int, respBody string) *mockServer {
	t.Helper()
	ms := &mockServer{status: status, respBody: respBody}
	ms.Server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		var bodyMap map[string]any
		json.Unmarshal(body, &bodyMap) //nolint

		ms.mu.Lock()
		ms.last = &capturedRequest{
			Method:  r.Method,
			URL:     r.URL.String(),
			Header:  r.Header.Clone(),
			Body:    body,
			BodyMap: bodyMap,
		}
		ms.mu.Unlock()

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(ms.status)
		io.WriteString(w, ms.respBody) //nolint
	}))
	t.Cleanup(ms.Close)
	return ms
}

// lastRequest returns the most recently captured request, or nil.
func (ms *mockServer) lastRequest() *capturedRequest {
	ms.mu.Lock()
	defer ms.mu.Unlock()
	return ms.last
}

// ── Mock gRPC implementations ────────────────────────────────────────────────

// mockUpdateServer implements UpdateServiceServer returning canned responses.
type mockUpdateServer struct {
	updatepb.UnimplementedUpdateServiceServer
}

func (m *mockUpdateServer) InstallAdapter(_ context.Context, req *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
	return &updatepb.InstallAdapterResponse{
		JobId:     "job_id-mock",
		AdapterId: "adapter-mock",
		State:     updatepb.AdapterState_ADAPTER_STATE_DOWNLOADING,
	}, nil
}

func (m *mockUpdateServer) ListAdapters(_ context.Context, _ *updatepb.ListAdaptersRequest) (*updatepb.ListAdaptersResponse, error) {
	return &updatepb.ListAdaptersResponse{
		Adapters: []*updatepb.AdapterInfo{
			{AdapterId: "adapter-1", ImageRef: "mock-image:v1", State: updatepb.AdapterState_ADAPTER_STATE_RUNNING},
		},
	}, nil
}

func (m *mockUpdateServer) GetAdapterStatus(_ context.Context, req *updatepb.GetAdapterStatusRequest) (*updatepb.AdapterInfo, error) {
	return &updatepb.AdapterInfo{
		AdapterId: req.AdapterId,
		State:     updatepb.AdapterState_ADAPTER_STATE_RUNNING,
	}, nil
}

func (m *mockUpdateServer) RemoveAdapter(_ context.Context, _ *updatepb.RemoveAdapterRequest) (*emptypb.Empty, error) {
	return &emptypb.Empty{}, nil
}

// mockAdapterServer implements AdapterServiceServer returning canned responses.
type mockAdapterServer struct {
	adapterpb.UnimplementedAdapterServiceServer
}

func (m *mockAdapterServer) StartSession(_ context.Context, req *adapterpb.StartSessionRequest) (*adapterpb.StartSessionResponse, error) {
	return &adapterpb.StartSessionResponse{
		SessionId: "session-mock",
		Active:    true,
		ZoneId:    req.ZoneId,
	}, nil
}

func (m *mockAdapterServer) StopSession(_ context.Context, _ *adapterpb.StopSessionRequest) (*adapterpb.SessionStatus, error) {
	return &adapterpb.SessionStatus{
		SessionId: "session-mock",
		Active:    false,
	}, nil
}

// startTCPListener starts a real gRPC server with mock implementations of
// UpdateService and AdapterService. It returns the listener address in
// "host:port" form. The server is shut down at test cleanup.
func startTCPListener(t *testing.T) string {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("startTCPListener: %v", err)
	}

	s := grpc.NewServer()
	updatepb.RegisterUpdateServiceServer(s, &mockUpdateServer{})
	adapterpb.RegisterAdapterServiceServer(s, &mockAdapterServer{})

	t.Cleanup(func() {
		s.GracefulStop()
		lis.Close()
	})

	go func() {
		_ = s.Serve(lis)
	}()

	return lis.Addr().String()
}
