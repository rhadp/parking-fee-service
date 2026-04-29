package setup_test

import (
	"os/exec"
	"path/filepath"
	"testing"
)

// TS-01-P4: Test isolation — all tests pass without infrastructure running
// Property: Property 4 (Test Isolation)
// Requirements: 01-REQ-8.1, 01-REQ-8.2, 01-REQ-8.3, 01-REQ-8.4
//
// This test verifies that all placeholder tests are self-contained and do
// not depend on external infrastructure. It first runs `make infra-down`
// to ensure no containers are running, then runs `make test` and asserts
// success.
func TestPropertyTestIsolation(t *testing.T) {
	root := repoRoot(t)

	// Verify required toolchains are available
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping test isolation property test")
	}

	// Step 1: Ensure no infrastructure is running.
	// infra-down may fail if podman is not installed; that is acceptable
	// because the point is to ensure no containers are running.
	infraDown := exec.Command("make", "infra-down")
	infraDown.Dir = root
	_ = infraDown.Run() // best-effort; ignore errors (podman may not be installed)

	// Step 2: Run make test and assert exit code 0.
	makeTest := exec.Command("make", "test")
	makeTest.Dir = root
	output, err := makeTest.CombinedOutput()
	if err != nil {
		t.Errorf("make test failed with infrastructure down (exit error: %v)\noutput:\n%s", err, string(output))
	}
}

// TS-01-P3: Infrastructure idempotency — repeated infra-up/infra-down
// cycles leave the system in a consistent state.
// Property: Property 3 (Infrastructure Idempotency)
// Requirements: 01-REQ-7.4, 01-REQ-7.5, 01-REQ-7.E2
//
// This test runs two full infra-up/infra-down cycles and verifies that
// each command exits 0 and that no infrastructure containers remain after
// the final infra-down.
func TestPropertyInfrastructureIdempotency(t *testing.T) {
	root := repoRoot(t)

	// Verify required toolchains are available
	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping infrastructure idempotency test")
	}
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not found on PATH; skipping infrastructure idempotency test")
	}

	// Run two cycles of infra-up / infra-down
	for cycle := 1; cycle <= 2; cycle++ {
		t.Run("cycle", func(t *testing.T) {
			// infra-up
			up := exec.Command("make", "infra-up")
			up.Dir = root
			upOutput, err := up.CombinedOutput()
			if err != nil {
				t.Fatalf("cycle %d: make infra-up failed: %v\noutput:\n%s", cycle, err, string(upOutput))
			}

			// infra-down
			down := exec.Command("make", "infra-down")
			down.Dir = root
			downOutput, err := down.CombinedOutput()
			if err != nil {
				t.Fatalf("cycle %d: make infra-down failed: %v\noutput:\n%s", cycle, err, string(downOutput))
			}
		})
	}

	// After final infra-down, verify no infrastructure containers remain
	t.Run("no containers remain", func(t *testing.T) {
		// Check for nats containers
		natsCheck := exec.Command("podman", "ps", "-q", "--filter", "name=nats")
		natsCheck.Dir = root
		natsOut, err := natsCheck.Output()
		if err != nil {
			t.Fatalf("failed to check for nats containers: %v", err)
		}
		if len(natsOut) > 0 {
			t.Errorf("nats containers still running after infra-down: %s", string(natsOut))
		}

		// Check for kuksa containers
		kuksaCheck := exec.Command("podman", "ps", "-q", "--filter", "name=kuksa")
		kuksaCheck.Dir = root
		kuksaOut, err := kuksaCheck.Output()
		if err != nil {
			t.Fatalf("failed to check for kuksa containers: %v", err)
		}
		if len(kuksaOut) > 0 {
			t.Errorf("kuksa containers still running after infra-down: %s", string(kuksaOut))
		}
	})
}

// TS-01-E8: infra-down with no running containers succeeds
// Requirement: 01-REQ-7.E2
func TestInfraDownNoContainers(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not found on PATH; skipping")
	}

	// Ensure nothing is running first
	preDown := exec.Command("make", "infra-down")
	preDown.Dir = root
	_ = preDown.Run()

	// Now run infra-down again with nothing running — should still succeed
	cmd := exec.Command("make", "infra-down")
	cmd.Dir = filepath.Join(root)
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Errorf("make infra-down failed when no containers running: %v\noutput:\n%s", err, string(output))
	}
}
