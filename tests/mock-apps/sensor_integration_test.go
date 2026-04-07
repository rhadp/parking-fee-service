package mockapps_test

import (
	"fmt"
	"net"
	"os"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-09-1: Location Sensor Publishes Coordinates
// Requirement: 09-REQ-1.1, 09-REQ-1.2
// ---------------------------------------------------------------------------

func TestLocationSensorPublishes(t *testing.T) {
	brokerAddr := requireDataBroker(t)
	root := findRepoRoot(t)
	bin := buildRustBinary(t, root, "location-sensor")
	env := baseEnv()

	_, stderr, exitCode := runBinary(t, bin, []string{
		"--lat=48.1351",
		"--lon=11.5820",
		"--broker-addr=" + brokerAddr,
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
	}
}

// ---------------------------------------------------------------------------
// TS-09-2: Speed Sensor Publishes Speed
// Requirement: 09-REQ-2.1, 09-REQ-2.2
// ---------------------------------------------------------------------------

func TestSpeedSensorPublishes(t *testing.T) {
	brokerAddr := requireDataBroker(t)
	root := findRepoRoot(t)
	bin := buildRustBinary(t, root, "speed-sensor")
	env := baseEnv()

	_, stderr, exitCode := runBinary(t, bin, []string{
		"--speed=0.0",
		"--broker-addr=" + brokerAddr,
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
	}
}

// ---------------------------------------------------------------------------
// TS-09-3: Door Sensor Publishes Open State
// Requirement: 09-REQ-3.1, 09-REQ-3.2
// ---------------------------------------------------------------------------

func TestDoorSensorPublishesOpen(t *testing.T) {
	brokerAddr := requireDataBroker(t)
	root := findRepoRoot(t)
	bin := buildRustBinary(t, root, "door-sensor")
	env := baseEnv()

	_, stderr, exitCode := runBinary(t, bin, []string{
		"--open",
		"--broker-addr=" + brokerAddr,
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
	}
}

// ---------------------------------------------------------------------------
// TS-09-4: Door Sensor Publishes Closed State
// Requirement: 09-REQ-3.1
// ---------------------------------------------------------------------------

func TestDoorSensorPublishesClosed(t *testing.T) {
	brokerAddr := requireDataBroker(t)
	root := findRepoRoot(t)
	bin := buildRustBinary(t, root, "door-sensor")
	env := baseEnv()

	_, stderr, exitCode := runBinary(t, bin, []string{
		"--closed",
		"--broker-addr=" + brokerAddr,
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
	}
}

// ---------------------------------------------------------------------------
// TS-09-SMOKE-1: End-to-End Sensor to DATA_BROKER
// Requirements: 09-REQ-1.1, 09-REQ-2.1, 09-REQ-3.1, 09-REQ-10.2
// ---------------------------------------------------------------------------

func TestSmokeEndToEndSensors(t *testing.T) {
	brokerAddr := requireDataBroker(t)
	root := findRepoRoot(t)
	env := baseEnv()

	locationBin := buildRustBinary(t, root, "location-sensor")
	speedBin := buildRustBinary(t, root, "speed-sensor")
	doorBin := buildRustBinary(t, root, "door-sensor")

	// Publish location.
	_, stderr, exitCode := runBinary(t, locationBin, []string{
		"--lat=48.13",
		"--lon=11.58",
		"--broker-addr=" + brokerAddr,
	}, env)
	if exitCode != 0 {
		t.Fatalf("location-sensor: exit %d\nstderr: %s", exitCode, stderr)
	}

	// Publish speed.
	_, stderr, exitCode = runBinary(t, speedBin, []string{
		"--speed=0.0",
		"--broker-addr=" + brokerAddr,
	}, env)
	if exitCode != 0 {
		t.Fatalf("speed-sensor: exit %d\nstderr: %s", exitCode, stderr)
	}

	// Publish door closed.
	_, stderr, exitCode = runBinary(t, doorBin, []string{
		"--closed",
		"--broker-addr=" + brokerAddr,
	}, env)
	if exitCode != 0 {
		t.Fatalf("door-sensor: exit %d\nstderr: %s", exitCode, stderr)
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// requireDataBroker checks if DATA_BROKER is available, skipping the test if
// not. Returns the broker address to use.
func requireDataBroker(t *testing.T) string {
	t.Helper()

	addr := os.Getenv("DATABROKER_ADDR")
	if addr == "" {
		addr = "http://localhost:55556"
	}

	// Extract host:port from the address for TCP dial check.
	host := addr
	for _, prefix := range []string{"http://", "https://", "grpc://"} {
		host = trimPrefixIfPresent(host, prefix)
	}

	conn, err := net.DialTimeout("tcp", host, 2*time.Second)
	if err != nil {
		t.Skipf("DATA_BROKER not available at %s: %v", addr, err)
	}
	conn.Close()

	return addr
}

func trimPrefixIfPresent(s, prefix string) string {
	if len(s) >= len(prefix) && s[:len(prefix)] == prefix {
		return s[len(prefix):]
	}
	return s
}

// Ensure fmt is used.
var _ = fmt.Sprintf
