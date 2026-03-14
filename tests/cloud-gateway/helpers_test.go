// Package cloudgateway contains integration tests for the CLOUD_GATEWAY
// component (spec 06_cloud_gateway). Tests verify NATS command routing,
// bearer token forwarding, response subscription, telemetry logging, startup
// logging, and graceful shutdown.
//
// Live tests require a running NATS server (started via podman compose or
// already available on localhost:4222) and the cloud-gateway binary (built
// from backend/cloud-gateway/). Tests skip gracefully when prerequisites are
// unavailable.
package cloudgateway

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sync"
	"sync/atomic"
	"syscall"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
)

// ---- Constants ---------------------------------------------------------------

const (
	defaultNATSURL = "nats://localhost:4222"
	testVIN        = "VIN12345"
	testToken      = "demo-token-car1"
)

// portCounter provides unique ports for each test gateway instance.
var portCounter int32 = 18080

// nextPort returns the next available port for a test gateway instance.
func nextPort() int {
	return int(atomic.AddInt32(&portCounter, 1))
}

// ---- safeBuffer -------------------------------------------------------------

// safeBuffer is a bytes.Buffer protected by a mutex, safe for concurrent
// reads and writes from process output goroutines.
type safeBuffer struct {
	mu  sync.Mutex
	buf bytes.Buffer
}

func (sb *safeBuffer) Write(p []byte) (n int, err error) {
	sb.mu.Lock()
	defer sb.mu.Unlock()
	return sb.buf.Write(p)
}

func (sb *safeBuffer) String() string {
	sb.mu.Lock()
	defer sb.mu.Unlock()
	return sb.buf.String()
}

// ---- Repository helpers ------------------------------------------------------

// repoRoot returns the absolute path to the repository root.
// Tests live in tests/cloud-gateway/, two levels up from the repo root.
func repoRoot(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	root := filepath.Join(filepath.Dir(filename), "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return abs
}

// composeFile returns the path to deployments/compose.yml.
func composeFile(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "deployments", "compose.yml")
}

// ---- Skip helpers -----------------------------------------------------------

// requirePodman skips the test if podman is not on PATH.
func requirePodman(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("skipping: podman not available")
	}
}

// ---- NATS availability -------------------------------------------------------

// isNATSRunning returns true if NATS is already listening on :4222.
func isNATSRunning() bool {
	conn, err := net.DialTimeout("tcp", "localhost:4222", 500*time.Millisecond)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

// ensureNATS verifies that NATS is available on localhost:4222.
// If NATS is not running and podman is available, it starts NATS via compose.
// Otherwise it skips the test.
func ensureNATS(t *testing.T) {
	t.Helper()
	if isNATSRunning() {
		return
	}
	// Try podman compose
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("skipping: NATS not available and podman not found")
	}
	startNATSViaPodman(t)
}

// startNATSViaPodman starts the NATS container via podman compose and waits
// for it to accept connections on :4222. Cleanup stops and removes the container.
func startNATSViaPodman(t *testing.T) {
	t.Helper()
	root := repoRoot(t)
	cf := composeFile(t)

	// Remove any stale container.
	cleanCmd := exec.Command("podman", "compose", "-f", cf, "rm", "-f", "nats")
	cleanCmd.Dir = root
	_ = cleanCmd.Run()

	cmd := exec.Command("podman", "compose", "-f", cf, "up", "-d", "nats")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to start NATS via podman compose: %v\n%s", err, string(out))
	}

	t.Cleanup(func() {
		stopCmd := exec.Command("podman", "compose", "-f", cf, "stop", "nats")
		stopCmd.Dir = root
		_ = stopCmd.Run()
		rmCmd := exec.Command("podman", "compose", "-f", cf, "rm", "-f", "nats")
		rmCmd.Dir = root
		_ = rmCmd.Run()
	})

	if !waitForTCP(t, "localhost:4222", 15*time.Second) {
		t.Fatal("NATS did not become available within timeout")
	}
}

// waitForTCP polls a TCP address until it accepts connections or the timeout elapses.
func waitForTCP(t *testing.T, addr string, timeout time.Duration) bool {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 300*time.Millisecond)
		if err == nil {
			conn.Close()
			return true
		}
		time.Sleep(200 * time.Millisecond)
	}
	return false
}

