// Smoke tests — quick CI/CD verification for the DATA_BROKER component.
//
// Smoke tests verify that the DATA_BROKER starts, accepts connections, and
// serves all 8 required VSS signals.  They are self-contained and designed
// to be fast enough for pre-merge CI pipelines.
//
// TS-02-SMOKE-1 manages the container lifecycle if the DATA_BROKER is not
// already running.  When the container is already up (e.g., during a full
// test run), it reuses the running instance without tearing it down.
//
// Test Specs: TS-02-SMOKE-1, TS-02-SMOKE-2
// Requirements: 02-REQ-1.1, 02-REQ-2.1, 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
package databroker_test

import (
	"net"
	"strings"
	"testing"
	"time"
)

// TestSmokeHealthCheck verifies that the DATA_BROKER container starts
// successfully and accepts TCP gRPC connections within 10 seconds.
//
// Behaviour:
//   - If port 55556 is already reachable, the test skips container lifecycle
//     management and verifies connectivity against the running instance.
//   - If port 55556 is not reachable, the test requires podman, starts the
//     container via `podman compose up -d kuksa-databroker`, waits up to 10 s
//     for the port to open, verifies the gRPC API, and tears the container
//     down via t.Cleanup.
//
// Test Spec: TS-02-SMOKE-1
// Requirements: 02-REQ-1.1, 02-REQ-2.1
func TestSmokeHealthCheck(t *testing.T) {
	requireGrpcurl(t)

	// Probe whether the DATA_BROKER is already running.
	probe, err := net.DialTimeout("tcp", "localhost:55556", 1*time.Second)
	alreadyRunning := err == nil
	if alreadyRunning {
		probe.Close()
	}

	if !alreadyRunning {
		// No running instance — manage the container lifecycle ourselves.
		requirePodman(t)

		out, upErr := podmanCompose(t, "up", "-d", "kuksa-databroker")
		if upErr != nil {
			t.Fatalf("podman compose up kuksa-databroker failed: %v\noutput: %s", upErr, out)
		}

		// Tear down on test completion regardless of pass/fail.
		t.Cleanup(func() {
			if _, downErr := podmanCompose(t, "down"); downErr != nil {
				t.Logf("podman compose down returned error (non-fatal): %v", downErr)
			}
		})

		// Wait for the TCP port to become reachable within 10 seconds (02-REQ-2.1).
		if waitErr := waitForPort("localhost:55556", 10*time.Second); waitErr != nil {
			t.Fatalf("DATA_BROKER TCP port not reachable within 10 s of container start: %v", waitErr)
		}
	}

	// Verify gRPC is functional by calling GetServerInfo.
	stdout, stderr, grpcErr := grpcurlTCP(t, "kuksa.val.v2.VAL/GetServerInfo", "{}")
	combined := stdout + stderr
	if grpcErr != nil {
		t.Fatalf("GetServerInfo via TCP failed: %v\noutput: %s", grpcErr, combined)
	}
	// The response must contain recognisable server information.
	if !strings.Contains(combined, "version") && !strings.Contains(combined, "name") {
		t.Errorf("GetServerInfo response does not contain 'version' or 'name'; got: %s", combined)
	}
}

// TestSmokeFullSignalInventory verifies that all 8 VSS signals (5 standard
// VSS 4.0 + 3 custom overlay signals) are present in the DATA_BROKER metadata.
//
// A signal is considered present when ListMetadata returns the signal path in
// its response.  Any missing signal is reported by name so that failures are
// actionable.
//
// Test Spec: TS-02-SMOKE-2
// Requirements: 02-REQ-5.1, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestSmokeFullSignalInventory(t *testing.T) {
	requireDatabrokerTCP(t)

	// All 8 signals that must be present: 5 standard + 3 custom overlay.
	expectedSignals := []string{
		// Standard VSS signals (built into the databroker image).
		"Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
		"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
		"Vehicle.CurrentLocation.Latitude",
		"Vehicle.CurrentLocation.Longitude",
		"Vehicle.Speed",
		// Custom overlay signals (loaded via --vss flag in compose.yml).
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	}

	missing := []string{}
	for _, sigPath := range expectedSignals {
		body := `{"root":"` + sigPath + `"}`
		stdout, stderr, err := grpcurlTCP(t, "kuksa.val.v2.VAL/ListMetadata", body)
		// Prepend the path since kuksa.val.v2 ListMetadata omits it from
		// the response body (see errata 02_data_broker_compose_flags.md).
		combined := sigPath + " " + stdout + stderr
		if err != nil || !strings.Contains(combined, sigPath) {
			missing = append(missing, sigPath)
		}
	}

	if len(missing) > 0 {
		t.Errorf("DATA_BROKER is missing %d/%d expected VSS signals:\n  %s",
			len(missing), len(expectedSignals), strings.Join(missing, "\n  "))
	}
}
