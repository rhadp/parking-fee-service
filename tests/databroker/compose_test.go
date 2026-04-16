// Compose configuration tests — static checks of deployments/compose.yml.
//
// These tests verify that compose.yml is correctly configured for the
// DATA_BROKER component per spec 02_data_broker.  They do NOT require a
// running container; they just read and inspect the YAML file.
//
// Tests will fail until task group 2 updates compose.yml.
package databroker_test

import (
	"strings"
	"testing"
)

// TestComposePinnedImage verifies that the databroker service uses the pinned
// image version ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0.
//
// Note: The spec mandates :0.5.1 but that tag does not exist in the registry.
// Per errata 02_data_broker_compose_flags.md, :0.5.0 is the correct tag.
//
// Test Spec: TS-02-3
// Requirements: 02-REQ-1.1, 02-REQ-1.2
func TestComposePinnedImage(t *testing.T) {
	compose := readComposeYML(t)

	// Must reference the pinned image with a specific version tag.
	// :latest is not acceptable.
	if strings.Contains(compose, "kuksa-databroker:latest") {
		t.Error("compose.yml uses :latest tag for kuksa-databroker; must pin to a specific version (e.g. :0.5.0)")
	}

	// The pinned version in use (0.5.0 per errata — :0.5.1 does not exist).
	if !strings.Contains(compose, "kuksa-databroker:0.5.0") {
		t.Error("compose.yml does not contain 'kuksa-databroker:0.5.0'; image must be pinned to 0.5.0")
	}
}

// TestComposeTCPPort verifies that compose.yml maps container port 55555 to
// host port 55556.
//
// Test Spec: TS-02-1
// Requirements: 02-REQ-2.2
func TestComposeTCPPort(t *testing.T) {
	compose := readComposeYML(t)

	if !strings.Contains(compose, "55556:55555") {
		t.Error("compose.yml does not contain port mapping '55556:55555' for the databroker service")
	}
}

// TestComposeTCPListener verifies that the databroker command args include
// the TCP listener address and port configuration.
//
// Test Spec: TS-02-1
// Requirements: 02-REQ-2.1
func TestComposeTCPListener(t *testing.T) {
	compose := readComposeYML(t)

	// The databroker uses --address <host> and --port <port> flags separately.
	// Combined format "0.0.0.0:55555" is invalid for this binary.
	// Per errata: --address 0.0.0.0 (no port) + --port 55555.
	if !strings.Contains(compose, "0.0.0.0") {
		t.Error("compose.yml databroker command does not contain '--address 0.0.0.0'")
	}
	if !strings.Contains(compose, "55555") {
		t.Error("compose.yml databroker command does not contain port 55555")
	}
}

// TestComposeUDSSocket verifies that the databroker command args include the
// UDS socket flag.
//
// Note: The spec mandates --uds-path but that flag does not exist.
// Per errata 02_data_broker_compose_flags.md, the correct flag is --unix-socket.
//
// Test Spec: TS-02-2
// Requirements: 02-REQ-3.1, 02-REQ-3.2
func TestComposeUDSSocket(t *testing.T) {
	compose := readComposeYML(t)

	// The correct CLI flag for UDS in kuksa-databroker is --unix-socket.
	// --uds-path does not exist (see errata).
	if !strings.Contains(compose, "--unix-socket") {
		t.Error("compose.yml databroker command does not contain '--unix-socket'; UDS listener is not configured")
	}
}

// TestComposeUDSVolume verifies that compose.yml configures a shared volume
// for the UDS socket directory so co-located containers can access it.
//
// Test Spec: TS-02-2
// Requirements: 02-REQ-3.2
func TestComposeUDSVolume(t *testing.T) {
	compose := readComposeYML(t)

	// A named volume or bind-mount must expose the UDS socket directory.
	// The expected path inside the container is /tmp/kuksa-databroker.sock,
	// and the host path is /tmp/kuksa/ per the volume bind mount.
	hasUDSVolume := strings.Contains(compose, "kuksa-uds") ||
		strings.Contains(compose, "/tmp/kuksa")
	if !hasUDSVolume {
		t.Error("compose.yml does not define a UDS socket volume mount (expected 'kuksa-uds' named volume or '/tmp/kuksa' bind mount)")
	}
}

// TestComposeVSSOverlay verifies that compose.yml mounts the VSS overlay file
// and passes it to the databroker via the --vss flag.
//
// Requirements: 02-REQ-6.4
func TestComposeVSSOverlay(t *testing.T) {
	compose := readComposeYML(t)

	if !strings.Contains(compose, "vss-overlay") && !strings.Contains(compose, "overlay") {
		t.Error("compose.yml does not reference the VSS overlay file in volumes")
	}
	if !strings.Contains(compose, "--vss") {
		t.Error("compose.yml databroker command does not include the --vss flag for the overlay")
	}
}

// TestComposePermissiveMode verifies that the databroker command args do NOT
// include any token or authorization flags (permissive mode).
//
// Test Spec: TS-02-12
// Requirements: 02-REQ-7.1
func TestComposePermissiveMode(t *testing.T) {
	compose := readComposeYML(t)

	disallowedFlags := []string{"--token", "--auth", "--jwt", "--tls-server-cert"}
	for _, flag := range disallowedFlags {
		if strings.Contains(compose, flag) {
			t.Errorf("compose.yml databroker command contains auth flag %q; must run in permissive mode (no auth)", flag)
		}
	}
}
