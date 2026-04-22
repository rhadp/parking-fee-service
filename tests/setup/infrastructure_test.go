package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-23: compose.yml defines NATS and Kuksa services
// Requirement: 01-REQ-7.1
func TestComposeDefinesServices(t *testing.T) {
	root := repoRoot(t)

	composePath := filepath.Join(root, "deployments", "compose.yml")
	data, err := os.ReadFile(composePath)
	if err != nil {
		t.Fatalf("expected %s to exist: %v", composePath, err)
	}

	content := string(data)

	// Verify NATS service is defined
	t.Run("nats_service", func(t *testing.T) {
		if !strings.Contains(content, "nats") {
			t.Error("expected compose.yml to define a nats service")
		}
	})

	// Verify NATS port 4222 is mapped
	t.Run("nats_port", func(t *testing.T) {
		if !strings.Contains(content, "4222") {
			t.Error("expected compose.yml to expose port 4222 for NATS")
		}
	})

	// Verify Kuksa Databroker service exists with port 55556
	t.Run("kuksa_port", func(t *testing.T) {
		if !strings.Contains(content, "55556") {
			t.Error("expected compose.yml to expose port 55556 for Kuksa Databroker")
		}
	})
}

// TS-01-24: NATS configuration file exists
// Requirement: 01-REQ-7.2
func TestNATSConfigExists(t *testing.T) {
	root := repoRoot(t)

	natsConfPath := filepath.Join(root, "deployments", "nats", "nats-server.conf")
	info, err := os.Stat(natsConfPath)
	if err != nil {
		t.Fatalf("expected %s to exist: %v", natsConfPath, err)
	}
	if info.Size() == 0 {
		t.Error("expected nats-server.conf to be non-empty")
	}
}

// TS-01-25: VSS overlay defines custom signals
// Requirement: 01-REQ-7.3
func TestVSSOverlayDefinesCustomSignals(t *testing.T) {
	root := repoRoot(t)

	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")
	data, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("expected %s to exist: %v", overlayPath, err)
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
				t.Errorf("expected VSS overlay to define signal %q", signal)
			}
		})
	}
}
