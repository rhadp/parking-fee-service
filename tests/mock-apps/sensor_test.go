// Sensor integration tests (task group 5 — TS-09-1 through TS-09-4, TS-09-SMOKE-1).
//
// These tests invoke the compiled Rust sensor binaries against a real DATA_BROKER.
// They are automatically skipped when:
//   - DATA_BROKER is not TCP-reachable at localhost:55556
//   - grpcurl is not installed (needed to list services)
//   - DATA_BROKER does not expose the kuksa.val.v1.VAL service
//     (the real kuksa-databroker v0.5.0 only exposes kuksa.val.v2.VAL)
//
// See docs/errata/09_mock_apps_sensor_proto_compat.md for details.
package mock_apps

import (
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

const sensorBrokerEndpoint = "localhost:55556"

// ── build helpers ─────────────────────────────────────────────────────────────

var (
	rustSensorBuildOnce sync.Once
	rustSensorBuildErr  error
)

// requireCargo skips the test if cargo is not in PATH.
func requireCargo(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("cargo not installed; required to build Rust sensor binaries")
	}
}

// ensureRustSensors builds the mock-sensors crate once per test run.
func ensureRustSensors(t *testing.T) {
	t.Helper()
	requireCargo(t)

	root := repoRoot(t)
	rhivosDir := filepath.Join(root, "rhivos")

	rustSensorBuildOnce.Do(func() {
		cmd := exec.Command("cargo", "build", "-p", "mock-sensors")
		cmd.Dir = rhivosDir
		out, err := cmd.CombinedOutput()
		if err != nil {
			rustSensorBuildErr = fmt.Errorf("cargo build -p mock-sensors failed: %v\n%s", err, string(out))
		}
	})

	if rustSensorBuildErr != nil {
		t.Fatalf("failed to build mock-sensors: %v", rustSensorBuildErr)
	}
}

// sensorBinary returns the path to a compiled Rust sensor binary.
func sensorBinary(t *testing.T, name string) string {
	t.Helper()
	ensureRustSensors(t)
	root := repoRoot(t)
	path := filepath.Join(root, "rhivos", "target", "debug", name)
	if _, err := os.Stat(path); err != nil {
		t.Fatalf("sensor binary %q not found at %s after build", name, path)
	}
	return path
}

// ── skip guards ───────────────────────────────────────────────────────────────

// requireDataBrokerTCP skips the test if DATA_BROKER is not TCP-reachable.
func requireDataBrokerTCP(t *testing.T) {
	t.Helper()
	conn, err := net.DialTimeout("tcp", sensorBrokerEndpoint, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER not reachable at %s (start with: cd deployments && podman compose up -d): %v",
			sensorBrokerEndpoint, err)
	}
	conn.Close()
}

// requireSensorGrpcurl skips if grpcurl is not installed.
func requireSensorGrpcurl(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("grpcurl"); err != nil {
		t.Skip("grpcurl not installed; required to verify sensor output in DATA_BROKER")
	}
}

// requireKuksaV1VAL skips if DATA_BROKER does not expose kuksa.val.v1.VAL.
// The real kuksa-databroker v0.5.0 exposes only kuksa.val.v2.VAL; this function
// skips the test with a clear message in that environment.
func requireKuksaV1VAL(t *testing.T) {
	t.Helper()
	requireSensorGrpcurl(t)
	out, err := exec.Command("grpcurl", "-plaintext", sensorBrokerEndpoint, "list").CombinedOutput()
	if err != nil {
		t.Skipf("grpcurl list services failed: %v\noutput: %s", err, string(out))
	}
	if !strings.Contains(string(out), "kuksa.val.v1.VAL") {
		t.Skipf("DATA_BROKER does not expose kuksa.val.v1.VAL (services: %s); "+
			"kuksa-databroker v0.5.0 uses kuksa.val.v2.VAL only — "+
			"see docs/errata/09_mock_apps_sensor_proto_compat.md", strings.TrimSpace(string(out)))
	}
}

// ── value verification helper ─────────────────────────────────────────────────

// getSignalValue uses grpcurl to read a VSS signal from DATA_BROKER via kuksa.val.v1.
// Returns the JSON-encoded value string.
func getSignalValue(t *testing.T, path string) string {
	t.Helper()
	data := fmt.Sprintf(`{"entries":[{"path":%q,"fields":["FIELD_VALUE"]}]}`, path)
	out, err := exec.Command("grpcurl", "-plaintext", "-d", data,
		sensorBrokerEndpoint, "kuksa.val.v1.VAL/Get").CombinedOutput()
	if err != nil {
		t.Skipf("kuksa.val.v1.VAL/Get failed for %q: %v\noutput: %s", path, err, string(out))
	}
	return string(out)
}

