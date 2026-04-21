package setup

import (
	"net"
	"os/exec"
	"strings"
	"testing"
	"time"
)

// skipIfNoPodman skips the test if podman-compose is not available.
func skipIfNoPodman(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("podman-compose"); err != nil {
		t.Skip("podman-compose not found on PATH — skipping infrastructure test")
	}
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not found on PATH — skipping infrastructure test")
	}
}

// TestInfraDownNoContainers verifies infra-down succeeds when no containers
// are running.
// Test Spec: TS-01-E8
// Requirement: 01-REQ-7.E2
func TestInfraDownNoContainers(t *testing.T) {
	skipIfNoPodman(t)

	root := findRepoRoot(t)

	// Ensure nothing is running first.
	downCmd := exec.Command("make", "infra-down")
	downCmd.Dir = root
	downCmd.CombinedOutput() //nolint:errcheck

	// Run infra-down again — should succeed even with nothing running.
	cmd := exec.Command("make", "infra-down")
	cmd.Dir = root
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make infra-down failed when no containers running:\n%s\nerror: %v", out, err)
	}
}

// TestInfraUpDownCycle verifies infrastructure containers can be started and
// stopped in a repeatable cycle.
// Test Spec: TS-01-P3, TS-01-SMOKE-2
// Property: Property 3 (Infrastructure Idempotency)
// Requirements: 01-REQ-7.4, 01-REQ-7.5
func TestInfraUpDownCycle(t *testing.T) {
	skipIfNoPodman(t)

	root := findRepoRoot(t)

	// Run two up/down cycles.
	for i := 1; i <= 2; i++ {
		// Start infrastructure.
		upCmd := exec.Command("make", "infra-up")
		upCmd.Dir = root
		out, err := upCmd.CombinedOutput()
		if err != nil {
			t.Fatalf("cycle %d: make infra-up failed:\n%s\nerror: %v", i, out, err)
		}

		// Allow containers a moment to start listening.
		time.Sleep(3 * time.Second)

		// Stop infrastructure.
		downCmd := exec.Command("make", "infra-down")
		downCmd.Dir = root
		out, err = downCmd.CombinedOutput()
		if err != nil {
			t.Fatalf("cycle %d: make infra-down failed:\n%s\nerror: %v", i, out, err)
		}
	}

	// After final infra-down, no containers should remain.
	checkCmd := exec.Command("podman", "ps", "-q",
		"--filter", "name=nats",
		"--filter", "name=kuksa",
	)
	out, err := checkCmd.CombinedOutput()
	if err != nil {
		t.Logf("podman ps check returned error (may be expected): %v", err)
	}
	if strings.TrimSpace(string(out)) != "" {
		t.Fatalf("containers still running after infra-down: %s", out)
	}
}

// TestPortConflictOnInfraUp verifies infra-up reports a port conflict when
// a required port is already in use.
// Test Spec: TS-01-E7
// Requirement: 01-REQ-7.E1
func TestPortConflictOnInfraUp(t *testing.T) {
	skipIfNoPodman(t)

	root := findRepoRoot(t)

	// Ensure no infra is running.
	downCmd := exec.Command("make", "infra-down")
	downCmd.Dir = root
	downCmd.CombinedOutput() //nolint:errcheck

	// Bind port 4222 to create a conflict.
	listener, err := net.Listen("tcp", ":4222")
	if err != nil {
		t.Skipf("could not bind port 4222 for test (already in use?): %v", err)
	}
	defer listener.Close()

	// Try to start infrastructure — should fail due to port conflict.
	upCmd := exec.Command("make", "infra-up")
	upCmd.Dir = root
	out, upErr := upCmd.CombinedOutput()
	if upErr == nil {
		// Clean up if it somehow succeeded.
		cleanupCmd := exec.Command("make", "infra-down")
		cleanupCmd.Dir = root
		cleanupCmd.CombinedOutput() //nolint:errcheck
		t.Fatal("expected make infra-up to fail due to port conflict, but it succeeded")
	}
	_ = out // Port conflict reported by podman-compose.
}

// TestInfraPortsReachable verifies that after infra-up, ports 4222 and 55556
// accept TCP connections.
// Test Spec: TS-01-SMOKE-2
// Requirements: 01-REQ-7.4
func TestInfraPortsReachable(t *testing.T) {
	skipIfNoPodman(t)

	root := findRepoRoot(t)

	// Ensure clean state.
	downCmd := exec.Command("make", "infra-down")
	downCmd.Dir = root
	downCmd.CombinedOutput() //nolint:errcheck

	// Start infrastructure.
	upCmd := exec.Command("make", "infra-up")
	upCmd.Dir = root
	out, err := upCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make infra-up failed:\n%s\nerror: %v", out, err)
	}
	defer func() {
		cleanup := exec.Command("make", "infra-down")
		cleanup.Dir = root
		cleanup.CombinedOutput() //nolint:errcheck
	}()

	// Wait for containers to start.
	time.Sleep(5 * time.Second)

	// Check port 4222 (NATS).
	conn4222, err := net.DialTimeout("tcp", "localhost:4222", 5*time.Second)
	if err != nil {
		t.Errorf("port 4222 (NATS) not reachable: %v", err)
	} else {
		conn4222.Close()
	}

	// Check port 55556 (Kuksa Databroker).
	conn55556, err := net.DialTimeout("tcp", "localhost:55556", 5*time.Second)
	if err != nil {
		t.Errorf("port 55556 (Kuksa Databroker) not reachable: %v", err)
	} else {
		conn55556.Close()
	}
}
