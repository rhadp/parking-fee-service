// Test helpers for integration tests.
//
// buildBinary builds a Go binary from the mock module using go build.
// startMockHTTPServer starts a local HTTP server that captures requests and
// returns a configured response — used to verify the correct HTTP calls are
// made by CLI tools.
// startTCPListener starts a bare TCP listener suitable as a fake gRPC endpoint.
package integration

import (
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
	mu      sync.Mutex
	last    *capturedRequest
	status  int
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

// startTCPListener opens a TCP listener on a random port.  It accepts
// connections and closes them immediately, simulating an unreachable / stub
// gRPC server (the gRPC client will connect but fail the handshake).
func startTCPListener(t *testing.T) string {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("startTCPListener: %v", err)
	}
	t.Cleanup(func() { lis.Close() })
	go func() {
		for {
			conn, err := lis.Accept()
			if err != nil {
				return
			}
			conn.Close()
		}
	}()
	return lis.Addr().String()
}
