package mockapps_test

import (
	"encoding/json"
	"net"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// TS-09-5: Parking App CLI Lookup
// Requirement: 09-REQ-4.1, 09-REQ-4.3
// ---------------------------------------------------------------------------

func TestLookup(t *testing.T) {
	var receivedPath string
	var receivedQuery string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedPath = r.URL.Path
		receivedQuery = r.URL.RawQuery
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode([]map[string]string{
			{"operator_id": "op-1", "name": "Demo"},
		})
	}))
	defer srv.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"lookup",
		"--lat=48.1351",
		"--lon=11.5820",
		"--service-addr="+srv.URL,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "op-1") {
		t.Errorf("expected stdout to contain 'op-1', got: %s", stdout)
	}

	if receivedPath != "/operators" {
		t.Errorf("expected path '/operators', got: %s", receivedPath)
	}

	if !strings.Contains(receivedQuery, "lat=48.1351") || !strings.Contains(receivedQuery, "lon=11.5820") {
		t.Errorf("expected query to contain lat and lon, got: %s", receivedQuery)
	}
}

// ---------------------------------------------------------------------------
// TS-09-6: Parking App CLI Adapter Info
// Requirement: 09-REQ-4.2
// ---------------------------------------------------------------------------

func TestAdapterInfo(t *testing.T) {
	var receivedPath string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedPath = r.URL.Path
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{
			"image_ref": "registry/adapter:v1",
			"checksum":  "sha256:abc",
		})
	}))
	defer srv.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"adapter-info",
		"--operator-id=op-1",
		"--service-addr="+srv.URL,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "image_ref") {
		t.Errorf("expected stdout to contain 'image_ref', got: %s", stdout)
	}

	if receivedPath != "/operators/op-1/adapter" {
		t.Errorf("expected path '/operators/op-1/adapter', got: %s", receivedPath)
	}
}

// ---------------------------------------------------------------------------
// TS-09-7: Parking App CLI Install Adapter
// Requirement: 09-REQ-5.1, 09-REQ-5.6
// ---------------------------------------------------------------------------

func TestInstall(t *testing.T) {
	// Start a TCP listener to act as a mock gRPC endpoint.
	// In task group 4, this will be upgraded to a proper gRPC mock server
	// with proto-generated types returning InstallAdapterResponse.
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}
	defer lis.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"install",
		"--image-ref=registry/adapter:v1",
		"--checksum=sha256:abc",
		"--update-addr="+lis.Addr().String(),
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	// Verify the response contains the expected fields from the mock server.
	if !strings.Contains(stdout, "j1") {
		t.Errorf("expected stdout to contain job_id 'j1', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-8: Parking App CLI List Adapters
// Requirement: 09-REQ-5.2
// ---------------------------------------------------------------------------

func TestList(t *testing.T) {
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}
	defer lis.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"list",
		"--update-addr="+lis.Addr().String(),
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected stdout to contain adapter_id 'a1', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-18: Parking App CLI Get Adapter Status
// Requirement: 09-REQ-5.4
// ---------------------------------------------------------------------------

func TestAdapterStatus(t *testing.T) {
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}
	defer lis.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"status",
		"--adapter-id=a1",
		"--update-addr="+lis.Addr().String(),
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "a1") {
		t.Errorf("expected stdout to contain adapter_id 'a1', got: %s", stdout)
	}

	if !strings.Contains(stdout, "RUNNING") {
		t.Errorf("expected stdout to contain 'RUNNING', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-19: Parking App CLI Remove Adapter
// Requirement: 09-REQ-5.5
// ---------------------------------------------------------------------------

func TestRemove(t *testing.T) {
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}
	defer lis.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"remove",
		"--adapter-id=a1",
		"--update-addr="+lis.Addr().String(),
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	// Verify the CLI actually called RemoveAdapter and produced output.
	if len(stdout) == 0 && len(stderr) == 0 {
		t.Error("expected some output confirming adapter removal")
	}
}

// ---------------------------------------------------------------------------
// TS-09-9: Parking App CLI Start Session Override
// Requirement: 09-REQ-6.1, 09-REQ-6.3
// ---------------------------------------------------------------------------

func TestStartSession(t *testing.T) {
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}
	defer lis.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"start-session",
		"--zone-id=zone-demo-1",
		"--adaptor-addr="+lis.Addr().String(),
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "s1") {
		t.Errorf("expected stdout to contain session_id 's1', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-10: Parking App CLI Stop Session Override
// Requirement: 09-REQ-6.2
// ---------------------------------------------------------------------------

func TestStopSession(t *testing.T) {
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}
	defer lis.Close()

	binary := buildBinary(t, "parking-app-cli")
	stdout, stderr, exitCode := runBinary(t, binary,
		"stop-session",
		"--adaptor-addr="+lis.Addr().String(),
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstderr: %s", exitCode, stderr)
	}

	if !strings.Contains(stdout, "stopped") {
		t.Errorf("expected stdout to contain 'stopped', got: %s", stdout)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E10: Parking App CLI gRPC Error
// Requirement: 09-REQ-5.E2, 09-REQ-6.E1
// ---------------------------------------------------------------------------

func TestInstallGRPCError(t *testing.T) {
	binary := buildBinary(t, "parking-app-cli")

	// Connect to a port where no service is running.
	_, stderr, exitCode := runBinary(t, binary,
		"install",
		"--image-ref=x",
		"--checksum=y",
		"--update-addr=localhost:19999",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 on gRPC error, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected error message on stderr when gRPC call fails")
	}
}

func TestSessionGRPCError(t *testing.T) {
	binary := buildBinary(t, "parking-app-cli")

	_, stderr, exitCode := runBinary(t, binary,
		"start-session",
		"--zone-id=zone-1",
		"--adaptor-addr=localhost:19999",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 on gRPC error, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected error message on stderr when gRPC call fails")
	}
}

// ---------------------------------------------------------------------------
// TS-09-E11: PARKING_FEE_SERVICE Non-2xx
// Requirement: 09-REQ-4.E2
// ---------------------------------------------------------------------------

func TestLookupHTTPError(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
		w.Write([]byte("internal error"))
	}))
	defer srv.Close()

	binary := buildBinary(t, "parking-app-cli")
	_, stderr, exitCode := runBinary(t, binary,
		"lookup",
		"--lat=0",
		"--lon=0",
		"--service-addr="+srv.URL,
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 on non-2xx response, got %d", exitCode)
	}

	if !strings.Contains(stderr, "500") {
		t.Errorf("expected stderr to contain '500', got: %s", stderr)
	}
}

// ---------------------------------------------------------------------------
// Parking App CLI Missing Args (09-REQ-4.E1)
// Addresses Skeptic finding: 09-REQ-4.E1 was untested.
// ---------------------------------------------------------------------------

func TestLookupMissingArgs(t *testing.T) {
	binary := buildBinary(t, "parking-app-cli")

	// Lookup requires --lat and --lon. Omit --lon.
	_, stderr, exitCode := runBinary(t, binary,
		"lookup",
		"--lat=48.1351",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 when required flags missing, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected usage error on stderr when required flags missing")
	}
}

func TestInstallMissingArgs(t *testing.T) {
	binary := buildBinary(t, "parking-app-cli")

	// Install requires --image-ref and --checksum. Omit both.
	_, stderr, exitCode := runBinary(t, binary,
		"install",
	)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1 when required flags missing, got %d", exitCode)
	}

	if len(stderr) == 0 {
		t.Error("expected usage error on stderr when required flags missing")
	}
}
