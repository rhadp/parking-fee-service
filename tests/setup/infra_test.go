package setup

import (
	"os"
	"strings"
	"testing"
)

// TestComposeFileDefinesNATSAndKuksa verifies deployments/compose.yml defines both
// infrastructure services with the correct ports.
// Test Spec: TS-01-23
// Requirement: 01-REQ-7.1
func TestComposeFileDefinesNATSAndKuksa(t *testing.T) {
	root := findRepoRoot(t)

	composePath := repoPath(root, "deployments", "compose.yml")
	assertFileExists(t, composePath)

	data, err := os.ReadFile(composePath)
	if err != nil {
		t.Fatalf("failed to read compose.yml: %v", err)
	}
	content := string(data)

	// NATS service must be defined.
	t.Run("nats service defined", func(t *testing.T) {
		if !strings.Contains(content, "nats") {
			t.Fatalf("compose.yml does not define a 'nats' service")
		}
	})

	// NATS port 4222 must appear in the port mapping.
	t.Run("nats port 4222", func(t *testing.T) {
		if !strings.Contains(content, "4222") {
			t.Fatalf("compose.yml does not reference port 4222 for NATS")
		}
	})

	// Kuksa Databroker port 55556 must appear.
	t.Run("kuksa port 55556", func(t *testing.T) {
		if !strings.Contains(content, "55556") {
			t.Fatalf("compose.yml does not reference port 55556 for Kuksa Databroker")
		}
	})

	// Kuksa Databroker service must be defined (by image name or service name).
	t.Run("kuksa service defined", func(t *testing.T) {
		if !strings.Contains(content, "kuksa") && !strings.Contains(content, "databroker") {
			t.Fatalf("compose.yml does not define a Kuksa Databroker service (expected 'kuksa' or 'databroker')")
		}
	})
}

// TestNATSConfigurationFileExists verifies the NATS server configuration file exists
// and is non-empty.
// Test Spec: TS-01-24
// Requirement: 01-REQ-7.2
func TestNATSConfigurationFileExists(t *testing.T) {
	root := findRepoRoot(t)

	natsConf := repoPath(root, "deployments", "nats", "nats-server.conf")

	t.Run("file exists", func(t *testing.T) {
		assertFileExists(t, natsConf)
	})

	t.Run("file non-empty", func(t *testing.T) {
		assertFileNonEmpty(t, natsConf)
	})
}

// TestVSSOverlayDefinesCustomSignals verifies the VSS overlay file defines the required
// custom vehicle signals.
// Test Spec: TS-01-25
// Requirement: 01-REQ-7.3
func TestVSSOverlayDefinesCustomSignals(t *testing.T) {
	root := findRepoRoot(t)

	// The VSS overlay could be a JSON file in deployments/.
	overlayPath := repoPath(root, "deployments", "vss-overlay.json")

	assertFileExists(t, overlayPath)

	data, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("failed to read vss-overlay.json: %v", err)
	}
	content := string(data)

	requiredSignals := []string{
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	}

	for _, signal := range requiredSignals {
		t.Run(signal, func(t *testing.T) {
			if !strings.Contains(content, signal) {
				t.Fatalf("vss-overlay.json does not define signal %q", signal)
			}
		})
	}
}
