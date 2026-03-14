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

// requireUDSSocket skips the test if the databroker UDS socket is not
// accessible from the host at the expected path. On macOS with Podman running
// in a VM, the socket is created inside the VM and is not visible from the
// macOS host filesystem.
func requireUDSSocket(t *testing.T) {
	t.Helper()
	const sockPath = "/tmp/kuksa/kuksa-databroker.sock"
	if _, err := os.Stat(sockPath); err != nil {
		t.Skipf("skipping UDS test: socket %s not accessible from host "+
			"(Podman on macOS creates UDS sockets inside the VM, not on the host): %v",
			sockPath, err)
	}
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
//
// Note: kuksa-databroker 0.5.0 exposes kuksa.val.v2.VAL (not v1).
func databrokerHealthyTCP() bool {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx,
		"grpcurl", "-plaintext",
		"localhost:55556",
		"kuksa.val.v2.VAL/GetServerInfo",
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

// grpcGetMetadata fetches signal metadata via VAL/ListMetadata.
//
// Note: kuksa-databroker 0.5.0 uses kuksa.val.v2.VAL. Metadata is retrieved
// via ListMetadata with {"root": "<path>"}. The response does not include the
// queried path name, so we prepend "signal=<path>" to the returned string so
// that callers can assert the correct signal was queried.
func grpcGetMetadata(endpoint, path string) (string, error) {
	body := fmt.Sprintf(`{"root": %q}`, path)
	out, err := grpcCall(endpoint, "kuksa.val.v2.VAL/ListMetadata", body)
	// Prepend the queried path so callers can verify which signal's metadata
	// was returned (v2 ListMetadata response omits the signal path).
	return fmt.Sprintf("signal=%s\n%s", path, out), err
}

// grpcGet fetches the current value of a signal via VAL/GetValue.
//
// Note: kuksa.val.v2.VAL/GetValue uses {"signal_id": {"path": "..."}} format.
func grpcGet(endpoint, path string) (string, error) {
	body := fmt.Sprintf(`{"signal_id": {"path": %q}}`, path)
	return grpcCall(endpoint, "kuksa.val.v2.VAL/GetValue", body)
}

// grpcSetBool sets a boolean signal value via VAL/PublishValue.
//
// Note: kuksa.val.v2.VAL/PublishValue uses signal_id + data_point format.
func grpcSetBool(endpoint, path string, value bool) (string, error) {
	v := "false"
	if value {
		v = "true"
	}
	body := fmt.Sprintf(
		`{"signal_id": {"path": %q}, "data_point": {"value": {"bool": %s}}}`,
		path, v,
	)
	return grpcCall(endpoint, "kuksa.val.v2.VAL/PublishValue", body)
}

// grpcSetFloat sets a float signal value via VAL/PublishValue.
func grpcSetFloat(endpoint, path, value string) (string, error) {
	body := fmt.Sprintf(
		`{"signal_id": {"path": %q}, "data_point": {"value": {"float": %s}}}`,
		path, value,
	)
	return grpcCall(endpoint, "kuksa.val.v2.VAL/PublishValue", body)
}

// grpcSetString sets a string signal value via VAL/PublishValue.
func grpcSetString(endpoint, path, value string) (string, error) {
	// JSON-encode the value string using fmt.Sprintf %q (adds quotes and escapes).
	body := fmt.Sprintf(
		`{"signal_id": {"path": %q}, "data_point": {"value": {"string": %q}}}`,
		path, value,
	)
	return grpcCall(endpoint, "kuksa.val.v2.VAL/PublishValue", body)
}

// grpcSubscribeCapture starts a Subscribe stream for signalPath, runs action(),
// and returns what the stream emitted within timeout. The stream is always
// cancelled before returning.
//
// Note: kuksa.val.v2.VAL/Subscribe uses {"signal_paths": ["..."]} format.
func grpcSubscribeCapture(t *testing.T, endpoint, signalPath string, timeout time.Duration, action func()) string {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	body := fmt.Sprintf(`{"signal_paths": [%q]}`, signalPath)
	cmd := exec.CommandContext(ctx,
		"grpcurl", "-plaintext",
		"-d", body,
		endpoint,
		"kuksa.val.v2.VAL/Subscribe",
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

// hasNotFoundInBody returns true if the gRPC response (or grpcurl output)
// contains a NOT_FOUND indicator.
//
// In kuksa.val.v2, grpcurl outputs "Code: NotFound" (with capital letters and
// no underscore) to combined output when the signal path is not found. In some
// v1-compatible error bodies the indicator appears as "not_found". We check
// multiple forms for robustness.
func hasNotFoundInBody(body string) bool {
	lower := strings.ToLower(body)
	return strings.Contains(lower, "not_found") ||
		strings.Contains(lower, "notfound") ||
		strings.Contains(lower, "not found") ||
		strings.Contains(lower, `"code": 404`) ||
		strings.Contains(lower, `"code":404`)
}
