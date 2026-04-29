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
		t.Fatalf("failed to read %s: %v", composePath, err)
	}
	content := string(data)

	t.Run("nats service defined", func(t *testing.T) {
		if !strings.Contains(content, "nats") {
			t.Error("compose.yml does not define a nats service")
		}
	})

	t.Run("nats port 4222", func(t *testing.T) {
		if !strings.Contains(content, "4222") {
			t.Error("compose.yml does not expose port 4222 for NATS")
		}
	})

	t.Run("kuksa service defined", func(t *testing.T) {
		if !strings.Contains(content, "kuksa") {
			t.Error("compose.yml does not define a kuksa service")
		}
	})

	t.Run("kuksa port 55556", func(t *testing.T) {
		if !strings.Contains(content, "55556") {
			t.Error("compose.yml does not expose port 55556 for Kuksa Databroker")
		}
	})
}

// TS-01-24: NATS configuration file exists
// Requirement: 01-REQ-7.2
func TestNATSConfigExists(t *testing.T) {
	root := repoRoot(t)
	confPath := filepath.Join(root, "deployments", "nats", "nats-server.conf")

	info, err := os.Stat(confPath)
	if os.IsNotExist(err) {
		t.Fatalf("NATS config file does not exist: %s", confPath)
	}
	if err != nil {
		t.Fatalf("error checking NATS config: %v", err)
	}
	if info.Size() == 0 {
		t.Error("NATS config file is empty")
	}
}

// TS-01-25: VSS overlay defines custom signals
// Requirement: 01-REQ-7.3
func TestVSSOverlaySignals(t *testing.T) {
	root := repoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")

	data, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("failed to read VSS overlay: %v", err)
	}
	content := string(data)

	signals := []string{
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	}

	for _, signal := range signals {
		t.Run(signal, func(t *testing.T) {
			if !strings.Contains(content, signal) {
				t.Errorf("VSS overlay does not define signal %q", signal)
			}
		})
	}
}
