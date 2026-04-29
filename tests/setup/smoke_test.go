package setup_test

import (
	"net"
	"os/exec"
	"testing"
	"time"
)

// TS-01-SMOKE-1: Full build-test cycle
// Verifies the complete build and test cycle works end-to-end from a clean state.
func TestSmokeBuildTestCycle(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}

	// make clean
	cleanCmd := exec.Command("make", "clean")
	cleanCmd.Dir = root
	cleanOutput, err := cleanCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make clean failed: %v\noutput:\n%s", err, string(cleanOutput))
	}

	// make build
	buildCmd := exec.Command("make", "build")
	buildCmd.Dir = root
	buildOutput, err := buildCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make build failed: %v\noutput:\n%s", err, string(buildOutput))
	}

	// make test
	testCmd := exec.Command("make", "test")
	testCmd.Dir = root
	testOutput, err := testCmd.CombinedOutput()
	if err != nil {
		t.Errorf("make test failed: %v\noutput:\n%s", err, string(testOutput))
	}
}

// TS-01-SMOKE-2: Infrastructure lifecycle
// Verifies NATS and Kuksa Databroker containers start, are reachable, and stop cleanly.
func TestSmokeInfrastructureLifecycle(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not found on PATH; skipping")
	}

	// Start infrastructure
	upCmd := exec.Command("make", "infra-up")
	upCmd.Dir = root
	upOutput, err := upCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make infra-up failed: %v\noutput:\n%s", err, string(upOutput))
	}

	// Ensure cleanup
	t.Cleanup(func() {
		downCmd := exec.Command("make", "infra-down")
		downCmd.Dir = root
		_ = downCmd.Run()
	})

	// Wait a moment for services to start
	time.Sleep(3 * time.Second)

	// Verify port 4222 (NATS) is reachable
	t.Run("nats_port_4222", func(t *testing.T) {
		conn, err := net.DialTimeout("tcp", "localhost:4222", 5*time.Second)
		if err != nil {
			t.Errorf("NATS port 4222 is not reachable: %v", err)
			return
		}
		conn.Close()
	})

	// Verify port 55556 (Kuksa Databroker) is reachable
	t.Run("kuksa_port_55556", func(t *testing.T) {
		conn, err := net.DialTimeout("tcp", "localhost:55556", 5*time.Second)
		if err != nil {
			t.Errorf("Kuksa Databroker port 55556 is not reachable: %v", err)
			return
		}
		conn.Close()
	})

	// Stop infrastructure
	downCmd := exec.Command("make", "infra-down")
	downCmd.Dir = root
	downOutput, err := downCmd.CombinedOutput()
	if err != nil {
		t.Errorf("make infra-down failed: %v\noutput:\n%s", err, string(downOutput))
	}

	// Verify containers are removed
	t.Run("no_containers_remain", func(t *testing.T) {
		psCmd := exec.Command("podman", "ps", "-q", "--filter", "name=nats", "--filter", "name=kuksa")
		psCmd.Dir = root
		psOut, err := psCmd.Output()
		if err != nil {
			t.Logf("warning: could not check container status: %v", err)
			return
		}
		if len(psOut) > 0 {
			t.Errorf("containers still running after infra-down: %s", string(psOut))
		}
	})
}

// TS-01-SMOKE-3: Proto generation and build integration
// Verifies proto generation produces code that integrates with the Go build.
func TestSmokeProtoGenerationAndBuild(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping")
	}
	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH; skipping")
	}
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go not found on PATH; skipping")
	}

	// Run make proto
	protoCmd := exec.Command("make", "proto")
	protoCmd.Dir = root
	protoOutput, err := protoCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make proto failed: %v\noutput:\n%s", err, string(protoOutput))
	}

	// Verify generated code compiles
	buildCmd := exec.Command("go", "build", "./...")
	buildCmd.Dir = root
	buildOutput, err := buildCmd.CombinedOutput()
	if err != nil {
		t.Errorf("go build ./... failed after make proto: %v\noutput:\n%s", err, string(buildOutput))
	}
}
