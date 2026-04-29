package mockapps_test

import (
	"context"
	"fmt"
	"net"
	"os/exec"
	"sync"
	"testing"

	kuksav2 "parking-fee-service/tests/mock-apps/internal/kuksav2"

	"google.golang.org/grpc"
)

// ---------------------------------------------------------------------------
// Stub kuksa.val.v2 gRPC server for sensor integration tests
// ---------------------------------------------------------------------------

// stubValServer implements the kuksa.val.v2.VAL service, capturing
// PublishValue calls for assertion.
type stubValServer struct {
	kuksav2.UnimplementedVALServer
	mu            sync.Mutex
	publishedVals []publishedVal
}

type publishedVal struct {
	Path  string
	Value *kuksav2.Value
}

func (s *stubValServer) PublishValue(_ context.Context, req *kuksav2.PublishValueRequest) (*kuksav2.PublishValueResponse, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	var path string
	if sid := req.GetSignalId(); sid != nil {
		if p := sid.GetPath(); p != "" {
			path = p
		}
	}

	var val *kuksav2.Value
	if dp := req.GetDataPoint(); dp != nil {
		val = dp.GetValue()
	}

	s.publishedVals = append(s.publishedVals, publishedVal{Path: path, Value: val})
	return &kuksav2.PublishValueResponse{}, nil
}

func (s *stubValServer) getPublished() []publishedVal {
	s.mu.Lock()
	defer s.mu.Unlock()
	cp := make([]publishedVal, len(s.publishedVals))
	copy(cp, s.publishedVals)
	return cp
}

// startStubDataBroker starts a stub kuksa.val.v2 gRPC server on a random port
// and returns the server, its address, and a cleanup function.
func startStubDataBroker(t *testing.T) (*stubValServer, string) {
	t.Helper()

	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen: %v", err)
	}

	stub := &stubValServer{}
	srv := grpc.NewServer()
	kuksav2.RegisterVALServer(srv, stub)

	go func() {
		if err := srv.Serve(lis); err != nil {
			// Server was stopped — expected during cleanup.
		}
	}()

	t.Cleanup(func() { srv.GracefulStop() })

	addr := fmt.Sprintf("http://%s", lis.Addr().String())
	return stub, addr
}

// ---------------------------------------------------------------------------
// TS-09-1: Location Sensor Publishes Coordinates
// Requirement: 09-REQ-1.1, 09-REQ-1.2, 09-REQ-10.1, 09-REQ-10.2
// ---------------------------------------------------------------------------

func TestLocationSensor(t *testing.T) {
	stub, addr := startStubDataBroker(t)

	root := findRepoRoot(t)
	bin := buildSensorBinary(t, root, "location-sensor")

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"--lat=48.1351",
		"--lon=11.5820",
		"--broker-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	published := stub.getPublished()
	if len(published) != 2 {
		t.Fatalf("expected 2 published values, got %d", len(published))
	}

	// Check Latitude
	if published[0].Path != "Vehicle.CurrentLocation.Latitude" {
		t.Errorf("expected Latitude path, got %q", published[0].Path)
	}
	if v := published[0].Value.GetDouble(); v != 48.1351 {
		t.Errorf("expected Latitude=48.1351, got %v", v)
	}

	// Check Longitude
	if published[1].Path != "Vehicle.CurrentLocation.Longitude" {
		t.Errorf("expected Longitude path, got %q", published[1].Path)
	}
	if v := published[1].Value.GetDouble(); v != 11.5820 {
		t.Errorf("expected Longitude=11.5820, got %v", v)
	}
}

// ---------------------------------------------------------------------------
// TS-09-2: Speed Sensor Publishes Speed
// Requirement: 09-REQ-2.1, 09-REQ-2.2
// ---------------------------------------------------------------------------

