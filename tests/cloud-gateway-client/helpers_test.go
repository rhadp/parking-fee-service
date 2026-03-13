// Package cloudgatewayclient contains integration tests for the
// CLOUD_GATEWAY_CLIENT component (spec 04_cloud_gateway_client).
//
// Tests verify NATS command subscription, command forwarding to DATA_BROKER,
// response relay, telemetry publishing, self-registration, startup logging,
// and graceful shutdown.
//
// Live tests require running NATS and Kuksa Databroker containers (started via
// deployments/compose.yml) and skip gracefully when Podman or required tools
// are unavailable.
package cloudgatewayclient

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
)

// ---- Constants ---------------------------------------------------------------

const (
	defaultNATSURL      = "nats://localhost:4222"
	defaultBrokerAddr   = "localhost:55556"
	testVIN             = "TEST_VIN_001"
	testBearerToken     = "demo-token"
	tcpEndpoint         = "localhost:55556"

	// VSS signal paths
	signalLockCommand = "Vehicle.Command.Door.Lock"
	signalResponse    = "Vehicle.Command.Door.Response"
	signalIsLocked    = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
	signalLatitude    = "Vehicle.CurrentLocation.Latitude"
	signalLongitude   = "Vehicle.CurrentLocation.Longitude"
	signalParking     = "Vehicle.Parking.SessionActive"
)

// ---- Repository helpers ------------------------------------------------------

// repoRoot returns the absolute path to the repository root.
// Tests live in tests/cloud-gateway-client/, so the root is two levels up.
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

// ---- Skip conditions ---------------------------------------------------------

// requirePodman skips the test if podman is not available on PATH.
func requirePodman(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("skipping: podman not available")
	}
}

// requireGrpcurl skips the test if grpcurl is not available on PATH.
func requireGrpcurl(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("skipping: grpcurl not available")
	}
}

// requireInfrastructure skips the test if podman or grpcurl is unavailable.
func requireInfrastructure(t *testing.T) {
	t.Helper()
	requirePodman(t)
	requireGrpcurl(t)
}

// ---- Container lifecycle -----------------------------------------------------

// composeFile returns the path to deployments/compose.yml.
func composeFile(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "deployments", "compose.yml")
}

// startNATS starts the NATS container via compose and waits for it to accept
// connections.
func startNATS(t *testing.T) {
	t.Helper()
	cf := composeFile(t)
	root := repoRoot(t)

	// Remove any stale container.
	cleanCmd := exec.Command("podman", "compose", "-f", cf, "rm", "-f", "nats")
	cleanCmd.Dir = root
	_ = cleanCmd.Run()

	cmd := exec.Command("podman", "compose", "-f", cf, "up", "-d", "nats")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to start NATS: %v\n%s", err, string(out))
	}
	t.Cleanup(func() {
		stopCmd := exec.Command("podman", "compose", "-f", cf, "stop", "nats")
		stopCmd.Dir = root
		_ = stopCmd.Run()
		rmCmd := exec.Command("podman", "compose", "-f", cf, "rm", "-f", "nats")
		rmCmd.Dir = root
		_ = rmCmd.Run()
	})

	if !waitForTCP(t, "localhost:4222", 10*time.Second) {
		t.Fatal("NATS did not become available within timeout")
	}
}

// startDatabroker starts the databroker container and waits for health.
func startDatabroker(t *testing.T) {
	t.Helper()
	root := repoRoot(t)
	cf := composeFile(t)

	// Ensure the host UDS directory exists (needed for the bind mount volume).
	_ = os.MkdirAll("/tmp/kuksa", 0o755)

	// On macOS with podman machine, /tmp/kuksa must also exist in the VM.
	_ = exec.Command("podman", "machine", "ssh", "--", "mkdir", "-p", "/tmp/kuksa").Run()

	// Remove any stale volume/container to avoid bind mount errors.
	cleanCmd := exec.Command("podman", "compose", "-f", cf, "down", "--volumes", "databroker")
	cleanCmd.Dir = root
	_ = cleanCmd.Run()

	cmd := exec.Command("podman", "compose", "-f", cf, "up", "-d", "databroker")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to start databroker: %v\n%s", err, string(out))
	}
	t.Cleanup(func() {
		stopCmd := exec.Command("podman", "compose", "-f", cf, "down", "--volumes")
		stopCmd.Dir = root
		_ = stopCmd.Run()
	})

	if !waitForDatabrokerHealthy(t, 15*time.Second) {
		t.Fatal("databroker did not become healthy within timeout")
	}
}


