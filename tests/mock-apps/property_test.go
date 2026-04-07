package mockapps_test

import (
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"syscall"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TS-09-P2: CLI Argument Validation
// Property 2 from design.md
// Validates: 09-REQ-1.E1, 09-REQ-2.E1, 09-REQ-3.E1, 09-REQ-4.E1, 09-REQ-5.E1
// Description: For any invocation with missing required arguments, exit code
// is 1 and stderr is non-empty.
// ---------------------------------------------------------------------------

func TestPropertyCLIArgumentValidation(t *testing.T) {
	root := findRepoRoot(t)

	// Build sensor binaries.
	locationBin := buildRustBinary(t, root, "location-sensor")
	speedBin := buildRustBinary(t, root, "speed-sensor")
	doorBin := buildRustBinary(t, root, "door-sensor")

	// Build Go CLI binaries.
	parkingApp := parkingAppBinary(t)
	companionApp := companionBinary(t)

	env := baseEnv()

	tests := []struct {
		name string
		bin  string
		args []string
	}{
		// location-sensor: missing --lon
		{"location-sensor missing --lon", locationBin, []string{"--lat=48.0"}},
		// location-sensor: missing --lat
		{"location-sensor missing --lat", locationBin, []string{"--lon=11.0"}},
		// location-sensor: no args
		{"location-sensor no args", locationBin, nil},
		// speed-sensor: no args
		{"speed-sensor no args", speedBin, nil},
		// door-sensor: no args
		{"door-sensor no args", doorBin, nil},
		// parking-app-cli: lookup missing args
		{"parking-app-cli lookup no args", parkingApp, []string{"lookup"}},
		// parking-app-cli: adapter-info missing operator-id
		{"parking-app-cli adapter-info no args", parkingApp, []string{"adapter-info"}},
		// parking-app-cli: install missing args
		{"parking-app-cli install no args", parkingApp, []string{"install"}},
		// companion-app-cli: lock missing vin
		{"companion-app-cli lock missing vin", companionApp, []string{"lock", "--token=x"}},
		// companion-app-cli: lock missing token
		{"companion-app-cli lock missing token", companionApp, []string{"lock", "--vin=X"}},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			_, stderr, exitCode := runBinary(t, tc.bin, tc.args, env)

			if exitCode != 1 {
				t.Errorf("expected exit code 1, got %d", exitCode)
			}
			if len(stderr) == 0 {
				t.Error("expected non-empty stderr")
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-09-P3: Parking Operator Session Integrity (integration variant)
// Property 4 from design.md
// Validates: 09-REQ-8.2, 09-REQ-8.3, 09-REQ-8.5
// Description: For any start-stop sequence, duration and total_amount are
// calculated correctly.
// Note: Also tested in mock/parking-operator/server/server_test.go. This
// variant exercises the handler via httptest for additional coverage.
// ---------------------------------------------------------------------------

func TestPropertySessionIntegrity(t *testing.T) {
	root := findRepoRoot(t)
	moduleDir := filepath.Join(root, "mock", "parking-operator")
	bin := buildGoBinary(t, moduleDir, "parking-operator")

	port := getFreeTestPort(t)
	addr := fmt.Sprintf("http://localhost:%d", port)

	// Start parking-operator in background.
	cmd := startParkingOperator(t, bin, port)
	_ = cmd // cleanup handled by t.Cleanup

	if !waitForReady(addr+"/parking/status/probe", 5*time.Second) {
		t.Fatal("parking-operator did not start within timeout")
	}

	testCases := []struct {
		startTS  int64
		stopTS   int64
		wantDur  int64
		wantAmt  float64
	}{
		{1700000000, 1700003600, 3600, 2.50},     // 1 hour
		{1700000000, 1700007200, 7200, 5.00},     // 2 hours
		{1700000000, 1700001800, 1800, 1.25},     // 30 minutes
		{1700000000, 1700000360, 360, 0.25},      // 6 minutes
		{1700000000, 1700000001, 1, 2.50 / 3600}, // 1 second
	}

	for i, tc := range testCases {
		t.Run(fmt.Sprintf("case_%d_dur_%ds", i, tc.wantDur), func(t *testing.T) {
			startBody := fmt.Sprintf(
				`{"vehicle_id":"V%d","zone_id":"z1","timestamp":%d}`, i, tc.startTS)
			startResp, err := http.Post(addr+"/parking/start", "application/json",
				strings.NewReader(startBody))
			if err != nil {
				t.Fatalf("start failed: %v", err)
			}
			defer startResp.Body.Close()

			if startResp.StatusCode != 200 {
				t.Fatalf("start: expected 200, got %d", startResp.StatusCode)
			}

			var startResult map[string]any
			startData, _ := io.ReadAll(startResp.Body)
			json.Unmarshal(startData, &startResult)

			sessionID, _ := startResult["session_id"].(string)

			stopBody := fmt.Sprintf(`{"session_id":"%s","timestamp":%d}`, sessionID, tc.stopTS)
			stopResp, err := http.Post(addr+"/parking/stop", "application/json",
				strings.NewReader(stopBody))
			if err != nil {
				t.Fatalf("stop failed: %v", err)
			}
			defer stopResp.Body.Close()

			if stopResp.StatusCode != 200 {
				t.Fatalf("stop: expected 200, got %d", stopResp.StatusCode)
			}

			var stopResult map[string]any
			stopData, _ := io.ReadAll(stopResp.Body)
			json.Unmarshal(stopData, &stopResult)

			duration, _ := stopResult["duration_seconds"].(float64)
			if int64(duration) != tc.wantDur {
				t.Errorf("duration: expected %d, got %v", tc.wantDur, duration)
			}

			totalAmount, _ := stopResult["total_amount"].(float64)
			diff := totalAmount - tc.wantAmt
			if diff < -0.01 || diff > 0.01 {
				t.Errorf("total_amount: expected %.4f, got %.4f", tc.wantAmt, totalAmount)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// TS-09-P4 / TS-09-P5: Parking Operator Session Uniqueness (integration)
// Property 5 from design.md
// Validates: 09-REQ-8.2, 09-REQ-8.5
// Description: For any number of start requests, all generated session_ids
// are unique UUIDs.
// ---------------------------------------------------------------------------

func TestPropertySessionUniqueness(t *testing.T) {
	root := findRepoRoot(t)
	moduleDir := filepath.Join(root, "mock", "parking-operator")
	bin := buildGoBinary(t, moduleDir, "parking-operator")

	port := getFreeTestPort(t)
	addr := fmt.Sprintf("http://localhost:%d", port)

	cmd := startParkingOperator(t, bin, port)
	_ = cmd

	if !waitForReady(addr+"/parking/status/probe", 5*time.Second) {
		t.Fatal("parking-operator did not start within timeout")
	}

	uuidRegex := regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`)
	ids := make(map[string]bool)

	for i := 0; i < 100; i++ {
		body := fmt.Sprintf(
			`{"vehicle_id":"V%d","zone_id":"z1","timestamp":%d}`, i, i+1)
		resp, err := http.Post(addr+"/parking/start", "application/json",
			strings.NewReader(body))
		if err != nil {
			t.Fatalf("request %d failed: %v", i, err)
		}

		if resp.StatusCode != 200 {
			resp.Body.Close()
			t.Fatalf("request %d: expected 200, got %d", i, resp.StatusCode)
		}

		var result map[string]any
		data, _ := io.ReadAll(resp.Body)
		resp.Body.Close()
		json.Unmarshal(data, &result)

		sessionID, _ := result["session_id"].(string)
		if !uuidRegex.MatchString(sessionID) {
			t.Errorf("session %d: %q is not a valid UUID", i, sessionID)
		}
		if ids[sessionID] {
			t.Errorf("session %d: duplicate session_id %q", i, sessionID)
		}
		ids[sessionID] = true
	}

	if len(ids) != 100 {
		t.Errorf("expected 100 unique session IDs, got %d", len(ids))
	}
}

// ---------------------------------------------------------------------------
// TS-09-P6: Bearer Token Enforcement (integration variant)
// Property 6 from design.md
// Validates: 09-REQ-7.4, 09-REQ-7.E2
// Note: Primary test is TestBearerTokenEnforcement in companion_app_test.go.
// This variant tests with environment variable token source.
// ---------------------------------------------------------------------------

func TestPropertyBearerTokenViaEnv(t *testing.T) {
	mock := newMockHTTPServer(t, 200, map[string]any{
		"command_id": "x",
		"status":     "pending",
	})

	bin := companionBinary(t)

	// Test token via CLOUD_GATEWAY_TOKEN env var.
	env := append(baseEnv(), "CLOUD_GATEWAY_TOKEN=env-secret-token")

	_, _, exitCode := runBinary(t, bin, []string{
		"lock",
		"--vin=VIN001",
		"--gateway-addr=" + mock.URL(),
	}, env)

	if exitCode != 0 {
		t.Fatalf("expected exit 0 with env token, got %d", exitCode)
	}

	reqs := mock.getRequests()
	if len(reqs) == 0 {
		t.Fatal("no requests received")
	}

	authHeader := reqs[0].Headers.Get("Authorization")
	if authHeader != "Bearer env-secret-token" {
		t.Errorf("expected Authorization 'Bearer env-secret-token', got %q", authHeader)
	}

	// Verify flag takes precedence over env.
	mock2 := newMockHTTPServer(t, 200, map[string]any{
		"command_id": "y",
		"status":     "pending",
	})

	env2 := append(baseEnv(), "CLOUD_GATEWAY_TOKEN=env-token")

	_, _, exitCode2 := runBinary(t, bin, []string{
		"lock",
		"--vin=VIN001",
		"--token=flag-token",
		"--gateway-addr=" + mock2.URL(),
	}, env2)

	if exitCode2 != 0 {
		t.Fatalf("expected exit 0, got %d", exitCode2)
	}

	reqs2 := mock2.getRequests()
	if len(reqs2) == 0 {
		t.Fatal("no requests received")
	}

	authHeader2 := reqs2[0].Headers.Get("Authorization")
	if authHeader2 != "Bearer flag-token" {
		t.Errorf("expected flag token precedence: 'Bearer flag-token', got %q", authHeader2)
	}
}

// ---------------------------------------------------------------------------
// TS-09-P1: Sensor Publish-and-Exit (integration)
// Property 1 from design.md
// Validates: 09-REQ-1.1, 09-REQ-2.1, 09-REQ-3.1
// Requires: Running DATA_BROKER at localhost:55556
// ---------------------------------------------------------------------------

func TestPropertySensorPublishAndExit(t *testing.T) {
	if os.Getenv("DATABROKER_ADDR") == "" {
		// Try default address to see if DATA_BROKER is available.
		conn, err := net.DialTimeout("tcp", "localhost:55556", 2*time.Second)
		if err != nil {
			t.Skip("DATA_BROKER not available, skipping sensor integration test")
		}
		conn.Close()
	}

	root := findRepoRoot(t)
	locationBin := buildRustBinary(t, root, "location-sensor")
	speedBin := buildRustBinary(t, root, "speed-sensor")
	doorBin := buildRustBinary(t, root, "door-sensor")

	env := baseEnv()

	// Test location-sensor with various coordinates.
	locTests := []struct {
		lat, lon string
	}{
		{"48.1351", "11.5820"},
		{"-33.8688", "151.2093"},
		{"0.0", "0.0"},
	}

	for _, tc := range locTests {
		t.Run(fmt.Sprintf("location_%s_%s", tc.lat, tc.lon), func(t *testing.T) {
			_, stderr, exitCode := runBinary(t, locationBin, []string{
				"--lat=" + tc.lat,
				"--lon=" + tc.lon,
			}, env)

			if exitCode != 0 {
				t.Errorf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
			}
		})
	}

	// Test speed-sensor.
	speedTests := []string{"0.0", "60.5", "120.0"}
	for _, speed := range speedTests {
		t.Run("speed_"+speed, func(t *testing.T) {
			_, stderr, exitCode := runBinary(t, speedBin, []string{
				"--speed=" + speed,
			}, env)

			if exitCode != 0 {
				t.Errorf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
			}
		})
	}

	// Test door-sensor.
	doorTests := []struct {
		flag string
	}{
		{"--open"},
		{"--closed"},
	}
	for _, tc := range doorTests {
		t.Run("door_"+tc.flag, func(t *testing.T) {
			_, stderr, exitCode := runBinary(t, doorBin, []string{tc.flag}, env)

			if exitCode != 0 {
				t.Errorf("expected exit 0, got %d\nstderr: %s", exitCode, stderr)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// buildRustBinary locates a pre-built Rust binary from the target directory,
// or builds it. Sensor binaries are in rhivos/mock-sensors/.
func buildRustBinary(t *testing.T, repoRoot, binaryName string) string {
	t.Helper()

	// Try to find pre-built binary in target/debug.
	debugPath := filepath.Join(repoRoot, "rhivos", "target", "debug", binaryName)
	if _, err := os.Stat(debugPath); err == nil {
		return debugPath
	}

	// Build if not found.
	cmd := exec.Command("cargo", "build", "-p", "mock-sensors")
	cmd.Dir = filepath.Join(repoRoot, "rhivos")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build %s: %v\n%s", binaryName, err, string(out))
	}

	if _, err := os.Stat(debugPath); err != nil {
		t.Fatalf("binary %s not found after build at %s", binaryName, debugPath)
	}
	return debugPath
}

// startParkingOperator starts the parking-operator binary and registers cleanup.
func startParkingOperator(t *testing.T, bin string, port int) *exec.Cmd {
	t.Helper()

	cmd := exec.Command(bin, "serve", fmt.Sprintf("--port=%d", port))
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		t.Fatalf("failed to start parking-operator: %v", err)
	}

	t.Cleanup(func() {
		_ = cmd.Process.Signal(syscall.SIGTERM)
		_ = cmd.Wait()
	})

	return cmd
}

