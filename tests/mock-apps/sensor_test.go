package mockapps_test

import (
	"context"
	"net"
	"os/exec"
	"path/filepath"
	"sync"
	"testing"

	kuksapb "github.com/rhadp/parking-fee-service/gen/kuksa/val/v1"
	"google.golang.org/grpc"
)

// ---------------------------------------------------------------------------
// Stub kuksa.val.v1 VAL gRPC server for sensor integration testing.
// Records all Set RPC calls so tests can verify published values.
// See docs/errata/09_mock_apps_sensor_proto_compat.md for context.
// ---------------------------------------------------------------------------

// setCall records a single Set RPC invocation.
type setCall struct {
	Path  string
	Value *kuksapb.Datapoint
}

// stubVALServer implements the kuksa.val.v1.VAL service, recording Set calls.
type stubVALServer struct {
	kuksapb.UnimplementedVALServer
	mu    sync.Mutex
	calls []setCall
}

func (s *stubVALServer) Set(_ context.Context, req *kuksapb.SetRequest) (*kuksapb.SetResponse, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	for _, u := range req.GetUpdates() {
		entry := u.GetEntry()
		if entry != nil {
			s.calls = append(s.calls, setCall{
				Path:  entry.GetPath(),
				Value: entry.GetValue(),
			})
		}
	}
	return &kuksapb.SetResponse{}, nil
}

func (s *stubVALServer) getCalls() []setCall {
	s.mu.Lock()
	defer s.mu.Unlock()
	out := make([]setCall, len(s.calls))
	copy(out, s.calls)
	return out
}

// startStubVALServer starts a stub kuksa.val.v1 gRPC server on a random port.
// Returns the server address and the stub for call inspection.
func startStubVALServer(t *testing.T) (string, *stubVALServer) {
	t.Helper()
	lis, err := net.Listen("tcp", "localhost:0")
	if err != nil {
		t.Fatal(err)
	}

	stub := &stubVALServer{}
	srv := grpc.NewServer()
	kuksapb.RegisterVALServer(srv, stub)
	go func() { _ = srv.Serve(lis) }()
	t.Cleanup(func() { srv.Stop() })

	return lis.Addr().String(), stub
}

// buildRustBinary compiles a Rust binary from rhivos/mock-sensors and returns
// the path to the compiled executable. Uses cargo build --release for the
// specific binary target.
func buildRustBinary(t *testing.T, name string) string {
	t.Helper()
	root := repoRoot(t)
	sensorDir := filepath.Join(root, "rhivos")

	cmd := exec.Command("cargo", "build", "-p", "mock-sensors", "--bin", name)
	cmd.Dir = sensorDir
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build %s: %v\n%s", name, err, output)
	}

	// Find the binary in the target directory.
	binary := filepath.Join(sensorDir, "target", "debug", name)
	return binary
}

// ---------------------------------------------------------------------------
// TS-09-1: Location Sensor Publishes Coordinates
// Requirements: 09-REQ-1.1, 09-REQ-1.2, 09-REQ-10.1, 09-REQ-10.2
// ---------------------------------------------------------------------------

