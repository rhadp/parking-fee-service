package databroker_test

// compose_test.go — static configuration checks for compose.yml and the VSS overlay.
//
// These tests parse compose.yml as text (no YAML library needed) and assert
// the required configuration for the DATA_BROKER service.  They run without a
// live container and serve as the primary failing signal when compose.yml has
// not yet been updated (task group 2).
//
// Also includes TestVSSOverlayFormat which validates deployments/vss-overlay.json
// structure, types, branch nodes, and descriptions (task group 3).
//
// Tests: TS-02-1 (port), TS-02-2 (UDS flag/volume), TS-02-3 (image pin),
//        TS-02-5 (VSS overlay flag), TS-02-12 (permissive mode),
//        TS-02-P1 (signal completeness via overlay static check).
// Requirements: 02-REQ-1.1, 02-REQ-2.1, 02-REQ-2.2, 02-REQ-3.1, 02-REQ-3.2,
//               02-REQ-5.3, 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4,
//               02-REQ-6.5, 02-REQ-7.1.

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// TestComposePinnedImage verifies that the databroker image is pinned to the
// exact version :0.5.0 (02-REQ-1.1, TS-02-3).
//
// EXPECTED TO FAIL before task group 2 (image is currently :latest).
func TestComposePinnedImage(t *testing.T) {
	compose := readComposeYML(t)
	assertContains(t, compose,
		"ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0",
		"compose.yml must pin the databroker image to :0.5.0")
}

// TestComposeTCPPort verifies that the host-to-container port mapping is
// 55556:55555 (02-REQ-2.2, TS-02-1).
//
// EXPECTED TO FAIL before task group 2 (port mapping is currently 55556:55556).
func TestComposeTCPPort(t *testing.T) {
	compose := readComposeYML(t)
	assertContains(t, compose,
		"55556:55555",
		"compose.yml must map host port 55556 to container port 55555")
}

// TestComposeTCPListener verifies that the databroker is configured to listen
// on 0.0.0.0:55555 inside the container via separate --address and --port args
// (02-REQ-2.1, TS-02-1).
//
// EXPECTED TO FAIL before task group 2.
func TestComposeTCPListener(t *testing.T) {
	compose := readComposeYML(t)
	assertContains(t, compose, "--address",
		"compose.yml command must include --address arg")
	assertContains(t, compose, "0.0.0.0",
		"compose.yml command must bind to 0.0.0.0")
	assertContains(t, compose, "--port",
		"compose.yml command must include --port arg")
	assertContains(t, compose, "55555",
		"compose.yml must use container port 55555")
}

// TestComposeUDSSocket verifies that the databroker command includes the
// --unix-socket flag pointing to /tmp/kuksa-databroker.sock (02-REQ-3.1, TS-02-2).
//
// EXPECTED TO FAIL before task group 2 (--unix-socket arg is missing).
func TestComposeUDSSocket(t *testing.T) {
	compose := readComposeYML(t)
	assertContains(t, compose, "--unix-socket",
		"compose.yml command must include --unix-socket flag")
	assertContains(t, compose, "kuksa-databroker.sock",
		"compose.yml command must reference kuksa-databroker.sock UDS path")
}

// TestComposeUDSVolume verifies that compose.yml defines a shared volume or
// bind-mount that makes the UDS socket directory accessible to co-located
// containers (02-REQ-3.2, TS-02-2).
//
// EXPECTED TO FAIL before task group 2 (no UDS volume mount configured).
func TestComposeUDSVolume(t *testing.T) {
	compose := readComposeYML(t)
	// A named volume called "kuksa-uds" or similar must be declared and mounted.
	// We accept any volume reference that ties the UDS socket to a shared mount.
	assertContains(t, compose, "kuksa-uds",
		"compose.yml must declare a named volume for the UDS socket (kuksa-uds)")
}

