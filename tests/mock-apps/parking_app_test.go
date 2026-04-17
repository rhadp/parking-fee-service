package mock_apps

import (
	"strings"
	"testing"
)

// TS-09-5: parking-app-cli lookup sends GET /operators?lat=...&lon=... to PARKING_FEE_SERVICE.
func TestLookup(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")

	serverURL, cap := startMockHTTPServerWithCapture(t,
		[]map[string]string{{"operator_id": "op-1", "name": "Demo"}}, 200)

	stdout, _, exitCode := runCmd(t, binary,
		[]string{"lookup", "--lat=48.1351", "--lon=11.5820", "--service-addr=" + serverURL},
		nil,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if !strings.Contains(cap.Path, "lat=48.1351") {
		t.Errorf("expected lat=48.1351 in request path %q", cap.Path)
	}
	if !strings.Contains(cap.Path, "lon=11.5820") {
		t.Errorf("expected lon=11.5820 in request path %q", cap.Path)
	}
	if !strings.Contains(stdout, "op-1") {
		t.Errorf("expected 'op-1' in stdout, got %q", stdout)
	}
}

// TS-09-6: parking-app-cli adapter-info sends GET /operators/{id}/adapter to PARKING_FEE_SERVICE.
func TestAdapterInfo(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")

	serverURL, cap := startMockHTTPServerWithCapture(t,
		map[string]string{"image_ref": "registry/adapter:v1", "checksum": "sha256:abc"}, 200)

	stdout, _, exitCode := runCmd(t, binary,
		[]string{"adapter-info", "--operator-id=op-1", "--service-addr=" + serverURL},
		nil,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if !strings.Contains(cap.Path, "op-1") {
		t.Errorf("expected 'op-1' in request path %q", cap.Path)
	}
	if !strings.Contains(stdout, "image_ref") {
		t.Errorf("expected 'image_ref' in stdout, got %q", stdout)
	}
}

// TS-09-7: parking-app-cli install calls InstallAdapter RPC on UPDATE_SERVICE.
// The mock gRPC server is a placeholder (TCP listener); the real gRPC mock is added in task group 4.
func TestInstall(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")

	// Placeholder gRPC server — binary will fail to connect.
	// In task group 4, this will be replaced with a real proto-based gRPC mock.
	grpcAddr := startMockTCPListener(t)

	stdout, _, exitCode := runCmd(t, binary,
		[]string{"install", "--image-ref=registry/adapter:v1", "--checksum=sha256:abc",
			"--update-addr=" + grpcAddr},
		nil,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if !strings.Contains(stdout, "job_id") && !strings.Contains(stdout, "adapter_id") {
		t.Errorf("expected job_id or adapter_id in stdout, got %q", stdout)
	}
}

// TS-09-8: parking-app-cli list calls ListAdapters RPC on UPDATE_SERVICE.
func TestList(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")
	grpcAddr := startMockTCPListener(t)

	stdout, _, exitCode := runCmd(t, binary,
		[]string{"list", "--update-addr=" + grpcAddr},
		nil,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	_ = stdout // output format depends on implementation
}

// TS-09-9: parking-app-cli start-session calls StartSession RPC on PARKING_OPERATOR_ADAPTOR.
func TestStartSession(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")
	grpcAddr := startMockTCPListener(t)

	stdout, _, exitCode := runCmd(t, binary,
		[]string{"start-session", "--zone-id=zone-demo-1", "--adaptor-addr=" + grpcAddr},
		nil,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	_ = stdout
}

// TS-09-10: parking-app-cli stop-session calls StopSession RPC on PARKING_OPERATOR_ADAPTOR.
func TestStopSession(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")
	grpcAddr := startMockTCPListener(t)

	stdout, _, exitCode := runCmd(t, binary,
		[]string{"stop-session", "--adaptor-addr=" + grpcAddr},
		nil,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	_ = stdout
}

// TS-09-E10: parking-app-cli install exits 1 when UPDATE_SERVICE is unreachable.
func TestInstallGRPCError(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")

	_, stderr, exitCode := runCmd(t, binary,
		[]string{"install", "--image-ref=x", "--checksum=y", "--update-addr=localhost:19999"},
		nil,
	)

	if exitCode != 1 {
		t.Errorf("expected exit code 1 when gRPC server unreachable, got %d", exitCode)
	}
	if len(stderr) == 0 {
		t.Error("expected non-empty stderr on gRPC connection error")
	}
}

// TS-09-E (missing args): parking-app-cli install exits 1 when --image-ref is missing.
func TestInstallMissingArgs(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")

	_, stderr, exitCode := runCmd(t, binary,
		[]string{"install"},
		nil,
	)

	if exitCode != 1 {
		t.Errorf("expected exit code 1 when args missing, got %d", exitCode)
	}
	if len(stderr) == 0 {
		t.Error("expected non-empty stderr when required args are missing")
	}
}

// TS-09-E (lookup missing args): parking-app-cli lookup exits 1 when --lat or --lon is missing.
func TestLookupMissingArgs(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")

	_, stderr, exitCode := runCmd(t, binary,
		[]string{"lookup", "--lat=48.13"}, // missing --lon
		nil,
	)

	if exitCode != 1 {
		t.Errorf("expected exit code 1 when --lon is missing, got %d", exitCode)
	}
	if len(stderr) == 0 {
		t.Error("expected non-empty stderr when required args are missing")
	}
}

// TS-09-E11: parking-app-cli lookup exits 1 on non-2xx response from PARKING_FEE_SERVICE.
func TestLookupHTTPError(t *testing.T) {
	binary := findBinary(t, "parking-app-cli")

	serverURL, _ := startMockHTTPServerWithCapture(t, "internal error", 500)

	_, stderr, exitCode := runCmd(t, binary,
		[]string{"lookup", "--lat=0", "--lon=0", "--service-addr=" + serverURL},
		nil,
	)

	if exitCode != 1 {
		t.Errorf("expected exit code 1 on HTTP 500, got %d", exitCode)
	}
	if !strings.Contains(stderr, "500") {
		t.Errorf("expected '500' in stderr, got %q", stderr)
	}
}
