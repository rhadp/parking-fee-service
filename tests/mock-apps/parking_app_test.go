package mockappstests

import (
	"strings"
	"testing"
)

// parkingAppBin builds and returns the parking-app-cli binary path.
var parkingAppBinCache string

func parkingAppBin(t *testing.T) string {
	t.Helper()
	if parkingAppBinCache == "" {
		parkingAppBinCache = buildBinary(t, "parking-app-cli")
	}
	return parkingAppBinCache
}

// TS-09-5: parking-app-cli lookup sends GET /operators?lat=...&lon=... to PARKING_FEE_SERVICE.
func TestLookup(t *testing.T) {
	mock := newMockHTTPServer(t, 200, `[{"operator_id":"op-1","name":"Demo"}]`)
	binary := parkingAppBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"lookup",
		"--lat=48.1351",
		"--lon=11.5820",
		"--service-addr="+mock.URL(),
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}

	// Verify correct endpoint was called
	if !strings.Contains(mock.lastPath, "/operators") {
		t.Errorf("expected /operators endpoint, got %q", mock.lastPath)
	}
	if !strings.Contains(mock.lastPath, "lat=48.1351") {
		t.Errorf("expected lat=48.1351 in query, got %q", mock.lastPath)
	}
	if !strings.Contains(mock.lastPath, "lon=11.5820") {
		t.Errorf("expected lon=11.5820 in query, got %q", mock.lastPath)
	}

	if !strings.Contains(stdout, "op-1") {
		t.Errorf("expected op-1 in stdout, got %q", stdout)
	}
}

// TS-09-6: parking-app-cli adapter-info sends GET /operators/{id}/adapter.
func TestAdapterInfo(t *testing.T) {
	mock := newMockHTTPServer(t, 200, `{"image_ref":"registry/adapter:v1","checksum":"sha256:abc"}`)
	binary := parkingAppBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"adapter-info",
		"--operator-id=op-1",
		"--service-addr="+mock.URL(),
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}

	if !strings.Contains(mock.lastPath, "/operators/op-1/adapter") {
		t.Errorf("expected /operators/op-1/adapter path, got %q", mock.lastPath)
	}

	if !strings.Contains(stdout, "image_ref") {
		t.Errorf("expected image_ref in stdout, got %q", stdout)
	}
}

// TS-09-7: parking-app-cli install calls InstallAdapter on UPDATE_SERVICE.
func TestInstall(t *testing.T) {
	mockAddr := startMockUpdateServer(t)
	binary := parkingAppBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"install",
		"--image-ref=registry/adapter:v1",
		"--checksum=sha256:abc",
		"--update-addr="+mockAddr,
	)

	if code != 0 {
		t.Errorf("expected exit 0 on successful install, got %d (stderr=%q)", code, stderr)
	}

	// TS-09-7: response should contain job_id and adapter_id
	if !strings.Contains(stdout, "job_id") {
		t.Errorf("expected job_id in stdout, got %q", stdout)
	}
}

// TS-09-8: parking-app-cli list calls ListAdapters on UPDATE_SERVICE.
func TestList(t *testing.T) {
	mockAddr := startMockUpdateServer(t)
	binary := parkingAppBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"list",
		"--update-addr="+mockAddr,
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}
	if !strings.Contains(stdout, "adapter") {
		t.Errorf("expected adapter in stdout, got %q", stdout)
	}
}

// 09-REQ-5.4: parking-app-cli status checks adapter status on UPDATE_SERVICE.
// (Skeptic finding: missing test for GetAdapterStatus)
func TestAdapterStatus(t *testing.T) {
	mockAddr := startMockUpdateServer(t)
	binary := parkingAppBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"status",
		"--adapter-id=a1",
		"--update-addr="+mockAddr,
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}
	if !strings.Contains(stdout, "adapter_id") {
		t.Errorf("expected adapter_id in stdout, got %q", stdout)
	}
}

