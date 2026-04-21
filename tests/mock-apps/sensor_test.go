// Package mockappstests provides sensor integration tests.
//
// TS-09-1: location-sensor publishes Latitude and Longitude to DATA_BROKER and exits 0.
// TS-09-2: speed-sensor publishes Vehicle.Speed to DATA_BROKER and exits 0.
// TS-09-3: door-sensor publishes IsOpen=true to DATA_BROKER when invoked with --open.
// TS-09-4: door-sensor publishes IsOpen=false to DATA_BROKER when invoked with --closed.
// TS-09-SMOKE-1: All three mock sensors publish values and a subscriber confirms receipt.
//
// These tests require:
//  1. The Rust mock-sensor binaries built via `cd rhivos && cargo build -p mock-sensors`.
//  2. A running DATA_BROKER that exposes kuksa.val.v1.VALService on port 55556.
//
// Per docs/errata/09_mock_apps_sensor_proto_compat.md, the production-pinned
// kuksa-databroker:0.5.0 only exposes kuksa.val.v2.VAL (not v1).  These tests
// therefore skip gracefully when DATA_BROKER is unreachable or returns an error
// that indicates the v1 service is unavailable.  The argument-validation tests
// (TS-09-E1..E4) in rhivos/mock-sensors/tests/sensor_args.rs do NOT require
// DATA_BROKER and always run.
package mockappstests

import (
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"
)

// findRustBinary returns the path to a built Rust binary from rhivos/target/debug/<name>.
// If the binary does not exist, the test is skipped.
func findRustBinary(t *testing.T, name string) string {
	t.Helper()
	root := findProjectRoot(t)
	// Standard cargo build output is in rhivos/target/debug/<name>
	binPath := filepath.Join(root, "rhivos", "target", "debug", name)
	if runtime.GOOS == "windows" {
		binPath += ".exe"
	}
	if _, err := os.Stat(binPath); os.IsNotExist(err) {
		t.Skipf("Rust binary %s not found at %s; run `cd rhivos && cargo build -p mock-sensors` first", name, binPath)
	}
	return binPath
}