func TestSpeedSensor(t *testing.T) {
	stub, addr := startStubDataBroker(t)

	root := findRepoRoot(t)
	bin := buildSensorBinary(t, root, "speed-sensor")

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"--speed=0.0",
		"--broker-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	published := stub.getPublished()
	if len(published) != 1 {
		t.Fatalf("expected 1 published value, got %d", len(published))
	}

	if published[0].Path != "Vehicle.Speed" {
		t.Errorf("expected Vehicle.Speed path, got %q", published[0].Path)
	}
	if v := published[0].Value.GetFloat(); v != 0.0 {
		t.Errorf("expected Speed=0.0, got %v", v)
	}
}

// ---------------------------------------------------------------------------
// TS-09-3: Door Sensor Publishes Open State
// Requirement: 09-REQ-3.1, 09-REQ-3.2
// ---------------------------------------------------------------------------

func TestDoorSensorOpen(t *testing.T) {
	stub, addr := startStubDataBroker(t)

	root := findRepoRoot(t)
	bin := buildSensorBinary(t, root, "door-sensor")

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"--open",
		"--broker-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	published := stub.getPublished()
	if len(published) != 1 {
		t.Fatalf("expected 1 published value, got %d", len(published))
	}

	if published[0].Path != "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen" {
		t.Errorf("expected IsOpen path, got %q", published[0].Path)
	}
	if v := published[0].Value.GetBool(); v != true {
		t.Errorf("expected IsOpen=true, got %v", v)
	}
}

// ---------------------------------------------------------------------------
// TS-09-4: Door Sensor Publishes Closed State
// Requirement: 09-REQ-3.1
// ---------------------------------------------------------------------------

