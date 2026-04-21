// Package mockappstests provides sensor integration tests.
//
// TS-09-1: location-sensor publishes Latitude and Longitude to DATA_BROKER and exits 0.
// TS-09-2: speed-sensor publishes Vehicle.Speed to DATA_BROKER and exits 0.
// TS-09-3: door-sensor publishes IsOpen=true to DATA_BROKER when invoked with --open.
// TS-09-4: door-sensor publishes IsOpen=false to DATA_BROKER when invoked with --closed.
// TS-09-SMOKE-1: All three mock sensors publish values and a subscriber confirms receipt.
// TS-09-P1: Sensor publish-and-exit property — various inputs exit 0 and publish correct values.
//
// These tests use a mock kuksa.val.v1 VAL gRPC server to capture published values,
// eliminating the need for a real DATA_BROKER and avoiding v1/v2 proto incompatibility
// (see docs/errata/09_mock_apps_sensor_proto_compat.md).
package mockappstests

import (
	"fmt"
	"math"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	valpb "github.com/rhadp/parking-fee-service/tests/mock-apps/pb/kuksa_val_v1"
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

// findEntry searches published entries for a matching VSS path.
func findEntry(entries []publishedEntry, path string) *publishedEntry {
	for _, e := range entries {
		if e.Path == path {
			return &e
		}
	}
	return nil
}

// TS-09-1: location-sensor --lat=48.1351 --lon=11.5820 publishes Latitude and Longitude
// to DATA_BROKER via kuksa.val.v1 Set RPC, and exits 0.
// Verifies actual published VSS values via mock VAL server (09-REQ-1.1).
func TestLocationSensor(t *testing.T) {
	binary := findRustBinary(t, "location-sensor")
	valAddr, mockVal := startMockVALServer(t)

	_, stderr, code := runBinary(t, binary,
		"--lat=48.1351",
		"--lon=11.5820",
		"--broker-addr=http://"+valAddr,
	)

	if code != 0 {
		t.Fatalf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}

	entries := mockVal.getEntries()

	// Verify Vehicle.CurrentLocation.Latitude
	latEntry := findEntry(entries, "Vehicle.CurrentLocation.Latitude")
	if latEntry == nil {
		t.Fatal("expected Vehicle.CurrentLocation.Latitude entry, got none")
	}
	latVal, ok := latEntry.Value.Value.(*valpb.Datapoint_Double)
	if !ok {
		t.Fatalf("expected double value for Latitude, got %T", latEntry.Value.Value)
	}
	if math.Abs(latVal.Double-48.1351) > 1e-6 {
		t.Errorf("expected Latitude=48.1351, got %v", latVal.Double)
	}

	// Verify Vehicle.CurrentLocation.Longitude
	lonEntry := findEntry(entries, "Vehicle.CurrentLocation.Longitude")
	if lonEntry == nil {
		t.Fatal("expected Vehicle.CurrentLocation.Longitude entry, got none")
	}
	lonVal, ok := lonEntry.Value.Value.(*valpb.Datapoint_Double)
	if !ok {
		t.Fatalf("expected double value for Longitude, got %T", lonEntry.Value.Value)
	}
	if math.Abs(lonVal.Double-11.5820) > 1e-6 {
		t.Errorf("expected Longitude=11.5820, got %v", lonVal.Double)
	}
}

// TS-09-2: speed-sensor --speed=0.0 publishes Vehicle.Speed to DATA_BROKER and exits 0.
// Verifies actual published VSS value via mock VAL server (09-REQ-2.1).
func TestSpeedSensor(t *testing.T) {
	binary := findRustBinary(t, "speed-sensor")
	valAddr, mockVal := startMockVALServer(t)

	_, stderr, code := runBinary(t, binary,
		"--speed=0.0",
		"--broker-addr=http://"+valAddr,
	)

	if code != 0 {
		t.Fatalf("expected exit 0, got %d (stderr=%q)", code, stderr)
	}

	entries := mockVal.getEntries()
	speedEntry := findEntry(entries, "Vehicle.Speed")
	if speedEntry == nil {
		t.Fatal("expected Vehicle.Speed entry, got none")
	}
	speedVal, ok := speedEntry.Value.Value.(*valpb.Datapoint_Float)
	if !ok {
		t.Fatalf("expected float value for Speed, got %T", speedEntry.Value.Value)
	}
	if speedVal.Float != 0.0 {
		t.Errorf("expected Speed=0.0, got %v", speedVal.Float)
	}
}

// TS-09-3: door-sensor --open publishes IsOpen=true to DATA_BROKER and exits 0.
// Verifies actual published VSS value via mock VAL server (09-REQ-3.1).
func TestDoorSensorOpen(t *testing.T) {
	binary := findRustBinary(t, "door-sensor")
	valAddr, mockVal := startMockVALServer(t)

	_, stderr, code := runBinary(t, binary,
		"--open",
		"--broker-addr=http://"+valAddr,
	)

	if code != 0 {
		t.Fatalf("expected exit 0 for --open, got %d (stderr=%q)", code, stderr)
	}

	entries := mockVal.getEntries()
	doorEntry := findEntry(entries, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
	if doorEntry == nil {
		t.Fatal("expected Vehicle.Cabin.Door.Row1.DriverSide.IsOpen entry, got none")
	}
	boolVal, ok := doorEntry.Value.Value.(*valpb.Datapoint_Bool)
	if !ok {
		t.Fatalf("expected bool value for IsOpen, got %T", doorEntry.Value.Value)
	}
	if !boolVal.Bool {
		t.Errorf("expected IsOpen=true for --open, got false")
	}
}

// TS-09-4: door-sensor --closed publishes IsOpen=false to DATA_BROKER and exits 0.
// Verifies actual published VSS value via mock VAL server (09-REQ-3.1).
func TestDoorSensorClosed(t *testing.T) {
	binary := findRustBinary(t, "door-sensor")
	valAddr, mockVal := startMockVALServer(t)

	_, stderr, code := runBinary(t, binary,
		"--closed",
		"--broker-addr=http://"+valAddr,
	)

	if code != 0 {
		t.Fatalf("expected exit 0 for --closed, got %d (stderr=%q)", code, stderr)
	}

	entries := mockVal.getEntries()
	doorEntry := findEntry(entries, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
	if doorEntry == nil {
		t.Fatal("expected Vehicle.Cabin.Door.Row1.DriverSide.IsOpen entry, got none")
	}
	boolVal, ok := doorEntry.Value.Value.(*valpb.Datapoint_Bool)
	if !ok {
		t.Fatalf("expected bool value for IsOpen, got %T", doorEntry.Value.Value)
	}
	if boolVal.Bool {
		t.Errorf("expected IsOpen=false for --closed, got true")
	}
}

// TS-09-SMOKE-1: End-to-end smoke test — all three sensors publish correct values.
// Uses a mock VAL server to verify each sensor publishes the expected VSS signal.
func TestSensorSmoke(t *testing.T) {
	locationBin := findRustBinary(t, "location-sensor")
	speedBin := findRustBinary(t, "speed-sensor")
	doorBin := findRustBinary(t, "door-sensor")

	valAddr, mockVal := startMockVALServer(t)
	brokerArg := "--broker-addr=http://" + valAddr

	tests := []struct {
		name       string
		binary     string
		args       []string
		expectPath string
	}{
		{"location", locationBin, []string{"--lat=48.13", "--lon=11.58", brokerArg}, "Vehicle.CurrentLocation.Latitude"},
		{"speed", speedBin, []string{"--speed=0.0", brokerArg}, "Vehicle.Speed"},
		{"door-closed", doorBin, []string{"--closed", brokerArg}, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen"},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			_, stderr, code := runBinary(t, tc.binary, tc.args...)
			if code != 0 {
				t.Errorf("%s: expected exit 0, got %d (stderr=%q)", tc.name, code, stderr)
			}
		})
	}

	// Verify all expected VSS paths were published
	entries := mockVal.getEntries()
	expectedPaths := []string{
		"Vehicle.CurrentLocation.Latitude",
		"Vehicle.CurrentLocation.Longitude",
		"Vehicle.Speed",
		"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
	}
	for _, path := range expectedPaths {
		if findEntry(entries, path) == nil {
			t.Errorf("expected VSS path %q to be published, but was not found", path)
		}
	}
}

// TS-09-P1: Sensor publish-and-exit property test.
// For various valid inputs, verifies that each sensor exits 0 and publishes
// the correct VSS value to DATA_BROKER (Design Property 1, 09-REQ-1.1, 09-REQ-2.1, 09-REQ-3.1).
func TestSensorPublishProperty(t *testing.T) {
	locationBin := findRustBinary(t, "location-sensor")
	speedBin := findRustBinary(t, "speed-sensor")
	doorBin := findRustBinary(t, "door-sensor")

	// Property: For any valid lat/lon, location-sensor publishes correct values and exits 0.
	t.Run("location-sensor", func(t *testing.T) {
		latLonCases := []struct {
			lat float64
			lon float64
		}{
			{0.0, 0.0},
			{48.1351, 11.5820},
			{-33.8688, 151.2093},   // Sydney
			{90.0, 180.0},          // max boundary
			{-90.0, -180.0},        // min boundary
			{51.5074, -0.1278},     // London
			{35.6762, 139.6503},    // Tokyo
			{-22.9068, -43.1729},   // Rio
			{0.0001, 0.0001},       // near origin
			{89.9999, -179.9999},   // near pole/date line
		}

		for _, tc := range latLonCases {
			valAddr, mockVal := startMockVALServer(t)
			_, stderr, code := runBinary(t, locationBin,
				fmt.Sprintf("--lat=%v", tc.lat),
				fmt.Sprintf("--lon=%v", tc.lon),
				"--broker-addr=http://"+valAddr,
			)
			if code != 0 {
				t.Errorf("lat=%.4f lon=%.4f: expected exit 0, got %d (stderr=%q)", tc.lat, tc.lon, code, stderr)
				continue
			}

			entries := mockVal.getEntries()
			latEntry := findEntry(entries, "Vehicle.CurrentLocation.Latitude")
			if latEntry == nil {
				t.Errorf("lat=%.4f lon=%.4f: Latitude not published", tc.lat, tc.lon)
				continue
			}
			latDP, ok := latEntry.Value.Value.(*valpb.Datapoint_Double)
			if !ok {
				t.Errorf("lat=%.4f lon=%.4f: Latitude not a double", tc.lat, tc.lon)
				continue
			}
			if math.Abs(latDP.Double-tc.lat) > 1e-4 {
				t.Errorf("lat=%.4f lon=%.4f: Latitude=%v, want %v", tc.lat, tc.lon, latDP.Double, tc.lat)
			}

			lonEntry := findEntry(entries, "Vehicle.CurrentLocation.Longitude")
			if lonEntry == nil {
				t.Errorf("lat=%.4f lon=%.4f: Longitude not published", tc.lat, tc.lon)
				continue
			}
			lonDP, ok := lonEntry.Value.Value.(*valpb.Datapoint_Double)
			if !ok {
				t.Errorf("lat=%.4f lon=%.4f: Longitude not a double", tc.lat, tc.lon)
				continue
			}
			if math.Abs(lonDP.Double-tc.lon) > 1e-4 {
				t.Errorf("lat=%.4f lon=%.4f: Longitude=%v, want %v", tc.lat, tc.lon, lonDP.Double, tc.lon)
			}
		}
	})

	// Property: For any valid speed, speed-sensor publishes correct value and exits 0.
	t.Run("speed-sensor", func(t *testing.T) {
		speedCases := []float32{0.0, 1.0, 50.5, 120.0, 200.0, 250.0, 0.001, 999.9}

		for _, speed := range speedCases {
			valAddr, mockVal := startMockVALServer(t)
			_, stderr, code := runBinary(t, speedBin,
				fmt.Sprintf("--speed=%v", speed),
				"--broker-addr=http://"+valAddr,
			)
			if code != 0 {
				t.Errorf("speed=%v: expected exit 0, got %d (stderr=%q)", speed, code, stderr)
				continue
			}

			entries := mockVal.getEntries()
			speedEntry := findEntry(entries, "Vehicle.Speed")
			if speedEntry == nil {
				t.Errorf("speed=%v: Vehicle.Speed not published", speed)
				continue
			}
			speedDP, ok := speedEntry.Value.Value.(*valpb.Datapoint_Float)
			if !ok {
				t.Errorf("speed=%v: Vehicle.Speed not a float", speed)
				continue
			}
			if math.Abs(float64(speedDP.Float-speed)) > 1e-3 {
				t.Errorf("speed=%v: published=%v", speed, speedDP.Float)
			}
		}
	})

	// Property: For any door state (open/closed), door-sensor publishes correct bool and exits 0.
	t.Run("door-sensor", func(t *testing.T) {
		doorCases := []struct {
			flag     string
			expected bool
		}{
			{"--open", true},
			{"--closed", false},
			{"--open", true},   // verify reproducibility
			{"--closed", false},
		}

		for _, tc := range doorCases {
			valAddr, mockVal := startMockVALServer(t)
			_, stderr, code := runBinary(t, doorBin,
				tc.flag,
				"--broker-addr=http://"+valAddr,
			)
			if code != 0 {
				t.Errorf("flag=%s: expected exit 0, got %d (stderr=%q)", tc.flag, code, stderr)
				continue
			}

			entries := mockVal.getEntries()
			doorEntry := findEntry(entries, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
			if doorEntry == nil {
				t.Errorf("flag=%s: IsOpen not published", tc.flag)
				continue
			}
			boolDP, ok := doorEntry.Value.Value.(*valpb.Datapoint_Bool)
			if !ok {
				t.Errorf("flag=%s: IsOpen not a bool", tc.flag)
				continue
			}
			if boolDP.Bool != tc.expected {
				t.Errorf("flag=%s: IsOpen=%v, want %v", tc.flag, boolDP.Bool, tc.expected)
			}
		}
	})
}

// TS-09-E4 (via Go): sensor binaries exit non-zero when DATA_BROKER is unreachable.
// This covers the same requirement as Rust's test_*_unreachable_broker tests
// but from the Go integration test side (09-REQ-1.E2, 09-REQ-2.E2, 09-REQ-3.E2).
func TestSensorsUnreachableBroker(t *testing.T) {
	unreachableAddr := "http://localhost:59998" // nothing listening here

	tests := []struct {
		name string
		bin  string
		args []string
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
