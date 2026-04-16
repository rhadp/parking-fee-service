// Integration tests for parking-app-cli.
//
// RED PHASE: the stub binary always exits 0 without making any REST or gRPC
// calls, so every assertion about output content and exit codes will FAIL
// until task group 4 implements the real CLI.
package integration

import (
	"os/exec"
	"strings"
	"testing"
)

const parkingAppPkg = "github.com/sdv-demo/mock/parking-app-cli"

// ── TS-09-5: parking-app-cli lookup ──────────────────────────────────────

// TestLookup verifies that `parking-app-cli lookup` sends GET /operators?lat=&lon=
// to PARKING_FEE_SERVICE and prints the JSON response.
func TestLookup(t *testing.T) {
	ms := startMockHTTPServer(t, 200, `[{"operator_id":"op-1","name":"Demo Parking"}]`)
	binary := buildBinary(t, parkingAppPkg)

	cmd := exec.Command(binary,
		"lookup",
		"--lat=48.1351",
		"--lon=11.5820",
		"--service-addr="+ms.URL,
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected exit 0, got: %v\noutput: %s", err, out)
	}
	if !strings.Contains(string(out), "op-1") {
		t.Fatalf("expected 'op-1' in stdout, got: %s", out)
	}

	req := ms.lastRequest()
	if req == nil {
		t.Fatal("no HTTP request captured")
	}
	if req.Method != "GET" {
		t.Fatalf("expected GET, got %s", req.Method)
	}
	if !strings.Contains(req.URL, "lat=48.1351") || !strings.Contains(req.URL, "lon=11.5820") {
		t.Fatalf("expected lat/lon query params, got: %s", req.URL)
	}
}

// TestLookupMissingArgs verifies that lookup without required flags exits 1.
func TestLookupMissingArgs(t *testing.T) {
	binary := buildBinary(t, parkingAppPkg)

	// Missing --lon
	cmd := exec.Command(binary, "lookup", "--lat=48.1351")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatalf("expected exit 1 for missing --lon, got 0\noutput: %s", out)
	}
}

// ── TS-09-6: parking-app-cli adapter-info ────────────────────────────────

// TestAdapterInfo verifies that `parking-app-cli adapter-info` sends
// GET /operators/{id}/adapter to PARKING_FEE_SERVICE.
func TestAdapterInfo(t *testing.T) {
	ms := startMockHTTPServer(t, 200, `{"image_ref":"registry/adapter:v1","checksum":"sha256:abc"}`)
	binary := buildBinary(t, parkingAppPkg)

	cmd := exec.Command(binary,
		"adapter-info",
		"--operator-id=op-1",
		"--service-addr="+ms.URL,
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected exit 0, got: %v\noutput: %s", err, out)
	}
	if !strings.Contains(string(out), "image_ref") {
		t.Fatalf("expected 'image_ref' in stdout, got: %s", out)
	}

	req := ms.lastRequest()
	if req == nil {
		t.Fatal("no HTTP request captured")
	}
	if !strings.Contains(req.URL, "/operators/op-1/adapter") {
		t.Fatalf("expected URL containing /operators/op-1/adapter, got: %s", req.URL)
	}
}

// ── TS-09-7: parking-app-cli install ─────────────────────────────────────

// TestInstall verifies that `parking-app-cli install` calls InstallAdapter on
// UPDATE_SERVICE and prints the response including job_id.
//
// The TCP listener simulates a gRPC endpoint; with the stub binary the test
// fails because no output is produced.
func TestInstall(t *testing.T) {
	addr := startTCPListener(t)
	binary := buildBinary(t, parkingAppPkg)

	cmd := exec.Command(binary,
		"install",
		"--image-ref=registry/adapter:v1",
		"--checksum=sha256:abc",
		"--update-addr="+addr,
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected exit 0, got: %v\noutput: %s", err, out)
	}
	if !strings.Contains(string(out), "job_id") {
		t.Fatalf("expected 'job_id' in stdout, got: %s", out)
	}
}

// TestInstallMissingArgs verifies that install without required flags exits 1.
func TestInstallMissingArgs(t *testing.T) {
	binary := buildBinary(t, parkingAppPkg)

	// Missing --checksum
	cmd := exec.Command(binary, "install", "--image-ref=registry/adapter:v1")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatalf("expected exit 1 for missing --checksum, got 0\noutput: %s", out)
	}
}