func TestDoorSensorClosed(t *testing.T) {
	stub, addr := startStubDataBroker(t)

	root := findRepoRoot(t)
	bin := buildSensorBinary(t, root, "door-sensor")

	stdout, stderr, exitCode := runBinary(t, bin, []string{
		"--closed",
		"--broker-addr=" + addr,
	}, baseEnv())

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	published := stub.getPublished()
	if len(published) != 1 {
		t.Fatalf("expected 1 published value, got %d", len(published))
	}

	if published[0].Path != "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen" {
		t.Errorf("expected IsOpen path, got %q", published[0].Path)
	}
	if v := published[0].Value.GetBool(); v != false {
		t.Errorf("expected IsOpen=false, got %v", v)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E4 (sensor integration variant): Sensors Unreachable DATA_BROKER
// Requirement: 09-REQ-1.E2, 09-REQ-2.E2, 09-REQ-3.E2
// ---------------------------------------------------------------------------

func TestSensorsUnreachableBroker(t *testing.T) {
	root := findRepoRoot(t)

	tests := []struct {
		name string
		args []string
	}{
		{"location-sensor", []string{"--lat=48.13", "--lon=11.58", "--broker-addr=http://localhost:19999"}},
		{"speed-sensor", []string{"--speed=60.0", "--broker-addr=http://localhost:19999"}},
		{"door-sensor", []string{"--open", "--broker-addr=http://localhost:19999"}},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			bin := buildSensorBinary(t, root, tc.name)
			env := baseEnv()
			// Ensure DATABROKER_ADDR doesn't override
			env = append(env, "DATABROKER_ADDR=")

			_, stderr, exitCode := runBinary(t, bin, tc.args, env)

			if exitCode != 1 {
				t.Errorf("expected exit 1, got %d", exitCode)
			}
			if len(stderr) == 0 {
				t.Error("expected error on stderr")
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-09-SMOKE-1: End-to-End Sensor to DATA_BROKER
// All three mock sensors publish values and a stub server confirms receipt.
// ---------------------------------------------------------------------------

func TestSensorSmoke(t *testing.T) {
	stub, addr := startStubDataBroker(t)
	root := findRepoRoot(t)

	// Location sensor
	locBin := buildSensorBinary(t, root, "location-sensor")
	_, stderr, exitCode := runBinary(t, locBin, []string{
		"--lat=48.13", "--lon=11.58", "--broker-addr=" + addr,
	}, baseEnv())
	if exitCode != 0 {
		t.Fatalf("location-sensor failed: exit %d, stderr: %s", exitCode, stderr)
	}

	// Speed sensor
	speedBin := buildSensorBinary(t, root, "speed-sensor")
	_, stderr, exitCode = runBinary(t, speedBin, []string{
		"--speed=0.0", "--broker-addr=" + addr,
	}, baseEnv())
	if exitCode != 0 {
		t.Fatalf("speed-sensor failed: exit %d, stderr: %s", exitCode, stderr)
	}

	// Door sensor
	doorBin := buildSensorBinary(t, root, "door-sensor")
	_, stderr, exitCode = runBinary(t, doorBin, []string{
		"--closed", "--broker-addr=" + addr,
	}, baseEnv())
	if exitCode != 0 {
		t.Fatalf("door-sensor failed: exit %d, stderr: %s", exitCode, stderr)
	}

	published := stub.getPublished()
	// location-sensor publishes 2 values, speed 1, door 1 = 4 total
	if len(published) != 4 {
		t.Fatalf("expected 4 published values in smoke test, got %d", len(published))
	}

	// Verify paths
	paths := make([]string, len(published))
	for i, p := range published {
		paths[i] = p.Path
	}
	expectedPaths := []string{
		"Vehicle.CurrentLocation.Latitude",
		"Vehicle.CurrentLocation.Longitude",
		"Vehicle.Speed",
		"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
	}
	for i, expected := range expectedPaths {
		if paths[i] != expected {
			t.Errorf("published[%d].Path = %q, want %q", i, paths[i], expected)
		}
	}
}

// ---------------------------------------------------------------------------
// TS-09-P1: Sensor Publish-and-Exit (property test)
// Property 1 from design.md
// Requirement: 09-REQ-1.1, 09-REQ-2.1, 09-REQ-3.1
// ---------------------------------------------------------------------------

func TestSensorPublishProperty(t *testing.T) {
	root := findRepoRoot(t)

	// Test location sensor with various lat/lon combinations
	t.Run("location_sensor", func(t *testing.T) {
		locBin := buildSensorBinary(t, root, "location-sensor")
		latLons := [][2]float64{
			{0.0, 0.0},
			{48.1351, 11.5820},
			{-33.8688, 151.2093}, // Sydney
			{90.0, 180.0},       // extremes
			{-90.0, -180.0},     // extremes
			{51.5074, -0.1278},  // London
			{40.7128, -74.0060}, // New York
			{35.6762, 139.6503}, // Tokyo
			{1.3521, 103.8198},  // Singapore
			{-22.9068, -43.1729}, // Rio
		}

		for _, ll := range latLons {
			stub, addr := startStubDataBroker(t)
			latStr := fmt.Sprintf("--lat=%g", ll[0])
			lonStr := fmt.Sprintf("--lon=%g", ll[1])

			_, stderr, exitCode := runBinary(t, locBin, []string{
				latStr, lonStr, "--broker-addr=" + addr,
			}, baseEnv())

			if exitCode != 0 {
				t.Errorf("lat=%g lon=%g: exit %d, stderr: %s", ll[0], ll[1], exitCode, stderr)
				continue
			}

			published := stub.getPublished()
			if len(published) != 2 {
				t.Errorf("lat=%g lon=%g: expected 2 values, got %d", ll[0], ll[1], len(published))
				continue
			}

			if published[0].Path != "Vehicle.CurrentLocation.Latitude" {
				t.Errorf("lat=%g: wrong path %q", ll[0], published[0].Path)
			}
			if published[1].Path != "Vehicle.CurrentLocation.Longitude" {
				t.Errorf("lon=%g: wrong path %q", ll[1], published[1].Path)
			}
		}
	})

	// Test speed sensor with various speed values
	t.Run("speed_sensor", func(t *testing.T) {
		speedBin := buildSensorBinary(t, root, "speed-sensor")
		speeds := []float32{0.0, 30.0, 60.5, 120.0, 200.0, 0.1, 999.9, 1.0}

		for _, spd := range speeds {
			stub, addr := startStubDataBroker(t)
			spdStr := fmt.Sprintf("--speed=%g", spd)

			_, stderr, exitCode := runBinary(t, speedBin, []string{
				spdStr, "--broker-addr=" + addr,
			}, baseEnv())

			if exitCode != 0 {
				t.Errorf("speed=%g: exit %d, stderr: %s", spd, exitCode, stderr)
				continue
			}

			published := stub.getPublished()
			if len(published) != 1 {
				t.Errorf("speed=%g: expected 1 value, got %d", spd, len(published))
				continue
			}
			if published[0].Path != "Vehicle.Speed" {
				t.Errorf("speed=%g: wrong path %q", spd, published[0].Path)
			}
		}
	})

	// Test door sensor with both states
	t.Run("door_sensor", func(t *testing.T) {
		doorBin := buildSensorBinary(t, root, "door-sensor")

		for _, state := range []struct {
			flag string
			want bool
		}{
			{"--open", true},
			{"--closed", false},
		} {
			stub, addr := startStubDataBroker(t)

			_, stderr, exitCode := runBinary(t, doorBin, []string{
				state.flag, "--broker-addr=" + addr,
			}, baseEnv())

			if exitCode != 0 {
				t.Errorf("door %s: exit %d, stderr: %s", state.flag, exitCode, stderr)
				continue
			}

			published := stub.getPublished()
			if len(published) != 1 {
				t.Errorf("door %s: expected 1 value, got %d", state.flag, len(published))
				continue
			}
			if published[0].Path != "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen" {
				t.Errorf("door %s: wrong path %q", state.flag, published[0].Path)
			}
			if published[0].Value.GetBool() != state.want {
				t.Errorf("door %s: expected %v, got %v", state.flag, state.want, published[0].Value.GetBool())
			}
		}
	})
}

// ---------------------------------------------------------------------------
// TS-09-P2 (sensor part): Sensor Argument Validation Property
// Property 2 from design.md
// Requirement: 09-REQ-1.E1, 09-REQ-2.E1, 09-REQ-3.E1
// ---------------------------------------------------------------------------

func TestSensorArgumentValidationProperty(t *testing.T) {
	root := findRepoRoot(t)

	tests := []struct {
		binary string
		desc   string
		args   []string
	}{
		// location-sensor: all missing-arg subsets
		{"location-sensor", "no args", nil},
		{"location-sensor", "missing lon", []string{"--lat=48.13"}},
		{"location-sensor", "missing lat", []string{"--lon=11.58"}},

		// speed-sensor: missing --speed
		{"speed-sensor", "no args", nil},

		// door-sensor: missing state flags
		{"door-sensor", "no args", nil},
		{"door-sensor", "both flags", []string{"--open", "--closed"}},
	}

	for _, tc := range tests {
		t.Run(tc.binary+"_"+tc.desc, func(t *testing.T) {
			bin := buildSensorBinary(t, root, tc.binary)
			_, stderr, exitCode := runBinary(t, bin, tc.args, baseEnv())

			if exitCode != 1 {
				t.Errorf("expected exit 1, got %d", exitCode)
			}
			if len(stderr) == 0 {
				t.Error("expected non-empty stderr")
			}
		})
	}
}

// buildSensorBinaryGo is an alias for buildSensorBinary for readability.
// The actual buildSensorBinary is defined in helpers_test.go.
func init() {
	// Ensure cargo is available for sensor binary builds.
	if _, err := exec.LookPath("cargo"); err != nil {
		// Tests will skip/fail if cargo is not available.
		return
	}
}
