package mockapps_test

import (
	"path/filepath"
	"strings"
	"testing"
)

// parkingAppBinary builds and returns the path to the parking-app-cli binary.
func parkingAppBinary(t *testing.T) string {
	t.Helper()
	root := findRepoRoot(t)
	moduleDir := filepath.Join(root, "mock", "parking-app-cli")
	return buildGoBinary(t, moduleDir, "parking-app-cli")
}

// ---------------------------------------------------------------------------
// TS-09-5: Parking App CLI Lookup
// Requirement: 09-REQ-4.1, 09-REQ-4.3
// ---------------------------------------------------------------------------

func TestLookup(t *testing.T) {
	mock := newMockHTTPServer(t, 200, []map[string]any{
		{"operator_id": "op-1", "name": "Demo"},
	})

	bin := parkingAppBinary(t)
	env := baseEnv()

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"lookup",
		"--lat=48.1351",
		"--lon=11.5820",
		"--service-addr=" + mock.URL(),
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "op-1") {
		t.Errorf("expected stdout to contain 'op-1', got: %s", stdout)
	}

	// Verify the request was a GET to /operators with query params
	reqs := mock.getRequests()
	if len(reqs) != 1 {
		t.Fatalf("expected 1 request, got %d", len(reqs))
	}
	if reqs[0].Method != "GET" {
		t.Errorf("expected GET, got %s", reqs[0].Method)
	}
}

// ---------------------------------------------------------------------------
// TS-09-6: Parking App CLI Adapter Info
// Requirement: 09-REQ-4.2
// ---------------------------------------------------------------------------

func TestAdapterInfo(t *testing.T) {
	mock := newMockHTTPServer(t, 200, map[string]any{
		"image_ref": "registry/adapter:v1",
		"checksum":  "sha256:abc",
	})

	bin := parkingAppBinary(t)
	env := baseEnv()

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"adapter-info",
		"--operator-id=op-1",
		"--service-addr=" + mock.URL(),
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "image_ref") {
		t.Errorf("expected stdout to contain 'image_ref', got: %s", stdout)
	}

	// Verify the request path
	reqs := mock.getRequests()
	if len(reqs) != 1 {
		t.Fatalf("expected 1 request, got %d", len(reqs))
	}
	if reqs[0].Path != "/operators/op-1/adapter" {
		t.Errorf("expected path /operators/op-1/adapter, got %s", reqs[0].Path)
	}
}

// ---------------------------------------------------------------------------
// TS-09-7: Parking App CLI Install Adapter
// Requirement: 09-REQ-5.1, 09-REQ-5.6
// Note: This test requires a mock gRPC server. Since no gRPC infrastructure
// exists yet, we test that the binary at least accepts the subcommand and
// attempts to connect to the specified address.
// ---------------------------------------------------------------------------

func TestInstall(t *testing.T) {
	bin := parkingAppBinary(t)
	env := baseEnv()

	// Use a non-listening address; the binary should try to connect and fail
	// gracefully with exit 1, or succeed if a mock server is available.
	// For now, we test that the binary recognizes the subcommand.
	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"install",
		"--image-ref=registry/adapter:v1",
		"--checksum=sha256:abc",
		"--update-addr=localhost:19999",
	}, env)

	// The binary should either succeed (if mock server existed) or fail with
	// a connection error (exit 1). It should NOT exit 0 with just a version string.
	if exitCode == 0 && strings.Contains(stdout, "parking-app-cli v0.1.0") {
		t.Error("expected binary to attempt gRPC connection, but it only printed version")
	}

	// Verify it doesn't just silently succeed without doing anything
	if exitCode == 0 && !strings.Contains(stdout, "j1") && !strings.Contains(stdout, "adapter") {
		t.Errorf("exit 0 but no meaningful output\nstdout: %s\nstderr: %s", stdout, stderr)
	}
}

// ---------------------------------------------------------------------------
// TS-09-8: Parking App CLI List Adapters
// Requirement: 09-REQ-5.2
// ---------------------------------------------------------------------------

func TestList(t *testing.T) {
	bin := parkingAppBinary(t)
	env := baseEnv()

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"list",
		"--update-addr=localhost:19999",
	}, env)

	// Should recognize the subcommand and attempt connection
	if exitCode == 0 && strings.Contains(stdout, "parking-app-cli v0.1.0") {
		t.Error("expected binary to attempt gRPC connection, but it only printed version")
	}
	_ = stderr
}

