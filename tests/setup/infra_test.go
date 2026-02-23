package setup_test

import (
	"testing"
	"time"
)

// TS-01-34: Infrastructure composition file exists (01-REQ-7.1)
func TestInfra_ComposeFileExists(t *testing.T) {
	root := repoRoot(t)
	assertFileExists(t, root, "infra/docker-compose.yml")
	assertFileContains(t, root, "infra/docker-compose.yml", "services:")
}

// TS-01-35: Infrastructure includes Mosquitto (01-REQ-7.2)
func TestInfra_MosquittoConfig(t *testing.T) {
	root := repoRoot(t)
	assertFileContains(t, root, "infra/docker-compose.yml", "eclipse-mosquitto")
	assertFileContains(t, root, "infra/docker-compose.yml", "1883")
}

// TS-01-36: Infrastructure includes Kuksa Databroker (01-REQ-7.3)
func TestInfra_KuksaConfig(t *testing.T) {
	root := repoRoot(t)
	assertFileContains(t, root, "infra/docker-compose.yml", "kuksa-databroker")
	assertFileContains(t, root, "infra/docker-compose.yml", "55556")
}

// TS-01-37: make infra-up starts services (01-REQ-7.4)
func TestInfra_Up(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping infrastructure test in short mode")
	}

	// Skip if no container runtime available
	if _, err := lookPath("podman"); err != nil {
		if _, err := lookPath("docker"); err != nil {
			t.Skip("no container runtime (podman/docker) available, skipping infra test")
		}
	}

	root := repoRoot(t)

	// Ensure clean state
	execCommand(t, root, ".", "make", "infra-down")

	// Start infrastructure
	result := execCommand(t, root, ".", "make", "infra-up")
	if result.ExitCode != 0 {
		t.Fatalf("make infra-up failed (exit %d): %s", result.ExitCode, result.Combined)
	}

	// Clean up after test
	defer execCommand(t, root, ".", "make", "infra-down")

	// Wait for ports to become reachable
	if !waitForPort(t, 1883, 30*time.Second) {
		t.Error("MQTT port 1883 not reachable after make infra-up")
	}
	if !waitForPort(t, 55556, 30*time.Second) {
		t.Error("Kuksa port 55556 not reachable after make infra-up")
	}
}

// TS-01-38: make infra-down stops services (01-REQ-7.5)
func TestInfra_Down(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping infrastructure test in short mode")
	}

	// Skip if no container runtime available
	if _, err := lookPath("podman"); err != nil {
		if _, err := lookPath("docker"); err != nil {
			t.Skip("no container runtime (podman/docker) available, skipping infra test")
		}
	}

	root := repoRoot(t)

	// Start infrastructure first
	execCommand(t, root, ".", "make", "infra-up")
	waitForPort(t, 1883, 30*time.Second)

	// Stop infrastructure
	result := execCommand(t, root, ".", "make", "infra-down")
	if result.ExitCode != 0 {
		t.Errorf("make infra-down failed (exit %d): %s", result.ExitCode, result.Combined)
	}

	// Wait a moment for ports to be released
	time.Sleep(2 * time.Second)

	// Verify ports are no longer in use
	if portIsOpen(t, 1883) {
		t.Error("MQTT port 1883 still open after make infra-down")
	}
	if portIsOpen(t, 55556) {
		t.Error("Kuksa port 55556 still open after make infra-down")
	}
}