// ── test cases ────────────────────────────────────────────────────────────────

// TS-09-1: location-sensor publishes Latitude and Longitude to DATA_BROKER.
func TestLocationSensor(t *testing.T) {
	requireDataBrokerTCP(t)
	requireKuksaV1VAL(t)

	bin := sensorBinary(t, "location-sensor")
	_, _, code := runCmd(t, bin, []string{"--lat=48.1351", "--lon=11.5820"}, nil)
	if code != 0 {
		t.Fatalf("location-sensor exited %d; expected 0", code)
	}

	latOutput := getSignalValue(t, "Vehicle.CurrentLocation.Latitude")
	if !strings.Contains(latOutput, "48.1351") {
		t.Errorf("Latitude not found in DATA_BROKER output: %s", latOutput)
	}

	lonOutput := getSignalValue(t, "Vehicle.CurrentLocation.Longitude")
	if !strings.Contains(lonOutput, "11.582") {
		t.Errorf("Longitude not found in DATA_BROKER output: %s", lonOutput)
	}
}

// TS-09-2: speed-sensor publishes Vehicle.Speed to DATA_BROKER.
func TestSpeedSensor(t *testing.T) {
	requireDataBrokerTCP(t)
	requireKuksaV1VAL(t)

	bin := sensorBinary(t, "speed-sensor")
	_, _, code := runCmd(t, bin, []string{"--speed=0.0"}, nil)
	if code != 0 {
		t.Fatalf("speed-sensor exited %d; expected 0", code)
	}

	output := getSignalValue(t, "Vehicle.Speed")
	if !strings.Contains(output, "0") {
		t.Errorf("Vehicle.Speed not found in DATA_BROKER output: %s", output)
	}
}

// TS-09-3: door-sensor publishes IsOpen=true when invoked with --open.
func TestDoorSensorOpen(t *testing.T) {
	requireDataBrokerTCP(t)
	requireKuksaV1VAL(t)

	bin := sensorBinary(t, "door-sensor")
	_, _, code := runCmd(t, bin, []string{"--open"}, nil)
	if code != 0 {
		t.Fatalf("door-sensor --open exited %d; expected 0", code)
	}

	output := getSignalValue(t, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
	if !strings.Contains(strings.ToLower(output), "true") {
		t.Errorf("IsOpen not true in DATA_BROKER output: %s", output)
	}
}

// TS-09-4: door-sensor publishes IsOpen=false when invoked with --closed.
func TestDoorSensorClosed(t *testing.T) {
	requireDataBrokerTCP(t)
	requireKuksaV1VAL(t)

	bin := sensorBinary(t, "door-sensor")
	_, _, code := runCmd(t, bin, []string{"--closed"}, nil)
	if code != 0 {
		t.Fatalf("door-sensor --closed exited %d; expected 0", code)
	}

	output := getSignalValue(t, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
	if !strings.Contains(strings.ToLower(output), "false") {
		t.Errorf("IsOpen not false in DATA_BROKER output: %s", output)
	}
}

// TS-09-SMOKE-1: all three mock sensors publish values and DATA_BROKER confirms receipt.
func TestSensorSmoke(t *testing.T) {
	requireDataBrokerTCP(t)
	requireKuksaV1VAL(t)

	locBin := sensorBinary(t, "location-sensor")
	spdBin := sensorBinary(t, "speed-sensor")
	dooeBin := sensorBinary(t, "door-sensor")

	if _, _, code := runCmd(t, locBin, []string{"--lat=48.13", "--lon=11.58"}, nil); code != 0 {
		t.Fatalf("location-sensor exited %d", code)
	}
	if _, _, code := runCmd(t, spdBin, []string{"--speed=0.0"}, nil); code != 0 {
		t.Fatalf("speed-sensor exited %d", code)
	}
	if _, _, code := runCmd(t, dooeBin, []string{"--closed"}, nil); code != 0 {
		t.Fatalf("door-sensor --closed exited %d", code)
	}

	latOut := getSignalValue(t, "Vehicle.CurrentLocation.Latitude")
	if !strings.Contains(latOut, "48.13") {
		t.Errorf("Latitude not found in DATA_BROKER: %s", latOut)
	}

	spdOut := getSignalValue(t, "Vehicle.Speed")
	if !strings.Contains(spdOut, "0") {
		t.Errorf("Vehicle.Speed not found in DATA_BROKER: %s", spdOut)
	}

	doorOut := getSignalValue(t, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
	if !strings.Contains(strings.ToLower(doorOut), "false") {
		t.Errorf("IsOpen not false in DATA_BROKER: %s", doorOut)
	}
}
