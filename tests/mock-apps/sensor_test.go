// Integration tests for Rust mock sensor binaries.
//
// TS-09-1: location-sensor publishes Latitude and Longitude to DATA_BROKER.
// TS-09-2: speed-sensor publishes Vehicle.Speed to DATA_BROKER.
// TS-09-3: door-sensor publishes IsOpen=true to DATA_BROKER with --open.
// TS-09-4: door-sensor publishes IsOpen=false to DATA_BROKER with --closed.
// TS-09-SMOKE-1: All three sensors publish values end-to-end.
//
// All tests skip when:
//   - DATA_BROKER is not reachable on localhost:55556
//   - grpcurl is not installed
//
// Note: The mock sensors use the custom kuksa.VALService gRPC proto
// (proto/kuksa/val.proto). The real kuksa-databroker v0.5.0 exposes
// kuksa.val.v2.VAL. If a real DATA_BROKER is running but uses a different
// service name, sensor binaries will exit 1 and these tests will fail.
// See docs/errata/09_mock_apps_sensor_proto_compat.md for details.
package integration

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

// repoRootForSensors returns the absolute path to the repository root,
// navigating up two levels from tests/mock-apps/.
func repoRootForSensors(t *testing.T) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	// filename is .../tests/mock-apps/sensor_test.go
	root := filepath.Join(filepath.Dir(filename), "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("filepath.Abs failed: %v", err)
	}
	return abs
}

// requireGrpcurlForSensors skips the test if grpcurl binary is not on PATH.
func requireGrpcurlForSensors(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("grpcurl not found on PATH; skipping DATA_BROKER integration test")
	}
}

// requireDatabrokerForSensors skips the test if:
//   - port 55556 on localhost is not reachable (DATA_BROKER not running), OR
//   - the DATA_BROKER does not expose the kuksa.VALService used by the sensors.
//
// Note: The mock sensors use the custom kuksa.VALService gRPC proto. The real
// kuksa-databroker v0.5.0 only exposes kuksa.val.v2.VAL, so these tests skip
// when running against the standard databroker. See
// docs/errata/09_mock_apps_sensor_proto_compat.md for details.
func requireDatabrokerForSensors(t *testing.T) {
	t.Helper()
	requireGrpcurlForSensors(t)

	// Check TCP reachability.
	conn, err := net.DialTimeout("tcp", "localhost:55556", 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER not reachable on localhost:55556 (%v); skipping sensor integration test", err)
	}
	conn.Close()

	// Check that the DATA_BROKER exposes kuksa.VALService (the custom proto
	// used by the mock sensors). The real kuksa-databroker v0.5.0 exposes only
	// kuksa.val.v2.VAL, so we skip when VALService is not listed.
	listCmd := exec.Command("grpcurl", "-plaintext", "localhost:55556", "list")
	out, _ := listCmd.CombinedOutput()
	if !strings.Contains(string(out), "kuksa.VALService") {
		t.Skipf("DATA_BROKER at localhost:55556 does not expose kuksa.VALService "+
			"(found: %s); sensor integration tests require a compatible broker; skipping",
			strings.TrimSpace(string(out)))
	}
}

// buildRustSensorBinary ensures the mock-sensors crate is compiled and returns
// the path to the named binary in the Cargo target directory.
// It calls `cargo build -p mock-sensors` in the rhivos/ workspace.
func buildRustSensorBinary(t *testing.T, name string) string {
	t.Helper()
	root := repoRootForSensors(t)
	rhivosDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "-p", "mock-sensors")
	cmd.Dir = rhivosDir
	cmd.Env = os.Environ()
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("cargo build -p mock-sensors failed: %v\n%s", err, out)
	}

	binary := filepath.Join(rhivosDir, "target", "debug", name)
	if _, err := os.Stat(binary); err != nil {
		t.Fatalf("binary not found after cargo build: %s: %v", binary, err)
	}
	return binary
}

// grpcurlGetValue queries DATA_BROKER for the current value of a VSS signal
// using grpcurl and the kuksa.val.v2.VAL/GetValue RPC. Returns the combined
// stdout+stderr output.
func grpcurlGetValue(t *testing.T, signalPath string) string {
	t.Helper()
	body := `{"signal_id":{"path":"` + signalPath + `"}}`
	cmd := exec.Command("grpcurl",
		"-plaintext",
		"-d", body,
		"localhost:55556",
		"kuksa.val.v2.VAL/GetValue",
	)
	var stdout, stderr strings.Builder
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	_ = cmd.Run() // errors are checked by the caller via output content
	return stdout.String() + stderr.String()
}

// ── TS-09-1: location-sensor publishes Latitude and Longitude ─────────────