// waitForTCP polls a TCP address until it accepts connections.
func waitForTCP(t *testing.T, addr string, timeout time.Duration) bool {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", addr, 500*time.Millisecond)
		if err == nil {
			conn.Close()
			return true
		}
		time.Sleep(250 * time.Millisecond)
	}
	return false
}

// protoDir returns the path to the vendored kuksa proto files.
func protoDir(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "rhivos", "cloud-gateway-client", "proto")
}

// waitForDatabrokerHealthy waits for the databroker gRPC endpoint to respond.
func waitForDatabrokerHealthy(t *testing.T, timeout time.Duration) bool {
	t.Helper()
	deadline := time.Now().Add(timeout)
	pDir := protoDir(t)
	for time.Now().Before(deadline) {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		err := exec.CommandContext(ctx, "grpcurl", "-plaintext",
			"-import-path", pDir, "-proto", "kuksa/val/v1/val.proto",
			tcpEndpoint, "kuksa.val.v1.VAL/GetServerInfo").Run()
		cancel()
		if err == nil {
			return true
		}
		time.Sleep(500 * time.Millisecond)
	}
	return false
}

// ---- Rust binary build -------------------------------------------------------

// buildCloudGatewayClient builds the cloud-gateway-client binary and returns
// its path.
func buildCloudGatewayClient(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")
	cmd := exec.Command("cargo", "build", "-p", "cloud-gateway-client")
	cmd.Dir = rhivosDir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("cargo build -p cloud-gateway-client failed: %v\n%s", err, string(out))
	}
	binPath := filepath.Join(rhivosDir, "target", "debug", "cloud-gateway-client")
	if _, err := os.Stat(binPath); err != nil {
		t.Fatalf("expected binary at %s: %v", binPath, err)
	}
	return binPath
}

// ---- Service process management ----------------------------------------------

// serviceProcess wraps a running cloud-gateway-client process.
type serviceProcess struct {
	cmd    *exec.Cmd
	stdout *bytes.Buffer
	stderr *bytes.Buffer
}

// startService starts the cloud-gateway-client binary with the given
// environment. It registers a cleanup that kills the process if still running.
func startService(t *testing.T, binPath string, env map[string]string) *serviceProcess {
	t.Helper()
	cmd := exec.Command(binPath)

	// Build env: inherit current env, then overlay test-specific vars.
	cmdEnv := os.Environ()
	for k, v := range env {
		cmdEnv = append(cmdEnv, fmt.Sprintf("%s=%s", k, v))
	}
	cmd.Env = cmdEnv

	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start cloud-gateway-client: %v", err)
	}

	sp := &serviceProcess{cmd: cmd, stdout: &stdout, stderr: &stderr}

	t.Cleanup(func() {
		if cmd.ProcessState == nil || !cmd.ProcessState.Exited() {
			_ = cmd.Process.Signal(syscall.SIGTERM)
			done := make(chan struct{})
			go func() {
				_ = cmd.Wait()
				close(done)
			}()
			select {
			case <-done:
			case <-time.After(3 * time.Second):
				_ = cmd.Process.Kill()
				_ = cmd.Wait()
			}
		}
	})

	return sp
}

// waitForReady polls the service output until "ready" appears in stderr/stdout.
func (sp *serviceProcess) waitForReady(t *testing.T, timeout time.Duration) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		combined := sp.stdout.String() + sp.stderr.String()
		if strings.Contains(combined, "ready") {
			return
		}
		time.Sleep(200 * time.Millisecond)
	}
	t.Fatalf("service did not become ready within %v\nstdout: %s\nstderr: %s",
		timeout, sp.stdout.String(), sp.stderr.String())
}

// sendSignal sends a signal to the service process.
func (sp *serviceProcess) sendSignal(sig syscall.Signal) error {
	return sp.cmd.Process.Signal(sig)
}

// wait waits for the process to exit and returns the exit code.
func (sp *serviceProcess) wait(timeout time.Duration) (int, error) {
	done := make(chan error, 1)
	go func() {
		done <- sp.cmd.Wait()
	}()
	select {
	case err := <-done:
		if err == nil {
			return 0, nil
		}
		if exitErr, ok := err.(*exec.ExitError); ok {
			return exitErr.ExitCode(), nil
		}
		return -1, err
	case <-time.After(timeout):
		return -1, fmt.Errorf("process did not exit within %v", timeout)
	}
}