// TestComposeVSSOverlay verifies that the compose.yml loads both the standard
// VSS release tree and the custom overlay via the --vss flag (02-REQ-5.3,
// 02-REQ-6.4, TS-02-4, TS-02-5).
//
// EXPECTED TO FAIL before task group 2 (standard VSS tree not included).
func TestComposeVSSOverlay(t *testing.T) {
	compose := readComposeYML(t)
	assertContains(t, compose, "vss_release_4.0.json",
		"compose.yml --vss must include the standard VSS 4.0 release tree")
	assertContains(t, compose, "vss-overlay.json",
		"compose.yml --vss must include the custom overlay file")
}

// TestComposePermissiveMode verifies that the databroker command does NOT
// include any token, authorization, or TLS flags (02-REQ-7.1, TS-02-12).
//
// Expected to PASS even before task group 2 (current compose has no auth flags).
func TestComposePermissiveMode(t *testing.T) {
	compose := readComposeYML(t)
	authFlags := []string{"--token", "--authorization", "--tls-server-cert", "--jwt"}
	for _, flag := range authFlags {
		assertNotContains(t, compose, flag,
			"compose.yml must not include auth flag "+flag+" (permissive mode)")
	}
}

// TestVSSOverlayFormat validates the structure and correctness of the VSS
// overlay file at deployments/vss-overlay.json without requiring a running
// container (02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4, 02-REQ-6.5).
//
// Verifies:
//   - The file is valid JSON
//   - All 3 custom leaf signals exist with correct datatypes
//   - All 3 required intermediate branch nodes are present with type "branch"
//   - All leaf signals have a description
//   - All branch nodes have a description
func TestVSSOverlayFormat(t *testing.T) {
	root := repoRoot(t)
	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")
	data, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("cannot read vss-overlay.json: %v", err)
	}

	t.Run("valid_json", func(t *testing.T) {
		var parsed map[string]any
		if err := json.Unmarshal(data, &parsed); err != nil {
			t.Fatalf("vss-overlay.json is not valid JSON: %v", err)
		}
	})

	// Parse overlay into a map for inspection.
	var overlay map[string]map[string]any
	if err := json.Unmarshal(data, &overlay); err != nil {
		t.Skipf("skipping subtests: overlay is not valid JSON: %v", err)
	}

	// --- leaf signal datatype checks ---

	leafSignals := []struct {
		path             string
		expectedDatatype string
	}{
		{"Vehicle.Parking.SessionActive", "boolean"},
		{"Vehicle.Command.Door.Lock", "string"},
		{"Vehicle.Command.Door.Response", "string"},
	}

	for _, sig := range leafSignals {
		t.Run(sig.path+" datatype", func(t *testing.T) {
			entry, ok := overlay[sig.path]
			if !ok {
				t.Fatalf("signal %q not found in vss-overlay.json", sig.path)
			}
			dt, _ := entry["datatype"].(string)
			if dt != sig.expectedDatatype {
				t.Errorf("signal %q has datatype %q, want %q", sig.path, dt, sig.expectedDatatype)
			}
		})
	}

	// --- branch node checks ---

	branchPaths := []string{
		"Vehicle.Parking",
		"Vehicle.Command",
		"Vehicle.Command.Door",
	}

	for _, path := range branchPaths {
		t.Run(path+" is branch", func(t *testing.T) {
			entry, ok := overlay[path]
			if !ok {
				t.Fatalf("branch node %q not found in vss-overlay.json (required by kuksa flat JSON format)", path)
			}
			nodeType, _ := entry["type"].(string)
			if nodeType != "branch" {
				t.Errorf("node %q has type %q, want \"branch\"", path, nodeType)
			}
		})
	}

	// --- description checks ---

	t.Run("leaf_signals_have_descriptions", func(t *testing.T) {
		for _, sig := range leafSignals {
			entry, ok := overlay[sig.path]
			if !ok {
				continue // already covered by datatype subtest
			}
			desc, _ := entry["description"].(string)
			if desc == "" {
				t.Errorf("signal %q has no description", sig.path)
			}
		}
	})

	t.Run("branch_nodes_have_descriptions", func(t *testing.T) {
		for _, path := range branchPaths {
			entry, ok := overlay[path]
			if !ok {
				continue // already covered by branch subtest
			}
			desc, _ := entry["description"].(string)
			if desc == "" {
				t.Errorf("branch node %q has no description", path)
			}
		}
	})
}