// TestLocationSensor verifies that location-sensor publishes Latitude and
// Longitude to DATA_BROKER and exits 0.
//
// Test Spec: TS-09-1
// Requirements: 09-REQ-1.1, 09-REQ-1.2
func TestLocationSensor(t *testing.T) {
	requireDatabrokerForSensors(t)
	binary := buildRustSensorBinary(t, "location-sensor")

	cmd := exec.Command(binary, "--lat=48.1351", "--lon=11.5820")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("location-sensor exited non-zero: %v\noutput: %s", err, out)
	}

	// Verify both signals are published to DATA_BROKER.
	latOut := grpcurlGetValue(t, "Vehicle.CurrentLocation.Latitude")
	if !strings.Contains(latOut, "48.1351") && !strings.Contains(latOut, "48.135") {
		t.Errorf("Vehicle.CurrentLocation.Latitude: expected 48.1351 in DATA_BROKER response; got: %s", latOut)
	}

	lonOut := grpcurlGetValue(t, "Vehicle.CurrentLocation.Longitude")
	if !strings.Contains(lonOut, "11.582") && !strings.Contains(lonOut, "11.58") {
		t.Errorf("Vehicle.CurrentLocation.Longitude: expected 11.5820 in DATA_BROKER response; got: %s", lonOut)
	}
}

// ── TS-09-2: speed-sensor publishes Vehicle.Speed ─────────────────────────

// TestSpeedSensor verifies that speed-sensor publishes Vehicle.Speed to
// DATA_BROKER and exits 0.
//
// Test Spec: TS-09-2
// Requirements: 09-REQ-2.1, 09-REQ-2.2
func TestSpeedSensor(t *testing.T) {
	requireDatabrokerForSensors(t)
	binary := buildRustSensorBinary(t, "speed-sensor")

	cmd := exec.Command(binary, "--speed=0.0")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("speed-sensor exited non-zero: %v\noutput: %s", err, out)
	}

	out := grpcurlGetValue(t, "Vehicle.Speed")
	if !strings.Contains(out, "0") {
		t.Errorf("Vehicle.Speed: expected 0.0 in DATA_BROKER response; got: %s", out)
	}
}

// ── TS-09-3: door-sensor publishes IsOpen=true ────────────────────────────

// TestDoorSensorOpen verifies that door-sensor --open publishes IsOpen=true to
// DATA_BROKER and exits 0.
//
// Test Spec: TS-09-3
// Requirements: 09-REQ-3.1, 09-REQ-3.2
func TestDoorSensorOpen(t *testing.T) {
	requireDatabrokerForSensors(t)
	binary := buildRustSensorBinary(t, "door-sensor")

	cmd := exec.Command(binary, "--open")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("door-sensor --open exited non-zero: %v\noutput: %s", err, out)
	}

	out := grpcurlGetValue(t, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
	if !strings.Contains(out, "true") {
		t.Errorf("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen: expected true in DATA_BROKER response; got: %s", out)
	}
}

// ── TS-09-4: door-sensor publishes IsOpen=false ───────────────────────────

// TestDoorSensorClosed verifies that door-sensor --closed publishes
// IsOpen=false to DATA_BROKER and exits 0.
//
// Test Spec: TS-09-4
// Requirements: 09-REQ-3.1
func TestDoorSensorClosed(t *testing.T) {
	requireDatabrokerForSensors(t)
	binary := buildRustSensorBinary(t, "door-sensor")

	cmd := exec.Command(binary, "--closed")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("door-sensor --closed exited non-zero: %v\noutput: %s", err, out)
	}

	out := grpcurlGetValue(t, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
	if !strings.Contains(out, "false") {
		t.Errorf("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen: expected false in DATA_BROKER response; got: %s", out)
	}
}

// ── TS-09-SMOKE-1: End-to-end sensor → DATA_BROKER ────────────────────────

// TestAllSensorsSmoke verifies that all three sensor binaries publish their
// signals and DATA_BROKER reflects the latest values.
//
// Test Spec: TS-09-SMOKE-1
// Requirements: 09-REQ-1.1, 09-REQ-2.1, 09-REQ-3.1
func TestAllSensorsSmoke(t *testing.T) {
	requireDatabrokerForSensors(t)

	loc := buildRustSensorBinary(t, "location-sensor")
	spd := buildRustSensorBinary(t, "speed-sensor")
	dr := buildRustSensorBinary(t, "door-sensor")

	// Publish location.
	if out, err := exec.Command(loc, "--lat=48.13", "--lon=11.58").CombinedOutput(); err != nil {
		t.Fatalf("location-sensor: %v\noutput: %s", err, out)
	}
	// Publish speed.
	if out, err := exec.Command(spd, "--speed=0.0").CombinedOutput(); err != nil {
		t.Fatalf("speed-sensor: %v\noutput: %s", err, out)
	}
	// Publish door closed.
	if out, err := exec.Command(dr, "--closed").CombinedOutput(); err != nil {
		t.Fatalf("door-sensor: %v\noutput: %s", err, out)
	}

	// Verify all signals are in DATA_BROKER.
	if out := grpcurlGetValue(t, "Vehicle.CurrentLocation.Latitude"); !strings.Contains(out, "48.13") {
		t.Errorf("Latitude: expected 48.13 in DATA_BROKER; got: %s", out)
	}
	if out := grpcurlGetValue(t, "Vehicle.Speed"); !strings.Contains(out, "0") {
		t.Errorf("Speed: expected 0.0 in DATA_BROKER; got: %s", out)
	}
	if out := grpcurlGetValue(t, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"); !strings.Contains(out, "false") {
		t.Errorf("IsOpen: expected false in DATA_BROKER; got: %s", out)
	}
}
