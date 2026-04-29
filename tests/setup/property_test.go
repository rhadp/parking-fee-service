package setup_test

import (
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
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

// TS-01-P1: Build completeness across all components
// Property: Property 1 (Build Completeness)
// Requirements: 01-REQ-2.4, 01-REQ-3.4, 01-REQ-6.2
func TestPropertyBuildCompleteness(t *testing.T) {
	root := repoRoot(t)

	if _, err := exec.LookPath("make"); err != nil {
		t.Skip("make not found on PATH; skipping build completeness property test")
	}

	cmd := exec.Command("make", "build")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make build failed: %v\noutput:\n%s", err, string(output))
	}

	// Verify all Rust build artifacts exist
	rustBinaries := []string{
		"locking-service",
		"cloud-gateway-client",
		"update-service",
		"parking-operator-adaptor",
		"location-sensor",
		"speed-sensor",
		"door-sensor",
	}

	for _, bin := range rustBinaries {
		t.Run("artifact/"+bin, func(t *testing.T) {
			binPath := filepath.Join(root, "rhivos", "target", "debug", bin)
			assertPathExists(t, binPath)
		})
	}
}

// TS-01-P2: Skeleton determinism across invocations
// Property: Property 2 (Skeleton Determinism)
// Requirements: 01-REQ-4.1, 01-REQ-4.2
func TestPropertySkeletonDeterminism(t *testing.T) {
	root := repoRoot(t)

	// Test Rust binaries
	if _, err := exec.LookPath("cargo"); err == nil {
		// Build first
		buildCmd := exec.Command("cargo", "build", "--workspace")
		buildCmd.Dir = filepath.Join(root, "rhivos")
		if buildOutput, err := buildCmd.CombinedOutput(); err != nil {
			t.Logf("cargo build failed: %v\noutput:\n%s", err, string(buildOutput))
		} else {
			rustBinaries := []string{
				"locking-service",
				"cloud-gateway-client",
				"update-service",
				"parking-operator-adaptor",
				"location-sensor",
				"speed-sensor",
				"door-sensor",
			}

			for _, bin := range rustBinaries {
				t.Run("rust/"+bin, func(t *testing.T) {
					binPath := filepath.Join(root, "rhivos", "target", "debug", bin)

					// First invocation
					cmd1 := exec.Command(binPath)
					out1, err1 := cmd1.CombinedOutput()

					// Second invocation
					cmd2 := exec.Command(binPath)
					out2, err2 := cmd2.CombinedOutput()

					if (err1 == nil) != (err2 == nil) {
						t.Errorf("%s exit status differs between invocations", bin)
						return
					}
					if string(out1) != string(out2) {
						t.Errorf("%s output differs between invocations:\n  first:  %s\n  second: %s",
							bin, string(out1), string(out2))
					}
				})
			}
		}
	}

	// Test Go binaries
	if _, err := exec.LookPath("go"); err == nil {
		goModules := map[string]string{
			"backend/parking-fee-service": "parking-fee-service",
			"backend/cloud-gateway":       "cloud-gateway",
			"mock/parking-app-cli":        "parking-app-cli",
			"mock/companion-app-cli":      "companion-app-cli",
			"mock/parking-operator":       "parking-operator",
		}

		for modPath, name := range goModules {
			t.Run("go/"+name, func(t *testing.T) {
				dir := filepath.Join(root, modPath)

				cmd1 := exec.Command("go", "run", ".")
				cmd1.Dir = dir
				out1, err1 := cmd1.CombinedOutput()

				cmd2 := exec.Command("go", "run", ".")
				cmd2.Dir = dir
				out2, err2 := cmd2.CombinedOutput()

				if (err1 == nil) != (err2 == nil) {
					t.Errorf("%s exit status differs between invocations", name)
					return
				}
				if string(out1) != string(out2) {
					t.Errorf("%s output differs between invocations:\n  first:  %s\n  second: %s",
						name, string(out1), string(out2))
				}
			})
		}
	}
}

// TS-01-P5: Proto consistency across all proto files
// Property: Property 5 (Proto Consistency)
// Requirements: 01-REQ-5.2, 01-REQ-5.3, 01-REQ-5.4, 01-REQ-10.1
func TestPropertyProtoConsistency(t *testing.T) {
	root := repoRoot(t)
	protoDir := filepath.Join(root, "proto")

	if _, err := exec.LookPath("protoc"); err != nil {
		t.Skip("protoc not found on PATH; skipping proto consistency property test")
	}

	protoFiles := findProtoFiles(t, protoDir)
	if len(protoFiles) == 0 {
		t.Fatal("no .proto files found in proto/ directory")
	}

	packageRe := regexp.MustCompile(`(?m)^package\s+\w+`)
	goPackageRe := regexp.MustCompile(`(?m)option\s+go_package\s*=`)

	for _, protoFile := range protoFiles {
		relPath, _ := filepath.Rel(root, protoFile)
		t.Run(relPath, func(t *testing.T) {
			content := readFileContent(t, protoFile)

			// Verify proto3 syntax
			if !strings.Contains(content, `syntax = "proto3"`) {
				t.Errorf("missing syntax = \"proto3\"")
			}

			// Verify package declaration
			if !packageRe.MatchString(content) {
				t.Errorf("missing package declaration")
			}

			// Verify go_package option
			if !goPackageRe.MatchString(content) {
				t.Errorf("missing go_package option")
			}

			// Verify protoc can parse it
			cmd := exec.Command("protoc",
				"--proto_path="+protoDir,
				"--descriptor_set_out=/dev/null",
				protoFile,
			)
			output, err := cmd.CombinedOutput()
			if err != nil {
				t.Errorf("protoc failed to parse: %v\noutput:\n%s", err, string(output))
			}
		})
	}
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
