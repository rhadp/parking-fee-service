package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestComposeFileDefinesServices verifies deployments/compose.yml defines
// NATS and Kuksa Databroker services with correct ports
// (TS-01-23, 01-REQ-7.1).
func TestComposeFileDefinesServices(t *testing.T) {
	root := repoRoot(t)
	composePath := filepath.Join(root, "deployments", "compose.yml")
	assertPathExists(t, composePath)

	data, err := os.ReadFile(composePath)
	if err != nil {
		t.Fatalf("cannot read deployments/compose.yml: %v", err)
	}
	content := string(data)

	// Verify NATS service with port 4222
	if !strings.Contains(content, "nats") {
		t.Error("compose.yml does not define a 'nats' service")
	}
	if !strings.Contains(content, "4222") {
		t.Error("compose.yml does not expose port 4222 for NATS")
	}

	// Verify Kuksa Databroker with port 55556
	if !strings.Contains(content, "55556") {
		t.Error("compose.yml does not expose port 55556 for Kuksa Databroker")
	}
}

// TestNATSConfigFileExists verifies deployments/nats/nats-server.conf exists
// and is non-empty (TS-01-24, 01-REQ-7.2).
func TestNATSConfigFileExists(t *testing.T) {
	root := repoRoot(t)
	confPath := filepath.Join(root, "deployments", "nats", "nats-server.conf")
	assertPathExists(t, confPath)

	info, err := os.Stat(confPath)
	if err != nil {
		t.Fatalf("cannot stat %s: %v", confPath, err)
	}
	if info.Size() == 0 {
		t.Errorf("deployments/nats/nats-server.conf is empty")
	}
}

// TestVSSOverlayDefinesCustomSignals verifies the VSS overlay file defines the
// three required custom signals (TS-01-25, 01-REQ-7.3).
func TestVSSOverlayDefinesCustomSignals(t *testing.T) {
	root := repoRoot(t)

	// Find the VSS overlay file in deployments/
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")
	assertPathExists(t, overlayPath)

	data, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("cannot read %s: %v", overlayPath, err)
	}
	content := string(data)

	requiredSignals := []string{
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	}
	for _, signal := range requiredSignals {
		if !strings.Contains(content, signal) {
			t.Errorf("vss-overlay.json does not define signal %q", signal)
		}
	}
}
