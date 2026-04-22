package databroker_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// readCompose reads the compose.yml content from the deployments directory.
func readCompose(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	data, err := os.ReadFile(filepath.Join(root, "deployments", "compose.yml"))
	if err != nil {
		t.Fatalf("failed to read compose.yml: %v", err)
	}
	return string(data)
}

// TestComposePinnedImage verifies that the databroker image is pinned to a
// specific version (not :latest).
// TS-02-3 | Requirement: 02-REQ-1.1
func TestComposePinnedImage(t *testing.T) {
	content := readCompose(t)

	// The image must reference kuksa-databroker with a specific version tag.
	// Per errata, the actual available version may differ from the spec.
	if !strings.Contains(content, "ghcr.io/eclipse-kuksa/kuksa-databroker:") {
		t.Fatal("compose.yml does not contain kuksa-databroker image reference")
	}
	if strings.Contains(content, "kuksa-databroker:latest") {
		t.Error("compose.yml uses :latest tag; image must be pinned to a specific version")
	}
}

// TestComposeTCPPort verifies that port mapping 55556:55555 exists.
// TS-02-1 | Requirement: 02-REQ-2.2
func TestComposeTCPPort(t *testing.T) {
	content := readCompose(t)

	if !strings.Contains(content, "55556:55555") {
		t.Error("compose.yml does not contain port mapping 55556:55555")
	}
}

// TestComposeTCPListener verifies that the databroker command includes TCP
// listener configuration binding to 0.0.0.0 on port 55555.
// TS-02-1 | Requirement: 02-REQ-2.1
func TestComposeTCPListener(t *testing.T) {
	content := readCompose(t)

	// Accept either combined form (--address 0.0.0.0:55555) or split form
	// (--address 0.0.0.0 ... --port 55555). The actual binary uses split form.
	hasCombined := strings.Contains(content, "--address 0.0.0.0:55555")
	hasSplitAddr := strings.Contains(content, "--address") && strings.Contains(content, "0.0.0.0")
	hasSplitPort := strings.Contains(content, "--port") && strings.Contains(content, "55555")

	if !hasCombined && !(hasSplitAddr && hasSplitPort) {
		t.Error("compose.yml command does not configure TCP listener on 0.0.0.0:55555 " +
			"(expected --address 0.0.0.0:55555 or --address 0.0.0.0 --port 55555)")
	}
}

// TestComposeUDSSocket verifies that the databroker command includes UDS
// listener configuration for /tmp/kuksa-databroker.sock.
// TS-02-2 | Requirement: 02-REQ-3.1
func TestComposeUDSSocket(t *testing.T) {
	content := readCompose(t)

	// Accept either --uds-path or --unix-socket flag name.
	// The actual binary uses --unix-socket.
	hasUDSPath := strings.Contains(content, "--uds-path")
	hasUnixSocket := strings.Contains(content, "--unix-socket")
	hasSocketPath := strings.Contains(content, "kuksa-databroker.sock")

	if !(hasUDSPath || hasUnixSocket) || !hasSocketPath {
		t.Error("compose.yml command does not configure UDS listener " +
			"(expected --uds-path or --unix-socket with kuksa-databroker.sock)")
	}
}

// TestComposeUDSVolume verifies that a shared volume is configured to expose
// the UDS socket to co-located containers.
// TS-02-2 | Requirement: 02-REQ-3.2
func TestComposeUDSVolume(t *testing.T) {
	content := readCompose(t)

	// Look for a volume that makes the UDS socket directory accessible.
	// This could be a named volume or bind mount involving /tmp/kuksa or
	// the socket path.
	hasUDSVolume := strings.Contains(content, "kuksa-uds") ||
		strings.Contains(content, "/tmp/kuksa")

	if !hasUDSVolume {
		t.Error("compose.yml does not define a shared volume for the UDS socket directory")
	}
}

// TestComposeVSSOverlay verifies that the VSS overlay file is mounted and
// referenced in the databroker command args.
// TS-02-5 | Requirement: 02-REQ-6.4
func TestComposeVSSOverlay(t *testing.T) {
	content := readCompose(t)

	// Check that the overlay file is volume-mounted.
	if !strings.Contains(content, "vss-overlay") {
		t.Error("compose.yml does not mount the VSS overlay file")
	}

	// Check that the command references the overlay via a flag.
	// Accept --vss, --metadata, or similar overlay flags.
	hasVSSFlag := strings.Contains(content, "--vss") ||
		strings.Contains(content, "--metadata")

	if !hasVSSFlag {
		t.Error("compose.yml command does not reference the VSS overlay via --vss or --metadata flag")
	}
}

// TestComposePermissiveMode verifies that no authentication flags are present
// in the databroker command args.
// TS-02-12 | Requirement: 02-REQ-7.1
func TestComposePermissiveMode(t *testing.T) {
	content := readCompose(t)

	authFlags := []string{"--token", "--auth", "--jwt", "--tls-server-cert"}
	for _, flag := range authFlags {
		if strings.Contains(content, flag) {
			t.Errorf("compose.yml contains auth flag %q; DATA_BROKER should run in permissive mode", flag)
		}
	}
}

