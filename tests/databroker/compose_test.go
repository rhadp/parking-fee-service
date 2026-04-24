package databroker_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// readCompose reads the compose.yml file from the deployments directory.
func readCompose(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	data, err := os.ReadFile(filepath.Join(root, "deployments", "compose.yml"))
	if err != nil {
		t.Fatalf("failed to read compose.yml: %v", err)
	}
	return string(data)
}

// TestComposePinnedImage verifies that the Kuksa Databroker image is pinned
// to a specific version in compose.yml (not :latest or untagged).
//
// Test Spec: TS-02-3
// Requirements: 02-REQ-1.1
func TestComposePinnedImage(t *testing.T) {
	content := readCompose(t)

	if !strings.Contains(content, "kuksa-databroker:") {
		t.Fatal("compose.yml does not contain a kuksa-databroker image reference")
	}

	// Verify the image is not :latest.
	if strings.Contains(content, "kuksa-databroker:latest") {
		t.Error("compose.yml uses :latest tag for kuksa-databroker; image must be pinned to a specific version")
	}

	// Verify the image is from the correct registry.
	if !strings.Contains(content, "ghcr.io/eclipse-kuksa/kuksa-databroker:") {
		t.Error("compose.yml image must be from ghcr.io/eclipse-kuksa/kuksa-databroker")
	}
}

// TestComposeTCPPort verifies that port mapping 55556:55555 is present in
// compose.yml for the databroker service.
//
// Test Spec: TS-02-1
// Requirements: 02-REQ-2.2
func TestComposeTCPPort(t *testing.T) {
	content := readCompose(t)

	if !strings.Contains(content, "55556:55555") {
		t.Error("compose.yml must map container port 55555 to host port 55556 (55556:55555)")
	}
}

// TestComposeTCPListener verifies that the databroker command args include
// the TCP listener address configuration.
//
// Note: Kuksa Databroker uses separate --address and --port flags (not
// combined host:port). See skeptic review findings for 02-REQ-2.
//
// Test Spec: TS-02-1
// Requirements: 02-REQ-2.1
func TestComposeTCPListener(t *testing.T) {
	content := readCompose(t)

	// The correct CLI form is: --address 0.0.0.0 --port 55555
	// (not --address 0.0.0.0:55555 which is invalid for this binary)
	if !strings.Contains(content, "--address") {
		t.Error("compose.yml command must include --address flag for TCP listener")
	}
	if !strings.Contains(content, "0.0.0.0") {
		t.Error("compose.yml command must bind TCP listener to 0.0.0.0")
	}
	if !strings.Contains(content, "--port") {
		t.Error("compose.yml command must include --port flag")
	}
}

// TestComposeUDSSocket verifies that the databroker command args include
// the UDS listener configuration.
//
// Note: Kuksa Databroker uses --unix-socket (not --uds-path which doesn't
// exist). See skeptic review findings for 02-REQ-3.1.
//
// Test Spec: TS-02-2
// Requirements: 02-REQ-3.1
func TestComposeUDSSocket(t *testing.T) {
	content := readCompose(t)

	// The correct CLI flag is --unix-socket (not --uds-path).
	if !strings.Contains(content, "--unix-socket") {
		t.Error("compose.yml command must include --unix-socket flag for UDS listener")
	}
	if !strings.Contains(content, "kuksa-databroker.sock") {
		t.Error("compose.yml command must reference kuksa-databroker.sock")
	}
}

// TestComposeUDSVolume verifies that a shared volume mount is configured
// for the UDS socket directory, allowing co-located containers to access it.
//
// Test Spec: TS-02-2
// Requirements: 02-REQ-3.2
func TestComposeUDSVolume(t *testing.T) {
	content := readCompose(t)

	// The UDS socket needs a volume mount that makes /tmp accessible to
	// co-located containers. We check for a volume entry that mounts to /tmp
	// in the container (beyond the overlay file mount).
	lines := strings.Split(content, "\n")
	foundUDSVolume := false
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		// Look for a volume mount that maps to /tmp in the container
		// and is NOT the overlay file mount.
		if strings.Contains(trimmed, ":/tmp") && !strings.Contains(trimmed, "overlay") && !strings.Contains(trimmed, ".json") {
			foundUDSVolume = true
			break
		}
	}
	if !foundUDSVolume {
		t.Error("compose.yml must include a volume mount for UDS socket directory (e.g., /tmp/kuksa:/tmp)")
	}
}

// TestComposeVSSOverlay verifies that the VSS overlay file is mounted and
// referenced in the databroker command args.
//
// Test Spec: TS-02-5
// Requirements: 02-REQ-6.4
func TestComposeVSSOverlay(t *testing.T) {
	content := readCompose(t)

	t.Run("overlay volume mount", func(t *testing.T) {
		if !strings.Contains(content, "vss-overlay.json") {
			t.Error("compose.yml must mount vss-overlay.json into the container")
		}
	})

	t.Run("overlay flag in command", func(t *testing.T) {
		// Check for --vss flag which loads the overlay.
		if !strings.Contains(content, "--vss") {
			t.Error("compose.yml command must include --vss flag to load the VSS overlay")
		}
	})
}

// TestComposePermissiveMode verifies that no authentication/authorization
// flags are present in the databroker command args, ensuring permissive mode.
//
// Test Spec: TS-02-12
// Requirements: 02-REQ-7.1
func TestComposePermissiveMode(t *testing.T) {
	content := readCompose(t)

	authFlags := []string{"--token", "--auth", "--jwt", "--tls-server-cert"}
	for _, flag := range authFlags {
		if strings.Contains(content, flag) {
			t.Errorf("compose.yml contains auth flag %q; databroker must run in permissive mode", flag)
		}
	}
}