// TestInstallGRPCError verifies that install exits 1 when UPDATE_SERVICE is
// unreachable, and prints the error to stderr.
func TestInstallGRPCError(t *testing.T) {
	binary := buildBinary(t, parkingAppPkg)

	cmd := exec.Command(binary,
		"install",
		"--image-ref=x",
		"--checksum=y",
		"--update-addr=localhost:19999",
	)
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatalf("expected exit 1 for unreachable gRPC, got 0\noutput: %s", out)
	}
	if len(out) == 0 {
		t.Fatal("expected error message on stderr")
	}
}

// ── TS-09-8: parking-app-cli list ────────────────────────────────────────

// TestList verifies that `parking-app-cli list` calls ListAdapters on
// UPDATE_SERVICE and prints the adapter list.
func TestList(t *testing.T) {
	addr := startTCPListener(t)
	binary := buildBinary(t, parkingAppPkg)

	cmd := exec.Command(binary, "list", "--update-addr="+addr)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected exit 0, got: %v\noutput: %s", err, out)
	}
	if !strings.Contains(string(out), "adapter") {
		t.Fatalf("expected adapter info in stdout, got: %s", out)
	}
}

// ── TS-09-E11: parking-app-cli lookup — non-2xx response ─────────────────

// TestLookupHTTPError verifies that lookup exits 1 when PARKING_FEE_SERVICE
// returns a non-2xx response and prints the status to stderr.
func TestLookupHTTPError(t *testing.T) {
	ms := startMockHTTPServer(t, 500, `{"error":"internal server error"}`)
	binary := buildBinary(t, parkingAppPkg)

	cmd := exec.Command(binary,
		"lookup",
		"--lat=0",
		"--lon=0",
		"--service-addr="+ms.URL,
	)
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatalf("expected exit 1 for 500 response, got 0\noutput: %s", out)
	}
	if !strings.Contains(string(out), "500") {
		t.Fatalf("expected HTTP status 500 in output, got: %s", out)
	}
}

// ── TS-09-9: parking-app-cli start-session ────────────────────────────────

// TestStartSessionCLI verifies that `parking-app-cli start-session` calls
// StartSession on PARKING_OPERATOR_ADAPTOR and prints the response.
func TestStartSessionCLI(t *testing.T) {
	addr := startTCPListener(t)
	binary := buildBinary(t, parkingAppPkg)

	cmd := exec.Command(binary,
		"start-session",
		"--zone-id=zone-demo-1",
		"--adaptor-addr="+addr,
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected exit 0, got: %v\noutput: %s", err, out)
	}
	if !strings.Contains(string(out), "session") {
		t.Fatalf("expected session info in stdout, got: %s", out)
	}
}

// ── TS-09-10: parking-app-cli stop-session ────────────────────────────────

// TestStopSessionCLI verifies that `parking-app-cli stop-session` calls
// StopSession on PARKING_OPERATOR_ADAPTOR and prints the response.
func TestStopSessionCLI(t *testing.T) {
	addr := startTCPListener(t)
	binary := buildBinary(t, parkingAppPkg)

	cmd := exec.Command(binary, "stop-session", "--adaptor-addr="+addr)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("expected exit 0, got: %v\noutput: %s", err, out)
	}
	if !strings.Contains(string(out), "stopped") {
		t.Fatalf("expected 'stopped' in stdout, got: %s", out)
	}
}

// TestSessionGRPCError verifies that session subcommands exit 1 when
// PARKING_OPERATOR_ADAPTOR is unreachable.
func TestSessionGRPCError(t *testing.T) {
	binary := buildBinary(t, parkingAppPkg)

	cmd := exec.Command(binary, "start-session", "--zone-id=z", "--adaptor-addr=localhost:19999")
	out, err := cmd.CombinedOutput()
	if err == nil {
		t.Fatalf("expected exit 1 for unreachable adaptor, got 0\noutput: %s", out)
	}
	if len(out) == 0 {
		t.Fatal("expected error message on stderr")
	}
}
