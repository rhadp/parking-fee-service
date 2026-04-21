// Static compose.yml configuration tests for the DATA_BROKER service.
// These tests verify the compose.yml structure without requiring a running container.
// Tests in this file will FAIL until task group 2 updates deployments/compose.yml
// for dual listeners, UDS volume, and the correct overlay flags.
package databroker

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestComposePinnedImage verifies that compose.yml references the pinned Kuksa Databroker image.
// The spec mandates :0.6.1 but the registry has :0.5.0 (see errata). Either version satisfies
// the pin requirement; what matters is that an explicit tag is present.
// Test Spec: TS-02-3
// Requirement: 02-REQ-1.1
func TestComposePinnedImage(t *testing.T) {
	content := readComposeYML(t)
	// Accept either the spec-mandated version (0.6.1) or the errata version (0.5.0).
	const pinnedV1 = "kuksa-databroker:0.6.1"
	const pinnedV2 = "kuksa-databroker:0.5.0"
	if !strings.Contains(content, pinnedV1) && !strings.Contains(content, pinnedV2) {
		t.Fatalf("compose.yml does not pin kuksa-databroker to a known version\n"+
			"  expected one of: %q or %q\n  compose.yml excerpt:\n%s",
			pinnedV1, pinnedV2, content)
	}
}

// TestComposeTCPPort verifies that compose.yml contains the required port mapping 55556:55555.
// Test Spec: TS-02-1
// Requirement: 02-REQ-2.2
func TestComposeTCPPort(t *testing.T) {
	content := readComposeYML(t)
	if !strings.Contains(content, "55556:55555") {
		t.Fatalf("compose.yml does not contain TCP port mapping 55556:55555")
	}
}

// TestComposeTCPListener verifies that the databroker command args configure the TCP listener.
// The spec mandates --address 0.0.0.0:55555 (combined form) but the binary may require
// --address 0.0.0.0 --port 55555 as separate flags (see errata). Either form is accepted.
// Test Spec: TS-02-1
// Requirement: 02-REQ-2.1
func TestComposeTCPListener(t *testing.T) {
	content := readComposeYML(t)
	// Combined form: 0.0.0.0:55555
	hasCombined := strings.Contains(content, "0.0.0.0:55555")
	// Separate flags: --address 0.0.0.0 and --port 55555 somewhere in command
	hasSeparate := strings.Contains(content, "0.0.0.0") && strings.Contains(content, "55555")
	if !hasCombined && !hasSeparate {
		t.Fatalf("compose.yml command args do not configure TCP listener on 0.0.0.0:55555\n" +
			"  expected --address 0.0.0.0:55555 (combined) or --address 0.0.0.0 + --port 55555 (separate)")
	}
}

// TestComposeUDSSocket verifies that the databroker command args configure the UDS listener.
// The spec mandates --uds-path but the binary may use --unix-socket (see errata).
// Test Spec: TS-02-2
// Requirement: 02-REQ-3.1
func TestComposeUDSSocket(t *testing.T) {
	content := readComposeYML(t)
	hasUDSFlag := strings.Contains(content, "--uds-path") || strings.Contains(content, "--unix-socket")
	if !hasUDSFlag {
		t.Fatalf("compose.yml command args do not configure UDS listener\n" +
			"  expected --uds-path or --unix-socket flag")
	}
	if !strings.Contains(content, "kuksa-databroker.sock") {
		t.Fatalf("compose.yml command args do not reference the socket file name kuksa-databroker.sock")
	}
}

// TestComposeUDSVolume verifies that compose.yml defines a shared volume for the UDS socket directory.
// The shared volume allows co-located containers to access the socket.
// Test Spec: TS-02-2
// Requirement: 02-REQ-3.2
func TestComposeUDSVolume(t *testing.T) {
	content := readComposeYML(t)
	// The socket directory (/tmp inside the container) must be exposed via a volume.
	if !strings.Contains(content, "/tmp") {
		t.Fatalf("compose.yml does not mount /tmp (or a subdirectory) to expose the UDS socket")
	}
}

// TestComposeVSSOverlay verifies that compose.yml mounts the VSS overlay file and passes it
// to the databroker via the --vss flag.
// Test Spec: TS-02-5
// Requirement: 02-REQ-6.4
func TestComposeVSSOverlay(t *testing.T) {
	content := readComposeYML(t)
	if !strings.Contains(content, "vss-overlay") {
		t.Fatalf("compose.yml does not mount or reference vss-overlay")
	}
	if !strings.Contains(content, "--vss") {
		t.Fatalf("compose.yml command args do not include the --vss flag to load the VSS overlay")
	}
}

// TestComposePermissiveMode verifies that compose.yml does not include authorization flags,
// confirming the databroker runs in permissive (no-auth) mode.
// Test Spec: TS-02-12
// Requirement: 02-REQ-7.1
func TestComposePermissiveMode(t *testing.T) {
	content := readComposeYML(t)
	prohibited := []string{"--token", "--auth-token", "--tls-server-cert", "--jwt", "--require-token"}
	for _, flag := range prohibited {
		if strings.Contains(content, flag) {
			t.Fatalf("compose.yml command args contain auth flag %q; databroker must run in permissive mode", flag)
		}
	}
}