// navigateOverlay traverses a nested JSON overlay structure following the given
// dot-separated path segments (e.g., ["Vehicle", "Parking", "SessionActive"]).
// Branch nodes are expected to hold their children under a "children" key.
// Returns the node at the final path segment, or nil if the path is invalid.
func navigateOverlay(root map[string]interface{}, segments []string) map[string]interface{} {
	current := root
	for i, seg := range segments {
		nodeRaw, ok := current[seg]
		if !ok {
			return nil
		}
		node, ok := nodeRaw.(map[string]interface{})
		if !ok {
			return nil
		}
		// If this is not the last segment, descend into "children".
		if i < len(segments)-1 {
			childrenRaw, ok := node["children"]
			if !ok {
				return nil
			}
			children, ok := childrenRaw.(map[string]interface{})
			if !ok {
				return nil
			}
			current = children
		} else {
			return node
		}
	}
	return nil
}

// TestVSSOverlayFormat verifies that the VSS overlay JSON file is valid and
// defines all 3 custom signals with correct types and branch nodes by properly
// navigating the JSON tree structure.
// TS-02-5 | Requirement: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestVSSOverlayFormat(t *testing.T) {
	root := repoRoot(t)
	data, err := os.ReadFile(filepath.Join(root, "deployments", "vss-overlay.json"))
	if err != nil {
		t.Fatalf("failed to read vss-overlay.json: %v", err)
	}

	// Verify valid JSON.
	var overlay map[string]interface{}
	if err := json.Unmarshal(data, &overlay); err != nil {
		t.Fatalf("vss-overlay.json is not valid JSON: %v", err)
	}

	// Verify each custom signal by navigating the tree to its exact location.
	type signalCheck struct {
		path     string
		datatype string
	}
	signals := []signalCheck{
		{"Vehicle.Parking.SessionActive", "boolean"},
		{"Vehicle.Command.Door.Lock", "string"},
		{"Vehicle.Command.Door.Response", "string"},
	}

	for _, sig := range signals {
		t.Run(sig.path, func(t *testing.T) {
			segments := strings.Split(sig.path, ".")
			node := navigateOverlay(overlay, segments)
			if node == nil {
				t.Fatalf("signal %s not found at expected path in overlay JSON tree", sig.path)
			}
			dt, ok := node["datatype"]
			if !ok {
				t.Fatalf("signal %s has no 'datatype' field", sig.path)
			}
			dtStr, ok := dt.(string)
			if !ok {
				t.Fatalf("signal %s 'datatype' is not a string: %T", sig.path, dt)
			}
			if dtStr != sig.datatype {
				t.Errorf("signal %s: expected datatype %q, got %q", sig.path, sig.datatype, dtStr)
			}
		})
	}

	// Verify branch nodes by navigating to each intermediate path and checking
	// their "type" field is "branch".
	type branchCheck struct {
		path string
	}
	branches := []branchCheck{
		{"Vehicle"},
		{"Vehicle.Parking"},
		{"Vehicle.Command"},
		{"Vehicle.Command.Door"},
	}

	for _, bc := range branches {
		t.Run("branch/"+bc.path, func(t *testing.T) {
			segments := strings.Split(bc.path, ".")
			node := navigateOverlay(overlay, segments)
			if node == nil {
				t.Fatalf("branch node %s not found in overlay JSON tree", bc.path)
			}
			nodeType, ok := node["type"]
			if !ok {
				t.Fatalf("branch node %s has no 'type' field", bc.path)
			}
			typeStr, ok := nodeType.(string)
			if !ok {
				t.Fatalf("branch node %s 'type' is not a string: %T", bc.path, nodeType)
			}
			if typeStr != "branch" {
				t.Errorf("branch node %s: expected type %q, got %q", bc.path, "branch", typeStr)
			}
			// Branch nodes must have a "children" field.
			if _, ok := node["children"]; !ok {
				t.Errorf("branch node %s has no 'children' field", bc.path)
			}
			// Branch nodes should have a description.
			if _, ok := node["description"]; !ok {
				t.Errorf("branch node %s has no 'description' field", bc.path)
			}
		})
	}

	// Verify leaf signal nodes have a valid "type" field (sensor or actuator).
	for _, sig := range signals {
		t.Run("leaf_type/"+sig.path, func(t *testing.T) {
			segments := strings.Split(sig.path, ".")
			node := navigateOverlay(overlay, segments)
			if node == nil {
				t.Fatalf("signal %s not found", sig.path)
			}
			nodeType, ok := node["type"]
			if !ok {
				t.Fatalf("signal %s has no 'type' field", sig.path)
			}
			typeStr, ok := nodeType.(string)
			if !ok {
				t.Fatalf("signal %s 'type' is not a string", sig.path)
			}
			if typeStr != "sensor" && typeStr != "actuator" && typeStr != "attribute" {
				t.Errorf("signal %s: expected type sensor/actuator/attribute, got %q", sig.path, typeStr)
			}
		})
	}
}
