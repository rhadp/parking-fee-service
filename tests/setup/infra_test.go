package setup_test

import (
	"path/filepath"
	"testing"
)

// TestComposeFileDefinesNATSAndKuksa verifies that deployments/compose.yml defines
// both the NATS service (port 4222) and the Kuksa Databroker service (port 55556).
// Test Spec: TS-01-23
// Requirements: 01-REQ-7.1
func TestComposeFileDefinesNATSAndKuksa(t *testing.T) {
	root := repoRoot(t)
	composePath := filepath.Join(root, "deployments", "compose.yml")

	t.Run("compose_file_exists", func(t *testing.T) {
		assertPathExists(t, composePath)
	})

	t.Run("nats_service_defined", func(t *testing.T) {
		assertFileContains(t, composePath, "nats")
	})

	t.Run("nats_port_4222", func(t *testing.T) {
		assertFileContains(t, composePath, "4222")
	})

	t.Run("kuksa_port_55556", func(t *testing.T) {
		assertFileContains(t, composePath, "55556")
	})
}

// TestNATSConfigFileExists verifies that the NATS server configuration file exists
// at deployments/nats/nats-server.conf and is non-empty.
// Test Spec: TS-01-24
// Requirements: 01-REQ-7.2
func TestNATSConfigFileExists(t *testing.T) {
	root := repoRoot(t)
	natsConfPath := filepath.Join(root, "deployments", "nats", "nats-server.conf")

	t.Run("nats_conf_exists", func(t *testing.T) {
		assertPathExists(t, natsConfPath)
	})

	t.Run("nats_conf_not_empty", func(t *testing.T) {
		assertFileNotEmpty(t, natsConfPath)
	})
}

// TestVSSOverlayDefinesCustomSignals verifies that the VSS overlay file defines the
// three required custom vehicle signals.
// Test Spec: TS-01-25
// Requirements: 01-REQ-7.3
func TestVSSOverlayDefinesCustomSignals(t *testing.T) {
	root := repoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")

	t.Run("overlay_file_exists", func(t *testing.T) {
		assertPathExists(t, overlayPath)
	})

	signals := []string{
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	}

	for _, signal := range signals {
		t.Run(signal, func(t *testing.T) {
			assertFileContains(t, overlayPath, signal)
		})
	}
}
