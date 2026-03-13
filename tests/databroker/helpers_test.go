// Package databroker contains integration tests for the DATA_BROKER component
// (spec 02_data_broker). Tests verify compose.yml configuration (dual listeners,
// version pinning), VSS signal availability (custom and standard), and pub/sub
// functionality via gRPC. Live tests require a running Kuksa Databroker container
// and skip gracefully when Podman or grpcurl is unavailable.
//
// Design drift note: the spec references image tag :0.5.1 and flags --uds-path /
// --metadata, but the real available image is :0.5.0 with flags --unix-socket /
// --vss. See docs/errata/02_data_broker_compose_flags.md for details.
package databroker

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// ---- Repository helpers -------------------------------------------------------

// repoRoot returns the absolute path to the repository root.
// Tests live in tests/databroker/, so the root is two levels up.
func repoRoot(t *testing.T) string {
	t.Helper()
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	root := filepath.Join(wd, "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return abs
}

// fileExists reports whether path exists and is a regular file.
func fileExists(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return !info.IsDir()
}

// ---- Skip conditions ----------------------------------------------------------

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

// requireLiveDatabroker skips if either podman or grpcurl is unavailable.
func requireLiveDatabroker(t *testing.T) {
	t.Helper()
	requirePodman(t)
	requireGrpcurl(t)
}

// ---- Container lifecycle ------------------------------------------------------

// composeFile returns the path to deployments/compose.yml.
func composeFile(t *testing.T) string {
	t.Helper()
	return filepath.Join(repoRoot(t), "deployments", "compose.yml")
}

// startDatabroker starts the databroker container and waits up to 15 s for it to
// become healthy. Registers a cleanup function to stop it after the test.
func startDatabroker(t *testing.T) {
	t.Helper()
	root := repoRoot(t)
	cf := composeFile(t)

	// Ensure the host UDS directory exists (needed for the bind mount).
	_ = os.MkdirAll("/tmp/kuksa", 0o755)

	cmd := exec.Command("podman", "compose", "-f", cf, "up", "-d", "databroker")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to start databroker: %v\n%s", err, string(out))
	}

	t.Cleanup(func() { stopDatabroker(t) })

	if !waitForDatabroker(t, 15*time.Second) {
		t.Fatal("databroker did not become healthy within timeout")
	}
}

// stopDatabroker stops the databroker container (best-effort, errors ignored).
func stopDatabroker(t *testing.T) {
	t.Helper()
	cf := composeFile(t)
	cmd := exec.Command("podman", "compose", "-f", cf, "down")
	_ = cmd.Run()
}

// waitForDatabroker polls the TCP endpoint until the databroker responds or the
// timeout elapses.
func waitForDatabroker(t *testing.T, timeout time.Duration) bool {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if databrokerHealthyTCP() {
			return true
		}
		time.Sleep(500 * time.Millisecond)
	}
	return false
}

// databrokerHealthyTCP returns true when the TCP endpoint responds to a
// GetServerInfo RPC.
func databrokerHealthyTCP() bool {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx,
		"grpcurl", "-plaintext",
		"localhost:55556",
		"kuksa.val.v1.VAL/GetServerInfo",
	)
	return cmd.Run() == nil
}

// ---- gRPC helpers (grpcurl) ---------------------------------------------------

const (
	tcpEndpoint = "localhost:55556"
	udsEndpoint = "unix:///tmp/kuksa/kuksa-databroker.sock"
)

// grpcCall runs a grpcurl command against endpoint and returns combined output
// and exit error. body may be empty ("") for RPCs with no request body.
func grpcCall(endpoint, method, body string) (string, error) {
	args := []string{"-plaintext"}
	if body != "" {
		args = append(args, "-d", body)
	}
	args = append(args, endpoint, method)
	out, err := exec.Command("grpcurl", args...).CombinedOutput()
	return string(out), err
}

