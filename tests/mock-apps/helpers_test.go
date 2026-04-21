// Package mockappstests provides integration tests for mock CLI apps.
package mockappstests

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
	"strings"
	"testing"
	"time"

	adapterpb "github.com/rhadp/parking-fee-service/tests/mock-apps/pb/adapter"
	updatepb "github.com/rhadp/parking-fee-service/tests/mock-apps/pb/update"
	"google.golang.org/grpc"
)

// findProjectRoot walks up from the current directory to find the go.work file,
// which identifies the project root.
func findProjectRoot(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "go.work")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find project root (go.work not found)")
		}
		dir = parent
	}
}

// buildBinary compiles a Go binary from the mock/ directory and returns its path.
// The binary is placed in sharedBinDir which persists for the whole test run.
func buildBinary(t *testing.T, name string) string {
	t.Helper()
	root := findProjectRoot(t)
	binPath := filepath.Join(sharedBinDir, name)

	pkgDir := filepath.Join(root, "mock", name)
	cmd := exec.Command("go", "build", "-o", binPath, pkgDir)
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("build %s: %v\n%s", name, err, out)
	}
	return binPath
}

// mockHTTPServer creates a test HTTP server that records the last received request
// and serves the given response JSON.
type mockHTTPServer struct {
	ts          *httptest.Server
	lastMethod  string
	lastPath    string
	lastBody    []byte
	lastHeaders http.Header
	respStatus  int
	respBody    string
}

// newMockHTTPServer creates a new mock HTTP server returning the given status and body.
func newMockHTTPServer(t *testing.T, status int, body string) *mockHTTPServer {
	t.Helper()
	m := &mockHTTPServer{respStatus: status, respBody: body}
	m.ts = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		m.lastMethod = r.Method
		m.lastPath = r.URL.Path
		if r.URL.RawQuery != "" {
			m.lastPath += "?" + r.URL.RawQuery
		}
		m.lastHeaders = r.Header.Clone()
		body, _ := io.ReadAll(r.Body)
		m.lastBody = body
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(m.respStatus)
		w.Write([]byte(m.respBody)) //nolint
	}))
	t.Cleanup(m.ts.Close)
	return m
}

// URL returns the base URL of the mock server.
func (m *mockHTTPServer) URL() string { return m.ts.URL }

// Header returns the value of a request header received by the mock server.
func (m *mockHTTPServer) Header(key string) string { return m.lastHeaders.Get(key) }

// decodeLastBody decodes the last received request body as JSON.
func (m *mockHTTPServer) decodeLastBody(t *testing.T) map[string]any {
	t.Helper()
	var v map[string]any
	if err := json.Unmarshal(m.lastBody, &v); err != nil {
		t.Fatalf("decode request body: %v (body=%q)", err, m.lastBody)
	}
	return v
}

// waitForPort polls until the given TCP port is listening or the deadline is reached.
func waitForPort(addr string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 100*time.Millisecond)
		if err == nil {
			conn.Close()
			return nil
		}
		time.Sleep(50 * time.Millisecond)
	}
	return &timeoutError{addr: addr}
}

type timeoutError struct{ addr string }

func (e *timeoutError) Error() string { return "timeout waiting for " + e.addr + " to listen" }

// runBinary runs the given binary with args, captures stdout and stderr,
// and returns the exit code. Returns -1 if the process could not be started.
func runBinary(t *testing.T, binary string, args ...string) (stdout, stderr string, exitCode int) {
	t.Helper()
	cmd := exec.Command(binary, args...)
	var outBuf, errBuf strings.Builder
	cmd.Stdout = &outBuf
	cmd.Stderr = &errBuf
	err := cmd.Run()
	stdout = outBuf.String()
	stderr = errBuf.String()
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			exitCode = exitErr.ExitCode()
		} else {
			exitCode = -1
		}
	} else {
		exitCode = 0
	}
	return
}

// ---------------------------------------------------------------------------
// Mock gRPC server helpers
// ---------------------------------------------------------------------------

// mockUpdateServer implements UpdateServiceServer for testing.
type mockUpdateServer struct {
	updatepb.UnimplementedUpdateServiceServer
}

func (s *mockUpdateServer) InstallAdapter(_ context.Context, req *updatepb.InstallAdapterRequest) (*updatepb.InstallAdapterResponse, error) {
	return &updatepb.InstallAdapterResponse{
		JobId:     "j1",
		AdapterId: "a1",
		State:     updatepb.AdapterState_DOWNLOADING,
	}, nil
}

func (s *mockUpdateServer) ListAdapters(_ context.Context, _ *updatepb.ListAdaptersRequest) (*updatepb.ListAdaptersResponse, error) {
	return &updatepb.ListAdaptersResponse{
		Adapters: []*updatepb.AdapterInfo{
			{AdapterId: "a1", ImageRef: "test:v1", State: updatepb.AdapterState_RUNNING},
		},
	}, nil
}

func (s *mockUpdateServer) GetAdapterStatus(_ context.Context, req *updatepb.GetAdapterStatusRequest) (*updatepb.GetAdapterStatusResponse, error) {
	return &updatepb.GetAdapterStatusResponse{
		Adapter: &updatepb.AdapterInfo{
			AdapterId: req.AdapterId,
			ImageRef:  "test:v1",
			State:     updatepb.AdapterState_RUNNING,
		},
	}, nil
}

func (s *mockUpdateServer) RemoveAdapter(_ context.Context, req *updatepb.RemoveAdapterRequest) (*updatepb.RemoveAdapterResponse, error) {
	return &updatepb.RemoveAdapterResponse{
		Success: true,
		Message: "adapter removed",
	}, nil
}

// mockAdapterServer implements AdapterServiceServer for testing.
type mockAdapterServer struct {
	adapterpb.UnimplementedAdapterServiceServer
}

func (s *mockAdapterServer) StartSession(_ context.Context, req *adapterpb.StartSessionRequest) (*adapterpb.StartSessionResponse, error) {
	return &adapterpb.StartSessionResponse{SessionId: "s1"}, nil
}

func (s *mockAdapterServer) StopSession(_ context.Context, _ *adapterpb.StopSessionRequest) (*adapterpb.StopSessionResponse, error) {
	return &adapterpb.StopSessionResponse{Success: true, Message: "session stopped"}, nil
}

// startMockUpdateServer starts a mock UpdateService gRPC server on a random port
// and returns its address (host:port). The server is stopped when the test ends.
func startMockUpdateServer(t *testing.T) string {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen for mock UpdateService: %v", err)
	}

	srv := grpc.NewServer()
	updatepb.RegisterUpdateServiceServer(srv, &mockUpdateServer{})

	go srv.Serve(lis) //nolint
	t.Cleanup(func() { srv.Stop() })

	return lis.Addr().String()
}

// startMockAdapterServer starts a mock AdapterService gRPC server on a random port
// and returns its address (host:port). The server is stopped when the test ends.
func startMockAdapterServer(t *testing.T) string {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen for mock AdapterService: %v", err)
	}

	srv := grpc.NewServer()
	adapterpb.RegisterAdapterServiceServer(srv, &mockAdapterServer{})

	go srv.Serve(lis) //nolint
	t.Cleanup(func() { srv.Stop() })

	return lis.Addr().String()
}