// output returns the combined stdout and stderr output.
func (sp *serviceProcess) output() string {
	return sp.stdout.String() + sp.stderr.String()
}

// ---- NATS helpers ------------------------------------------------------------

// connectNATS connects to NATS and registers cleanup.
func connectNATS(t *testing.T) *nats.Conn {
	t.Helper()
	nc, err := nats.Connect(defaultNATSURL)
	if err != nil {
		t.Fatalf("failed to connect to NATS: %v", err)
	}
	t.Cleanup(func() { nc.Close() })
	return nc
}

// publishCommand publishes a command message with an Authorization header.
func publishCommand(t *testing.T, nc *nats.Conn, vin, payload, token string) {
	t.Helper()
	msg := &nats.Msg{
		Subject: fmt.Sprintf("vehicles.%s.commands", vin),
		Data:    []byte(payload),
		Header:  nats.Header{},
	}
	if token != "" {
		msg.Header.Set("Authorization", fmt.Sprintf("Bearer %s", token))
	}
	if err := nc.PublishMsg(msg); err != nil {
		t.Fatalf("failed to publish command: %v", err)
	}
	if err := nc.Flush(); err != nil {
		t.Fatalf("NATS flush failed: %v", err)
	}
}

// validCommandPayload returns a valid lock command JSON string.
func validCommandPayload() string {
	return `{"command_id":"test-cmd-001","action":"lock","doors":["driver"],"source":"integration_test","vin":"TEST_VIN_001","timestamp":1700000000}`
}

// ---- gRPC helpers (grpcurl) --------------------------------------------------

// grpcCall runs a grpcurl command against the given endpoint. It automatically
// includes -import-path and -proto flags since kuksa-databroker 0.5.0 does not
// support gRPC reflection.
func grpcCall(pDir, endpoint, method, body string) (string, error) {
	args := []string{
		"-plaintext",
		"-import-path", pDir,
		"-proto", "kuksa/val/v1/val.proto",
	}
	if body != "" {
		args = append(args, "-d", body)
	}
	args = append(args, endpoint, method)
	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	return string(out), err
}

// grpcGet fetches the current value of a signal.
func grpcGet(t *testing.T, endpoint, path string) (string, error) {
	t.Helper()
	body := fmt.Sprintf(`{"entries": [{"path": %q, "fields": ["FIELD_VALUE"]}]}`, path)
	return grpcCall(protoDir(t), endpoint, "kuksa.val.v1.VAL/Get", body)
}

// grpcSetString sets a string signal value.
func grpcSetString(t *testing.T, endpoint, path, value string) (string, error) {
	t.Helper()
	body := fmt.Sprintf(
		`{"updates": [{"entry": {"path": %q, "value": {"string": %q}}, "fields": ["FIELD_VALUE"]}]}`,
		path, value,
	)
	return grpcCall(protoDir(t), endpoint, "kuksa.val.v1.VAL/Set", body)
}

// grpcSetBool sets a boolean signal value.
func grpcSetBool(t *testing.T, endpoint, path string, value bool) (string, error) {
	t.Helper()
	v := "false"
	if value {
		v = "true"
	}
	body := fmt.Sprintf(
		`{"updates": [{"entry": {"path": %q, "value": {"bool": %s}}, "fields": ["FIELD_VALUE"]}]}`,
		path, v,
	)
	return grpcCall(protoDir(t), endpoint, "kuksa.val.v1.VAL/Set", body)
}

// ---- Default service env -----------------------------------------------------

// defaultServiceEnv returns environment variables for the cloud-gateway-client
// when running integration tests.
func defaultServiceEnv() map[string]string {
	return map[string]string{
		"VIN":            testVIN,
		"NATS_URL":       defaultNATSURL,
		"DATABROKER_ADDR": fmt.Sprintf("http://%s", defaultBrokerAddr),
		"BEARER_TOKEN":   testBearerToken,
		"RUST_LOG":       "info",
	}
}

// ---- JSON helpers ------------------------------------------------------------

// parseJSON parses a JSON byte slice into a map.
func parseJSON(t *testing.T, data []byte) map[string]interface{} {
	t.Helper()
	var m map[string]interface{}
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("failed to parse JSON %q: %v", string(data), err)
	}
	return m
}