// grpcGetMetadata fetches signal metadata via VAL/Get with FIELD_METADATA.
// The kuksa.val.v1.VAL service has no separate GetMetadata RPC; metadata is
// retrieved via Get with fields=["FIELD_METADATA"].
func grpcGetMetadata(endpoint, path string) (string, error) {
	body := fmt.Sprintf(`{"entries": [{"path": %q, "fields": ["FIELD_METADATA"]}]}`, path)
	return grpcCall(endpoint, "kuksa.val.v1.VAL/Get", body)
}

// grpcGet fetches the current value of a signal via VAL/Get with FIELD_VALUE.
func grpcGet(endpoint, path string) (string, error) {
	body := fmt.Sprintf(`{"entries": [{"path": %q, "fields": ["FIELD_VALUE"]}]}`, path)
	return grpcCall(endpoint, "kuksa.val.v1.VAL/Get", body)
}

// grpcSetBool sets a boolean signal value via VAL/Set.
// The EntryUpdate must include fields=["FIELD_VALUE"] to specify what to write.
func grpcSetBool(endpoint, path string, value bool) (string, error) {
	v := "false"
	if value {
		v = "true"
	}
	body := fmt.Sprintf(
		`{"updates": [{"entry": {"path": %q, "value": {"bool": %s}}, "fields": ["FIELD_VALUE"]}]}`,
		path, v,
	)
	return grpcCall(endpoint, "kuksa.val.v1.VAL/Set", body)
}

// grpcSetFloat sets a float signal value via VAL/Set.
func grpcSetFloat(endpoint, path, value string) (string, error) {
	body := fmt.Sprintf(
		`{"updates": [{"entry": {"path": %q, "value": {"float": %s}}, "fields": ["FIELD_VALUE"]}]}`,
		path, value,
	)
	return grpcCall(endpoint, "kuksa.val.v1.VAL/Set", body)
}

// grpcSetString sets a string signal value via VAL/Set.
func grpcSetString(endpoint, path, value string) (string, error) {
	// JSON-encode the value string using fmt.Sprintf %q (adds quotes and escapes).
	body := fmt.Sprintf(
		`{"updates": [{"entry": {"path": %q, "value": {"string": %q}}, "fields": ["FIELD_VALUE"]}]}`,
		path, value,
	)
	return grpcCall(endpoint, "kuksa.val.v1.VAL/Set", body)
}

// grpcSubscribeCapture starts a Subscribe stream for signalPath, runs action(),
// and returns what the stream emitted within timeout. The stream is always
// cancelled before returning.
//
// Subscribe request format: {"entries": [{"path": "...", "fields": ["FIELD_VALUE"]}]}
func grpcSubscribeCapture(t *testing.T, endpoint, signalPath string, timeout time.Duration, action func()) string {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	body := fmt.Sprintf(`{"entries": [{"path": %q, "fields": ["FIELD_VALUE"]}]}`, signalPath)
	cmd := exec.CommandContext(ctx,
		"grpcurl", "-plaintext",
		"-d", body,
		endpoint,
		"kuksa.val.v1.VAL/Subscribe",
	)
	var buf bytes.Buffer
	cmd.Stdout = &buf
	cmd.Stderr = &buf

	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start Subscribe stream: %v", err)
	}

	// Allow the subscription to be established before triggering the action.
	time.Sleep(300 * time.Millisecond)

	action()

	// Let the notification arrive then cancel.
	time.Sleep(1 * time.Second)
	cancel()
	_ = cmd.Wait()

	return buf.String()
}

// ---- compose.yml parser (text-based) ------------------------------------------

// readCompose reads deployments/compose.yml as a string.
func readCompose(t *testing.T) string {
	t.Helper()
	path := filepath.Join(repoRoot(t), "deployments", "compose.yml")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read deployments/compose.yml: %v", err)
	}
	return string(data)
}

// hasNotFoundInBody returns true if the JSON response body contains a not_found
// indicator. grpcurl exits 0 even for NOT_FOUND responses — the error is in the
// JSON body.
func hasNotFoundInBody(body string) bool {
	lower := strings.ToLower(body)
	return strings.Contains(lower, "not_found") ||
		strings.Contains(lower, `"code": 404`) ||
		strings.Contains(lower, `"code":404`)
}
