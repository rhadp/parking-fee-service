package setup

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-24: Compose File Defines Services
// Requirement: 01-REQ-6.1
func TestComposeFileServices(t *testing.T) {
	root := repoRoot(t)

	path := filepath.Join(root, "deployments", "compose.yml")
	content, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read deployments/compose.yml: %v", err)
	}

	text := string(content)
	checks := map[string]string{
		"nats service":           "nats",
		"NATS port 4222":        "4222",
		"databroker service":     "databroker",
		"databroker port 55556":  "55556",
	}

	for desc, substr := range checks {
		if !strings.Contains(text, substr) {
			t.Errorf("compose.yml missing %s (expected %q)", desc, substr)
		}
	}
}

// TS-01-25: Infrastructure Starts
// Requirement: 01-REQ-6.2
func TestInfraUp(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping infrastructure test in short mode")
	}

	// This test requires Podman to be running.
	// It is intentionally skipped in CI or when Podman is unavailable.
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("skipping: podman not available")
	}

	root := repoRoot(t)
	cmd := exec.Command("make", "infra-up")
	cmd.Dir = root
	output, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make infra-up failed: %v\n%s", err, string(output))
	}

	// Check containers are running
	psCmd := exec.Command("podman", "ps")
	psOutput, err := psCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("podman ps failed: %v\n%s", err, string(psOutput))
	}

	psText := string(psOutput)
	if !strings.Contains(psText, "nats") {
		t.Error("podman ps does not show nats container")
	}
	if !strings.Contains(psText, "databroker") && !strings.Contains(psText, "kuksa") {
		t.Error("podman ps does not show databroker container")
	}

	// Clean up
	cleanCmd := exec.Command("make", "infra-down")
	cleanCmd.Dir = root
	_ = cleanCmd.Run()
}

// TS-01-26: Infrastructure Stops
// Requirement: 01-REQ-6.3
func TestInfraDown(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping infrastructure test in short mode")
	}

	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("skipping: podman not available")
	}

	root := repoRoot(t)

	// Start infrastructure
	upCmd := exec.Command("make", "infra-up")
	upCmd.Dir = root
	if output, err := upCmd.CombinedOutput(); err != nil {
		t.Fatalf("make infra-up failed: %v\n%s", err, string(output))
	}

	// Stop infrastructure
	downCmd := exec.Command("make", "infra-down")
	downCmd.Dir = root
	output, err := downCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("make infra-down failed: %v\n%s", err, string(output))
	}
	_ = output

	// Verify containers are gone
	psCmd := exec.Command("podman", "ps")
	psOutput, err := psCmd.CombinedOutput()
	if err != nil {
		t.Fatalf("podman ps failed: %v", err)
	}

	psText := string(psOutput)
	if strings.Contains(psText, "nats") {
		t.Error("nats container still running after infra-down")
	}
	if strings.Contains(psText, "databroker") || strings.Contains(psText, "kuksa") {
		t.Error("databroker container still running after infra-down")
	}
}

// TS-01-27: NATS Config Exists
// Requirement: 01-REQ-6.4
func TestNatsConfigExists(t *testing.T) {
	root := repoRoot(t)

	path := filepath.Join(root, "deployments", "nats", "nats-server.conf")
	if !fileExists(path) {
		t.Error("expected deployments/nats/nats-server.conf to exist")
		return
	}

	content, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read nats-server.conf: %v", err)
	}

	if !strings.Contains(string(content), "4222") {
		t.Error("nats-server.conf does not contain port 4222")
	}
}

// TS-01-28: VSS Overlay Exists
// Requirement: 01-REQ-6.5
func TestVssOverlayExists(t *testing.T) {
	root := repoRoot(t)

	path := filepath.Join(root, "deployments", "vss-overlay.json")
	if !fileExists(path) {
		t.Error("expected deployments/vss-overlay.json to exist")
		return
	}

	content, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read vss-overlay.json: %v", err)
	}

	text := string(content)
	signals := []string{"SessionActive", "Lock", "Response"}
	for _, sig := range signals {
		if !strings.Contains(text, sig) {
			t.Errorf("vss-overlay.json missing signal %q", sig)
		}
	}
}