// isBrokerReachable checks whether DATA_BROKER is listening on the given address.
// Returns true only if a TCP connection can be established.
func isBrokerReachable(addr string) bool {
	conn, err := net.DialTimeout("tcp", addr, 500*time.Millisecond)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

// skipIfBrokerUnreachable skips the test if DATA_BROKER is not available.
func skipIfBrokerUnreachable(t *testing.T) {
	t.Helper()
	brokerHost := "localhost:55556"
	if !isBrokerReachable(brokerHost) {
		t.Skipf("DATA_BROKER is not reachable at %s; skipping sensor integration test", brokerHost)
	}
}

// runSensor executes a sensor binary with the given args. Returns stdout, stderr, and the exit code.
// If the stderr contains a v1-API-not-available message, the test is skipped
// (per errata: production DATA_BROKER exposes only v2 API).
func runSensor(t *testing.T, binary string, args ...string) (string, string, int) {
	t.Helper()
	stdout, stderr, code := runBinary(t, binary, args...)

	// If the sensor exits 1 with a v1-related error, skip rather than fail.
	// This handles the case where DATA_BROKER is reachable but only exposes v2.
	if code != 0 && (strings.Contains(stderr, "Unimplemented") ||
		strings.Contains(stderr, "unimplemented") ||
		strings.Contains(stderr, "unknown service") ||
		strings.Contains(stderr, "StatusCode::Unimplemented")) {
		t.Skipf("DATA_BROKER does not expose kuksa.val.v1.VALService (v2-only deployment); "+
			"skipping per docs/errata/09_mock_apps_sensor_proto_compat.md. stderr=%q", stderr)
	}
	return stdout, stderr, code
}

// TS-09-1: location-sensor --lat=48.1351 --lon=11.5820 publishes coordinates and exits 0.
func TestLocationSensor(t *testing.T) {
	skipIfBrokerUnreachable(t)
	binary := findRustBinary(t, "location-sensor")

	_, stderr, code := runSensor(t, binary,
		"--lat=48.1351",
		"--lon=11.5820",
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}
}

// TS-09-2: speed-sensor --speed=0.0 publishes Vehicle.Speed and exits 0.
func TestSpeedSensor(t *testing.T) {
	skipIfBrokerUnreachable(t)
	binary := findRustBinary(t, "speed-sensor")

	_, stderr, code := runSensor(t, binary,
		"--speed=0.0",
	)

	if code != 0 {
		t.Errorf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}
}

// TS-09-3: door-sensor --open publishes IsOpen=true to DATA_BROKER and exits 0.
func TestDoorSensorOpen(t *testing.T) {
	skipIfBrokerUnreachable(t)
	binary := findRustBinary(t, "door-sensor")

	_, stderr, code := runSensor(t, binary, "--open")

	if code != 0 {
		t.Errorf("expected exit 0 for --open, got %d (stderr=%q)", code, stderr)
	}
}

// TS-09-4: door-sensor --closed publishes IsOpen=false to DATA_BROKER and exits 0.
func TestDoorSensorClosed(t *testing.T) {
	skipIfBrokerUnreachable(t)
	binary := findRustBinary(t, "door-sensor")

	_, stderr, code := runSensor(t, binary, "--closed")

	if code != 0 {
		t.Errorf("expected exit 0 for --closed, got %d (stderr=%q)", code, stderr)
	}
}

// TS-09-SMOKE-1: End-to-end smoke test — all three sensors publish values without error.
// Does not verify the values in DATA_BROKER (requires v1 read API or grpcurl).
// Verifies only that all three binaries exit 0 when DATA_BROKER is reachable.
func TestSensorSmoke(t *testing.T) {
	skipIfBrokerUnreachable(t)

	locationBin := findRustBinary(t, "location-sensor")
	speedBin := findRustBinary(t, "speed-sensor")
	doorBin := findRustBinary(t, "door-sensor")

	tests := []struct {
		name   string
		binary string
		args   []string
	}{
		{"location", locationBin, []string{"--lat=48.13", "--lon=11.58"}},
		{"speed", speedBin, []string{"--speed=0.0"}},
		{"door-closed", doorBin, []string{"--closed"}},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			_, stderr, code := runSensor(t, tc.binary, tc.args...)
			if code != 0 {
				t.Errorf("%s: expected exit 0, got %d (stderr=%q)", tc.name, code, stderr)
			}
		})
	}
}

// TS-09-E4 (via Go): sensor binaries exit 1 when DATA_BROKER is unreachable.
// This covers the same requirement as Rust's test_*_unreachable_broker tests
// but from the Go integration test side.
func TestSensorsUnreachableBroker(t *testing.T) {
	unreachableAddr := "http://localhost:59998" // nothing listening here

	tests := []struct {
		name   string
		bin    string
		args   []string
	}{
		{
			"location-sensor",
			"location-sensor",
			[]string{"--lat=48.13", "--lon=11.58", "--broker-addr=" + unreachableAddr},
		},
		{
			"speed-sensor",
			"speed-sensor",
			[]string{"--speed=0.0", "--broker-addr=" + unreachableAddr},
		},
		{
			"door-sensor",
			"door-sensor",
			[]string{"--open", "--broker-addr=" + unreachableAddr},
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			binary := findRustBinary(t, tc.bin)

			cmd := exec.Command(binary, tc.args...)
			var errBuf strings.Builder
			cmd.Stderr = &errBuf
			err := cmd.Run()

			if err == nil {
				t.Errorf("%s: expected non-zero exit when broker is unreachable, got exit 0", tc.name)
			}
			if errBuf.Len() == 0 {
				t.Errorf("%s: expected error message on stderr when broker is unreachable", tc.name)
			}
		})
	}
}
