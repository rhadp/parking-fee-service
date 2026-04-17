// Compose configuration tests — static checks of deployments/compose.yml.
//
// These tests verify that compose.yml is correctly configured for the
// DATA_BROKER component per spec 02_data_broker.  They do NOT require a
// running container; they just read and inspect the YAML file.
//
// Tests will fail until task group 2 updates compose.yml.
package databroker_test

import (
	"encoding/json"
	"os"
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

// TestVSSOverlayFormat verifies that the VSS overlay file (deployments/vss-overlay.json)
// is valid JSON and contains all 3 required custom signals with their correct
// data types.  Also verifies that intermediate branch nodes are defined so the
// overlay can be loaded by kuksa-databroker without errors.
//
// This is a static test — no running container required.
//
// Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
func TestVSSOverlayFormat(t *testing.T) {
	root := repoRoot(t)
	overlayPath := root + "/deployments/vss-overlay.json"
	data, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("cannot read %s: %v", overlayPath, err)
	}

	// Validate JSON syntax.
	var overlay map[string]map[string]string
	if err := json.Unmarshal(data, &overlay); err != nil {
		t.Fatalf("vss-overlay.json is not valid JSON: %v", err)
	}

	// 02-REQ-6.1: Vehicle.Parking.SessionActive must be type boolean.
	t.Run("Vehicle.Parking.SessionActive/boolean", func(t *testing.T) {
		node, ok := overlay["Vehicle.Parking.SessionActive"]
		if !ok {
			t.Fatal("Vehicle.Parking.SessionActive is missing from overlay")
		}
		dt := strings.ToLower(node["datatype"])
		if dt != "boolean" && dt != "bool" {
			t.Errorf("Vehicle.Parking.SessionActive: expected datatype boolean, got %q", node["datatype"])
		}
	})

	// 02-REQ-6.2: Vehicle.Command.Door.Lock must be type string.
	t.Run("Vehicle.Command.Door.Lock/string", func(t *testing.T) {
		node, ok := overlay["Vehicle.Command.Door.Lock"]
		if !ok {
			t.Fatal("Vehicle.Command.Door.Lock is missing from overlay")
		}
		if strings.ToLower(node["datatype"]) != "string" {
			t.Errorf("Vehicle.Command.Door.Lock: expected datatype string, got %q", node["datatype"])
		}
	})

	// 02-REQ-6.3: Vehicle.Command.Door.Response must be type string.
	t.Run("Vehicle.Command.Door.Response/string", func(t *testing.T) {
		node, ok := overlay["Vehicle.Command.Door.Response"]
		if !ok {
			t.Fatal("Vehicle.Command.Door.Response is missing from overlay")
		}
		if strings.ToLower(node["datatype"]) != "string" {
			t.Errorf("Vehicle.Command.Door.Response: expected datatype string, got %q", node["datatype"])
		}
	})

	// Verify intermediate branch nodes are defined so kuksa-databroker can load
	// the overlay without missing-parent errors.
	for _, branchPath := range []string{"Vehicle.Parking", "Vehicle.Command", "Vehicle.Command.Door"} {
		t.Run(branchPath+"/branch", func(t *testing.T) {
			node, ok := overlay[branchPath]
			if !ok {
				t.Errorf("intermediate branch node %q is missing from overlay; kuksa-databroker may fail to load it", branchPath)
				return
			}
			if strings.ToLower(node["type"]) != "branch" {
				t.Errorf("%q: expected type=branch, got %q", branchPath, node["type"])
			}
		})
	}

	// All 3 custom leaf signals must have a description.
	for _, path := range []string{
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	} {
		t.Run(path+"/description", func(t *testing.T) {
			node := overlay[path]
			if node["description"] == "" {
				t.Errorf("%q: missing description field (required by kuksa-databroker)", path)
			}
		})
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