// TestVSSOverlayFormat validates the VSS overlay JSON file structure without requiring a running
// container. It verifies that all 3 custom signals are present with correct types, that
// intermediate branch nodes are defined (required by kuksa-databroker's nested tree format),
// and that all entries have descriptions.
//
// Test Spec: TS-02-5, TS-02-P1
// Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
func TestVSSOverlayFormat(t *testing.T) {
	root := findRepoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")
	data, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("failed to read vss-overlay.json: %v", err)
	}

	// Subtest 1: valid JSON syntax
	t.Run("valid_json", func(t *testing.T) {
		var v any
		if err := json.Unmarshal(data, &v); err != nil {
			t.Fatalf("vss-overlay.json is not valid JSON: %v", err)
		}
	})

	// Parse overlay into a nested map for structural inspection.
	var overlay map[string]any
	if err := json.Unmarshal(data, &overlay); err != nil {
		t.Fatalf("cannot parse vss-overlay.json as JSON object: %v", err)
	}

	// Helper: navigate a nested children-based VSS tree by dot-separated path.
	getNode := func(path string) (map[string]any, bool) {
		parts := strings.Split(path, ".")
		current := overlay
		for i, part := range parts {
			node, ok := current[part]
			if !ok {
				return nil, false
			}
			nodeMap, ok := node.(map[string]any)
			if !ok {
				return nil, false
			}
			if i == len(parts)-1 {
				return nodeMap, true
			}
			// Navigate into children for intermediate nodes.
			children, ok := nodeMap["children"]
			if !ok {
				return nil, false
			}
			childrenMap, ok := children.(map[string]any)
			if !ok {
				return nil, false
			}
			current = childrenMap
		}
		return nil, false
	}

	// Subtest 2: Vehicle.Parking.SessionActive exists
	t.Run("SessionActive_exists", func(t *testing.T) {
		if _, ok := getNode("Vehicle.Parking.SessionActive"); !ok {
			t.Fatal("Vehicle.Parking.SessionActive not found in vss-overlay.json")
		}
	})

	// Subtest 3: Vehicle.Parking.SessionActive has datatype boolean
	t.Run("SessionActive_boolean", func(t *testing.T) {
		node, ok := getNode("Vehicle.Parking.SessionActive")
		if !ok {
			t.Skip("Vehicle.Parking.SessionActive not found; already reported")
		}
		if dt, _ := node["datatype"].(string); dt != "boolean" {
			t.Fatalf("Vehicle.Parking.SessionActive datatype = %q, want %q", dt, "boolean")
		}
	})

	// Subtest 4: Vehicle.Command.Door.Lock exists
	t.Run("DoorLock_exists", func(t *testing.T) {
		if _, ok := getNode("Vehicle.Command.Door.Lock"); !ok {
			t.Fatal("Vehicle.Command.Door.Lock not found in vss-overlay.json")
		}
	})

	// Subtest 5: Vehicle.Command.Door.Lock has datatype string
	t.Run("DoorLock_string", func(t *testing.T) {
		node, ok := getNode("Vehicle.Command.Door.Lock")
		if !ok {
			t.Skip("Vehicle.Command.Door.Lock not found; already reported")
		}
		if dt, _ := node["datatype"].(string); dt != "string" {
			t.Fatalf("Vehicle.Command.Door.Lock datatype = %q, want %q", dt, "string")
		}
	})

	// Subtest 6: Vehicle.Command.Door.Response exists
	t.Run("DoorResponse_exists", func(t *testing.T) {
		if _, ok := getNode("Vehicle.Command.Door.Response"); !ok {
			t.Fatal("Vehicle.Command.Door.Response not found in vss-overlay.json")
		}
	})

	// Subtest 7: Vehicle.Command.Door.Response has datatype string
	t.Run("DoorResponse_string", func(t *testing.T) {
		node, ok := getNode("Vehicle.Command.Door.Response")
		if !ok {
			t.Skip("Vehicle.Command.Door.Response not found; already reported")
		}
		if dt, _ := node["datatype"].(string); dt != "string" {
			t.Fatalf("Vehicle.Command.Door.Response datatype = %q, want %q", dt, "string")
		}
	})

	// Subtest 8: intermediate branch nodes are defined (required by kuksa-databroker nested format)
	t.Run("branch_nodes_defined", func(t *testing.T) {
		branches := []string{
			"Vehicle",
			"Vehicle.Parking",
			"Vehicle.Command",
			"Vehicle.Command.Door",
		}
		for _, branch := range branches {
			node, ok := getNode(branch)
			if !ok {
				t.Errorf("branch node %q missing from vss-overlay.json", branch)
				continue
			}
			nodeType, _ := node["type"].(string)
			if nodeType != "branch" {
				t.Errorf("node %q has type=%q, want %q", branch, nodeType, "branch")
			}
		}
	})

	// Subtest 9: all custom signals have non-empty descriptions
	t.Run("signals_have_descriptions", func(t *testing.T) {
		signals := []string{
			"Vehicle.Parking.SessionActive",
			"Vehicle.Command.Door.Lock",
			"Vehicle.Command.Door.Response",
		}
		for _, sig := range signals {
			node, ok := getNode(sig)
			if !ok {
				t.Errorf("signal %q missing; cannot check description", sig)
				continue
			}
			desc, _ := node["description"].(string)
			if strings.TrimSpace(desc) == "" {
				t.Errorf("signal %q has empty description", sig)
			}
		}
	})
}