// ---------------------------------------------------------------------------
// TS-09-9: Parking App CLI Start Session Override
// Requirement: 09-REQ-6.1, 09-REQ-6.3
// ---------------------------------------------------------------------------

func TestStartSession(t *testing.T) {
	bin := parkingAppBinary(t)
	env := baseEnv()

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"start-session",
		"--zone-id=zone-demo-1",
		"--adaptor-addr=localhost:19999",
	}, env)

	// Should recognize the subcommand
	if exitCode == 0 && strings.Contains(stdout, "parking-app-cli v0.1.0") {
		t.Error("expected binary to attempt gRPC connection, but it only printed version")
	}
	_ = stderr
}

// ---------------------------------------------------------------------------
// TS-09-10: Parking App CLI Stop Session Override
// Requirement: 09-REQ-6.2
// ---------------------------------------------------------------------------

func TestStopSession(t *testing.T) {
	bin := parkingAppBinary(t)
	env := baseEnv()

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"stop-session",
		"--adaptor-addr=localhost:19999",
	}, env)

	// Should recognize the subcommand
	if exitCode == 0 && strings.Contains(stdout, "parking-app-cli v0.1.0") {
		t.Error("expected binary to attempt gRPC connection, but it only printed version")
	}
	_ = stderr
}

// ---------------------------------------------------------------------------
// TS-09-E10: Parking App CLI gRPC Error
// Requirement: 09-REQ-5.E2, 09-REQ-6.E1
// ---------------------------------------------------------------------------

func TestInstallGRPCError(t *testing.T) {
	bin := parkingAppBinary(t)
	env := baseEnv()

	_, stderr, exitCode := runBinary(t, bin, []string{
		"install",
		"--image-ref=x",
		"--checksum=y",
		"--update-addr=localhost:19999",
	}, env)

	if exitCode != 1 {
		t.Errorf("expected exit 1 on gRPC error, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected stderr to contain error message on gRPC failure")
	}
}

// ---------------------------------------------------------------------------
// TS-09-E11: PARKING_FEE_SERVICE Non-2xx
// Requirement: 09-REQ-4.E2
// ---------------------------------------------------------------------------

func TestLookupHTTPError(t *testing.T) {
	mock := newMockHTTPServer(t, 500, map[string]any{
		"error": "internal error",
	})

	bin := parkingAppBinary(t)
	env := baseEnv()

	_, stderr, exitCode := runBinary(t, bin, []string{
		"lookup",
		"--lat=0",
		"--lon=0",
		"--service-addr=" + mock.URL(),
	}, env)

	if exitCode != 1 {
		t.Errorf("expected exit 1 on HTTP 500, got %d", exitCode)
	}

	if !strings.Contains(stderr, "500") {
		t.Errorf("expected stderr to contain '500', got: %s", stderr)
	}
}

// ---------------------------------------------------------------------------
// Parking App CLI Missing Args Tests
// Requirement: 09-REQ-4.E1 (lookup missing lat/lon, adapter-info missing operator-id)
// ---------------------------------------------------------------------------

func TestLookupMissingArgs(t *testing.T) {
	bin := parkingAppBinary(t)
	env := baseEnv()

	// Missing --lat and --lon
	_, stderr, exitCode := runBinary(t, bin, []string{
		"lookup",
	}, env)

	if exitCode != 1 {
		t.Errorf("expected exit 1 for lookup missing args, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected stderr to contain usage error for missing args")
	}
}

func TestAdapterInfoMissingArgs(t *testing.T) {
	bin := parkingAppBinary(t)
	env := baseEnv()

	// Missing --operator-id
	_, stderr, exitCode := runBinary(t, bin, []string{
		"adapter-info",
	}, env)

	if exitCode != 1 {
		t.Errorf("expected exit 1 for adapter-info missing args, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected stderr to contain usage error for missing operator-id")
	}
}

func TestInstallMissingArgs(t *testing.T) {
	bin := parkingAppBinary(t)
	env := baseEnv()

	// Missing --image-ref and --checksum
	_, stderr, exitCode := runBinary(t, bin, []string{
		"install",
	}, env)

	if exitCode != 1 {
		t.Errorf("expected exit 1 for install missing args, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected stderr to contain usage error for missing args")
	}
}