// ---- Cloud-gateway binary build ---------------------------------------------

var (
	buildOnce sync.Once
	builtBin  string
	buildErr  error
)

// ensureBinary builds the cloud-gateway binary (once per test run) and
// returns the path to the compiled executable.
func ensureBinary(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)

	buildOnce.Do(func() {
		binPath := filepath.Join(os.TempDir(), "cloud-gateway-test-bin")
		cmd := exec.Command("go", "build", "-o", binPath, "./backend/cloud-gateway/")
		cmd.Dir = root
		out, err := cmd.CombinedOutput()
		if err != nil {
			buildErr = fmt.Errorf("go build ./backend/cloud-gateway/ failed: %v\n%s", err, string(out))
			return
		}
		builtBin = binPath
	})

	if buildErr != nil {
		t.Fatalf("failed to build cloud-gateway: %v", buildErr)
	}
	return builtBin
}

// ---- Config helpers ---------------------------------------------------------

// gatewayConfig holds configuration for a test gateway instance.
type gatewayConfig struct {
	Port           int
	NatsURL        string
	TimeoutSeconds int
	Tokens         map[string]string // token -> VIN
}

// createConfig writes a temporary config JSON file and returns its path.
func createConfig(t *testing.T, cfg gatewayConfig) string {
	t.Helper()
	if cfg.NatsURL == "" {
		cfg.NatsURL = defaultNATSURL
	}
	if cfg.TimeoutSeconds == 0 {
		cfg.TimeoutSeconds = 30
	}

	type tokenEntry struct {
		Token string `json:"token"`
		VIN   string `json:"vin"`
	}
	tokens := make([]tokenEntry, 0, len(cfg.Tokens))
	for tok, vin := range cfg.Tokens {
		tokens = append(tokens, tokenEntry{Token: tok, VIN: vin})
	}

	configData := map[string]interface{}{
		"port":                    cfg.Port,
		"nats_url":                cfg.NatsURL,
		"command_timeout_seconds": cfg.TimeoutSeconds,
		"tokens":                  tokens,
	}

	data, err := json.Marshal(configData)
	if err != nil {
		t.Fatalf("failed to marshal test config: %v", err)
	}

	f, err := os.CreateTemp(t.TempDir(), "cloud-gateway-test-config-*.json")
	if err != nil {
		t.Fatalf("failed to create temp config: %v", err)
	}
	defer f.Close()

	if _, err := f.Write(data); err != nil {
		t.Fatalf("failed to write temp config: %v", err)
	}
	return f.Name()
}

// ---- Gateway process management ---------------------------------------------

// gatewayProcess wraps a running cloud-gateway process with log capture
// and lifecycle management.
type gatewayProcess struct {
	cmd    *exec.Cmd
	outBuf *safeBuffer
	port   int
	done   chan struct{}
	exitCode int
}

// startGateway builds (if needed) and starts the cloud-gateway binary with
// the given config file. It waits for the health endpoint to respond before
// returning. Cleanup kills the process.
func startGateway(t *testing.T, cfg gatewayConfig) *gatewayProcess {
	t.Helper()
	bin := ensureBinary(t)
	configPath := createConfig(t, cfg)

	buf := &safeBuffer{}
	cmd := exec.Command(bin)
	cmd.Env = append(os.Environ(), "CONFIG_PATH="+configPath)
	cmd.Stdout = buf
	cmd.Stderr = buf

	gp := &gatewayProcess{
		cmd:    cmd,
		outBuf: buf,
		port:   cfg.Port,
		done:   make(chan struct{}),
	}

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start cloud-gateway: %v", err)
	}

	// Background goroutine to track exit code.
	go func() {
		err := cmd.Wait()
		if err != nil {
			if ee, ok := err.(*exec.ExitError); ok {
				gp.exitCode = ee.ExitCode()
			} else {
				gp.exitCode = -1
			}
		} else {
			gp.exitCode = 0
		}
		close(gp.done)
	}()

	t.Cleanup(func() {
		gp.kill()
		gp.waitForExit(5 * time.Second)
	})

	if !gp.waitForHealth(15 * time.Second) {
		logs := gp.logs()
		gp.kill()
		t.Fatalf("cloud-gateway did not become healthy on port %d; logs:\n%s", cfg.Port, logs)
	}

	return gp
}