// 09-REQ-5.5: parking-app-cli remove calls RemoveAdapter on UPDATE_SERVICE.
// (Skeptic finding: missing test for RemoveAdapter)
func TestRemoveAdapter(t *testing.T) {
	mockAddr := startMockUpdateServer(t)
	binary := parkingAppBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"remove",
		"--adapter-id=a1",
		"--update-addr="+mockAddr,
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}
	if !strings.Contains(stdout, "removed") || !strings.Contains(stdout, "adapter_id") {
		t.Errorf("expected removed/adapter_id in stdout, got %q", stdout)
	}
}

// TS-09-9: parking-app-cli start-session calls StartSession on PARKING_OPERATOR_ADAPTOR.
func TestStartSession(t *testing.T) {
	mockAddr := startMockAdapterServer(t)
	binary := parkingAppBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"start-session",
		"--zone-id=zone-demo-1",
		"--adaptor-addr="+mockAddr,
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}
	if !strings.Contains(stdout, "session_id") {
		t.Errorf("expected session_id in stdout, got %q", stdout)
	}
}

// TS-09-10: parking-app-cli stop-session calls StopSession on PARKING_OPERATOR_ADAPTOR.
func TestStopSession(t *testing.T) {
	mockAddr := startMockAdapterServer(t)
	binary := parkingAppBin(t)

	stdout, stderr, code := runBinary(t, binary,
		"stop-session",
		"--adaptor-addr="+mockAddr,
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}
	if !strings.Contains(stdout, "stopped") {
		t.Errorf("expected stopped in stdout, got %q", stdout)
	}
}

// TS-09-E10: parking-app-cli exits 1 and prints gRPC error when UPDATE_SERVICE is unreachable.
func TestInstallGRPCError(t *testing.T) {
	binary := parkingAppBin(t)

	_, stderr, code := runBinary(t, binary,
		"install",
		"--image-ref=x",
		"--checksum=y",
		"--update-addr=localhost:59997", // unreachable
	)

	// With real implementation: exit 1 when gRPC fails
	// With stub: exits 0 → this test fails (red phase ✓)
	if code != 1 {
		t.Errorf("expected exit 1 on gRPC error (unreachable server), got %d", code)
	}
	if len(stderr) == 0 {
		t.Error("expected error message in stderr on gRPC failure")
	}
}

// TS-09-E11 / 09-REQ-4.E2: parking-app-cli exits 1 on non-2xx HTTP from PARKING_FEE_SERVICE.
func TestLookupHTTPError(t *testing.T) {
	mock := newMockHTTPServer(t, 500, `internal error`)
	binary := parkingAppBin(t)

	_, stderr, code := runBinary(t, binary,
		"lookup",
		"--lat=0",
		"--lon=0",
		"--service-addr="+mock.URL(),
	)

	if code != 1 {
		t.Errorf("expected exit 1 on non-2xx HTTP response, got %d", code)
	}
	if !strings.Contains(stderr, "500") {
		t.Errorf("expected HTTP 500 in stderr, got %q", stderr)
	}
}

// 09-REQ-4.E1: parking-app-cli lookup exits 1 when --lat or --lon is missing.
// (Skeptic finding: missing test for lookup required-flag validation)
func TestLookupMissingArgs(t *testing.T) {
	binary := parkingAppBin(t)

	// Missing --lon
	_, stderr, code := runBinary(t, binary, "lookup", "--lat=48.1351")
	if code != 1 {
		t.Errorf("lookup without --lon: expected exit 1, got %d", code)
	}
	if len(stderr) == 0 {
		t.Error("expected error in stderr when --lon is missing")
	}

	// Missing both
	_, stderr2, code2 := runBinary(t, binary, "lookup")
	if code2 != 1 {
		t.Errorf("lookup without args: expected exit 1, got %d", code2)
	}
	if len(stderr2) == 0 {
		t.Error("expected error in stderr when lat/lon missing")
	}
}

// 09-REQ-5.E1: parking-app-cli install exits 1 when --image-ref or --checksum is missing.
func TestInstallMissingArgs(t *testing.T) {
	binary := parkingAppBin(t)

	// Missing --checksum
	_, stderr, code := runBinary(t, binary, "install", "--image-ref=x")
	if code != 1 {
		t.Errorf("install without --checksum: expected exit 1, got %d", code)
	}
	if len(stderr) == 0 {
		t.Error("expected error in stderr when --checksum is missing")
	}
}