func TestLocationSensor(t *testing.T) {
	addr, stub := startStubVALServer(t)
	binary := buildRustBinary(t, "location-sensor")

	stdout, stderr, exitCode := runBinary(t, binary,
		"--lat=48.1351",
		"--lon=11.5820",
		"--broker-addr=http://"+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	calls := stub.getCalls()
	if len(calls) != 2 {
		t.Fatalf("expected 2 Set calls (lat + lon), got %d", len(calls))
	}

	// Verify latitude
	foundLat := false
	foundLon := false
	for _, c := range calls {
		switch c.Path {
		case "Vehicle.CurrentLocation.Latitude":
			foundLat = true
			if v := c.Value.GetDoubleValue(); v != 48.1351 {
				t.Errorf("expected Latitude 48.1351, got %f", v)
			}
		case "Vehicle.CurrentLocation.Longitude":
			foundLon = true
			if v := c.Value.GetDoubleValue(); v != 11.5820 {
				t.Errorf("expected Longitude 11.5820, got %f", v)
			}
		default:
			t.Errorf("unexpected VSS path: %s", c.Path)
		}
	}
	if !foundLat {
		t.Error("missing Set call for Vehicle.CurrentLocation.Latitude")
	}
	if !foundLon {
		t.Error("missing Set call for Vehicle.CurrentLocation.Longitude")
	}
}

// ---------------------------------------------------------------------------
// TS-09-2: Speed Sensor Publishes Speed
// Requirements: 09-REQ-2.1, 09-REQ-2.2
// ---------------------------------------------------------------------------

func TestSpeedSensor(t *testing.T) {
	addr, stub := startStubVALServer(t)
	binary := buildRustBinary(t, "speed-sensor")

	stdout, stderr, exitCode := runBinary(t, binary,
		"--speed=0.0",
		"--broker-addr=http://"+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	calls := stub.getCalls()
	if len(calls) != 1 {
		t.Fatalf("expected 1 Set call (speed), got %d", len(calls))
	}

	c := calls[0]
	if c.Path != "Vehicle.Speed" {
		t.Errorf("expected path 'Vehicle.Speed', got '%s'", c.Path)
	}

	if v := c.Value.GetFloatValue(); v != 0.0 {
		t.Errorf("expected speed 0.0, got %f", v)
	}
}

// ---------------------------------------------------------------------------
// TS-09-3: Door Sensor Publishes Open State
// Requirement: 09-REQ-3.1, 09-REQ-3.2
// ---------------------------------------------------------------------------

func TestDoorSensorOpen(t *testing.T) {
	addr, stub := startStubVALServer(t)
	binary := buildRustBinary(t, "door-sensor")

	stdout, stderr, exitCode := runBinary(t, binary,
		"--open",
		"--broker-addr=http://"+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	calls := stub.getCalls()
	if len(calls) != 1 {
		t.Fatalf("expected 1 Set call (door open), got %d", len(calls))
	}

	c := calls[0]
	if c.Path != "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen" {
		t.Errorf("expected path 'Vehicle.Cabin.Door.Row1.DriverSide.IsOpen', got '%s'", c.Path)
	}

	if v := c.Value.GetBoolValue(); !v {
		t.Errorf("expected IsOpen=true for --open, got %v", v)
	}
}

// ---------------------------------------------------------------------------
// TS-09-4: Door Sensor Publishes Closed State
// Requirement: 09-REQ-3.1
// ---------------------------------------------------------------------------

func TestDoorSensorClosed(t *testing.T) {
	addr, stub := startStubVALServer(t)
	binary := buildRustBinary(t, "door-sensor")

	stdout, stderr, exitCode := runBinary(t, binary,
		"--closed",
		"--broker-addr=http://"+addr,
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}

	calls := stub.getCalls()
	if len(calls) != 1 {
		t.Fatalf("expected 1 Set call (door closed), got %d", len(calls))
	}

	c := calls[0]
	if c.Path != "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen" {
		t.Errorf("expected path 'Vehicle.Cabin.Door.Row1.DriverSide.IsOpen', got '%s'", c.Path)
	}

	// --closed should publish false
	if v := c.Value.GetBoolValue(); v {
		t.Errorf("expected IsOpen=false for --closed, got %v", v)
	}
}

// ---------------------------------------------------------------------------
// TS-09-E4 (extended): Sensor Unreachable DATA_BROKER (Go-side)
// Requirements: 09-REQ-1.E2, 09-REQ-2.E2, 09-REQ-3.E2
// ---------------------------------------------------------------------------

func TestSensorsUnreachableBroker(t *testing.T) {
	sensors := []struct {
		name string
		args []string
	}{
		{"location-sensor", []string{"--lat=48.13", "--lon=11.58"}},
		{"speed-sensor", []string{"--speed=50.0"}},
		{"door-sensor", []string{"--open"}},
	}

	for _, s := range sensors {
		t.Run(s.name, func(t *testing.T) {
			binary := buildRustBinary(t, s.name)
			args := append(s.args, "--broker-addr=http://localhost:19999")
			_, stderr, exitCode := runBinary(t, binary, args...)

			if exitCode != 1 {
				t.Fatalf("expected exit code 1 when broker unreachable, got %d", exitCode)
			}

			if len(stderr) == 0 {
				t.Error("expected error message on stderr when broker is unreachable")
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-09-SMOKE-1: End-to-End Sensor to DATA_BROKER
// Smoke test: all three sensors publish via stub server.
// ---------------------------------------------------------------------------

func TestSensorSmoke(t *testing.T) {
	addr, stub := startStubVALServer(t)

	// Run all three sensors sequentially against the same stub.
	locBin := buildRustBinary(t, "location-sensor")
	_, stderr, exitCode := runBinary(t, locBin,
		"--lat=48.13", "--lon=11.58", "--broker-addr=http://"+addr)
	if exitCode != 0 {
		t.Fatalf("location-sensor failed: exit %d, stderr: %s", exitCode, stderr)
	}

	speedBin := buildRustBinary(t, "speed-sensor")
	_, stderr, exitCode = runBinary(t, speedBin,
		"--speed=0.0", "--broker-addr=http://"+addr)
	if exitCode != 0 {
		t.Fatalf("speed-sensor failed: exit %d, stderr: %s", exitCode, stderr)
	}

	doorBin := buildRustBinary(t, "door-sensor")
	_, stderr, exitCode = runBinary(t, doorBin,
		"--closed", "--broker-addr=http://"+addr)
	if exitCode != 0 {
		t.Fatalf("door-sensor failed: exit %d, stderr: %s", exitCode, stderr)
	}

	// Verify all expected calls were made.
	calls := stub.getCalls()

	paths := make(map[string]bool)
	for _, c := range calls {
		paths[c.Path] = true
	}

	expected := []string{
		"Vehicle.CurrentLocation.Latitude",
		"Vehicle.CurrentLocation.Longitude",
		"Vehicle.Speed",
		"Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
	}
	for _, p := range expected {
		if !paths[p] {
			t.Errorf("missing Set call for %s", p)
		}
	}
}