// kill sends SIGKILL to the process (best-effort).
func (gp *gatewayProcess) kill() {
	if gp.cmd.Process != nil {
		_ = gp.cmd.Process.Kill()
	}
}

// sendSIGTERM sends SIGTERM to the process.
func (gp *gatewayProcess) sendSIGTERM() {
	if gp.cmd.Process != nil {
		_ = gp.cmd.Process.Signal(syscall.SIGTERM)
	}
}

// sendSIGINT sends SIGINT to the process.
func (gp *gatewayProcess) sendSIGINT() {
	if gp.cmd.Process != nil {
		_ = gp.cmd.Process.Signal(syscall.SIGINT)
	}
}

// waitForExit waits for the process to exit within timeout.
// Returns (exitCode, true) on success or (-1, false) on timeout.
func (gp *gatewayProcess) waitForExit(timeout time.Duration) (int, bool) {
	select {
	case <-gp.done:
		return gp.exitCode, true
	case <-time.After(timeout):
		return -1, false
	}
}

// waitForHealth polls the health endpoint until it responds 200 or timeout elapses.
func (gp *gatewayProcess) waitForHealth(timeout time.Duration) bool {
	url := fmt.Sprintf("http://localhost:%d/health", gp.port)
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		resp, err := http.Get(url) //nolint:noctx
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == http.StatusOK {
				return true
			}
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

// logContains reports whether the captured log output contains s.
func (gp *gatewayProcess) logContains(s string) bool {
	return bytes.Contains([]byte(gp.outBuf.String()), []byte(s))
}

// logs returns all captured log output.
func (gp *gatewayProcess) logs() string {
	return gp.outBuf.String()
}

// waitForLog polls until the log contains s or the timeout elapses.
func (gp *gatewayProcess) waitForLog(s string, timeout time.Duration) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if gp.logContains(s) {
			return true
		}
		time.Sleep(100 * time.Millisecond)
	}
	return false
}

// ---- NATS helpers -----------------------------------------------------------

// connectNATS connects to the default NATS URL and registers cleanup.
func connectNATS(t *testing.T) *nats.Conn {
	t.Helper()
	nc, err := nats.Connect(defaultNATSURL)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	t.Cleanup(func() { nc.Close() })
	return nc
}

// ---- HTTP helpers -----------------------------------------------------------

// postCommand sends a POST /vehicles/{vin}/commands request to the gateway.
func postCommand(t *testing.T, port int, vin, token, body string) *http.Response {
	t.Helper()
	url := fmt.Sprintf("http://localhost:%d/vehicles/%s/commands", port, vin)
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewBufferString(body))
	if err != nil {
		t.Fatalf("failed to create request: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("failed to send POST /vehicles/%s/commands: %v", vin, err)
	}
	return resp
}

// getCommandStatus sends a GET /vehicles/{vin}/commands/{id} request.
func getCommandStatus(t *testing.T, port int, vin, token, commandID string) *http.Response {
	t.Helper()
	url := fmt.Sprintf("http://localhost:%d/vehicles/%s/commands/%s", port, vin, commandID)
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		t.Fatalf("failed to create request: %v", err)
	}
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		t.Fatalf("failed to send GET /vehicles/%s/commands/%s: %v", vin, commandID, err)
	}
	return resp
}

// decodeJSON decodes the response body as a JSON object.
func decodeJSON(t *testing.T, resp *http.Response) map[string]interface{} {
	t.Helper()
	defer resp.Body.Close()
	var m map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&m); err != nil {
		t.Fatalf("failed to decode JSON response: %v", err)
	}
	return m
}

// defaultTestConfig returns a gatewayConfig suitable for integration tests.
func defaultTestConfig() gatewayConfig {
	return gatewayConfig{
		Port:           nextPort(),
		NatsURL:        defaultNATSURL,
		TimeoutSeconds: 30,
		Tokens:         map[string]string{testToken: testVIN},
	}
}
