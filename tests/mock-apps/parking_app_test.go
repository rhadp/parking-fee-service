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
// TS-09-E13: Parking App CLI Missing Required Flags
// Requirement: 09-REQ-4.E1
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

// ---------------------------------------------------------------------------
// TS-09-E14: Parking App CLI Lifecycle Missing Flags
// Requirement: 09-REQ-5.E1
// ---------------------------------------------------------------------------

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
// TS-09-P2 (Go CLI part): Argument Validation Property for Go CLI apps
// Property 2 from design.md
// Requirement: 09-REQ-4.E1, 09-REQ-5.E1, 09-REQ-7.E1
// ---------------------------------------------------------------------------

func TestGoCliArgumentValidationProperty(t *testing.T) {
	parkBin := parkingAppBinary(t)
	compBin := companionBinary(t)

	tests := []struct {
		name   string
		binary string
		args   []string
	}{
		// parking-app-cli: missing args per subcommand
		{"parking_no_subcommand", "parking", nil},
		{"lookup_missing_all", "parking", []string{"lookup"}},
		{"lookup_missing_lon", "parking", []string{"lookup", "--lat=48.13"}},
		{"lookup_missing_lat", "parking", []string{"lookup", "--lon=11.58"}},
		{"adapter_info_missing_id", "parking", []string{"adapter-info"}},
		{"install_missing_all", "parking", []string{"install"}},
		{"install_missing_checksum", "parking", []string{"install", "--image-ref=x"}},
		{"install_missing_image_ref", "parking", []string{"install", "--checksum=y"}},
		{"status_missing_adapter_id", "parking", []string{"status"}},
		{"remove_missing_adapter_id", "parking", []string{"remove"}},
		{"start_session_missing_zone", "parking", []string{"start-session"}},

		// companion-app-cli: missing args per subcommand
		{"companion_no_subcommand", "companion", nil},
		{"lock_missing_all", "companion", []string{"lock"}},
		{"lock_missing_vin", "companion", []string{"lock", "--token=t"}},
		{"lock_missing_token", "companion", []string{"lock", "--vin=V1"}},
		{"unlock_missing_vin", "companion", []string{"unlock", "--token=t"}},
		{"unlock_missing_token", "companion", []string{"unlock", "--vin=V1"}},
		{"status_missing_vin", "companion", []string{"status", "--token=t", "--command-id=c1"}},
		{"status_missing_token", "companion", []string{"status", "--vin=V1", "--command-id=c1"}},
		{"status_missing_command_id", "companion", []string{"status", "--vin=V1", "--token=t"}},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			var bin string
			if tc.binary == "parking" {
				bin = parkBin
			} else {
				bin = compBin
			}

			env := baseEnv()
			_, stderr, exitCode := runBinary(t, bin, tc.args, env)

			if exitCode != 1 {
				t.Errorf("expected exit 1, got %d", exitCode)
			}
			if len(stderr) == 0 {
				t.Error("expected non-empty stderr")
			}
		})
	}
}
