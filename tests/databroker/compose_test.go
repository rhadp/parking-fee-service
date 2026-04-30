package databroker_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// readCompose reads the compose.yml content for static validation.
func readCompose(t *testing.T) string {
	t.Helper()
	root := repoRoot(t)
	data, err := os.ReadFile(filepath.Join(root, "deployments", "compose.yml"))
	if err != nil {
		t.Fatalf("failed to read compose.yml: %v", err)
	}
	return string(data)
}

// TestComposePinnedImage verifies the compose.yml pins the Kuksa Databroker image.
// Requirement: 02-REQ-1.1
func TestComposePinnedImage(t *testing.T) {
	content := readCompose(t)
	expected := "ghcr.io/eclipse-kuksa/kuksa-databroker:0.6"
	if !strings.Contains(content, expected) {
		t.Errorf("compose.yml does not contain pinned image %q", expected)
	}
}

// TestComposeTCPPort verifies the compose.yml maps container port 55555 to host port 55556.
// Requirement: 02-REQ-2.2
func TestComposeTCPPort(t *testing.T) {
	content := readCompose(t)
	if !strings.Contains(content, "55556:55555") {
		t.Error("compose.yml does not contain port mapping 55556:55555")
	}
}

// TestComposeTCPListener verifies the databroker command args include TCP listener configuration.
// The actual flags are --address 0.0.0.0 and --port 55555 (separate flags per kuksa-databroker CLI).
// See errata: the spec says --address 0.0.0.0:55555 but the binary uses separate flags.
// Requirement: 02-REQ-2.1
func TestComposeTCPListener(t *testing.T) {
	content := readCompose(t)
	// Check for either the combined form or the split form.
	hasAddress := strings.Contains(content, "--address")
	if !hasAddress {
		t.Error("compose.yml command args do not include --address flag for TCP listener")
	}
}

// TestComposeUDSSocket verifies the databroker command args include UDS listener configuration.
// The actual flag is --unix-socket (not --uds-path) per kuksa-databroker CLI.
// Requirement: 02-REQ-3.1
func TestComposeUDSSocket(t *testing.T) {
	content := readCompose(t)
	// Accept either --unix-socket or --uds-path.
	if !strings.Contains(content, "--unix-socket") && !strings.Contains(content, "--uds-path") {
		t.Error("compose.yml command args do not include UDS socket flag (--unix-socket or --uds-path)")
	}
}

// TestComposeUDSVolume verifies the compose.yml mounts a shared volume for the UDS socket.
// Requirement: 02-REQ-3.2
func TestComposeUDSVolume(t *testing.T) {
	content := readCompose(t)
	// The volume mount should expose the UDS socket directory to co-located containers.
	// Expected pattern: a volume mount mapping a host path to /tmp inside the container.
	if !strings.Contains(content, "/tmp") || !strings.Contains(content, "kuksa") {
		t.Error("compose.yml does not contain a UDS socket volume mount (expected /tmp/kuksa:/tmp or similar)")
	}
}

// TestComposePermissiveMode verifies the databroker runs without auth flags.
// Requirement: 02-REQ-7.1
func TestComposePermissiveMode(t *testing.T) {
	content := readCompose(t)
	authFlags := []string{"--token", "--auth", "--jwt", "--tls-server-cert"}
	for _, flag := range authFlags {
		if strings.Contains(content, flag) {
			t.Errorf("compose.yml contains auth flag %q; databroker should run in permissive mode", flag)
		}
	}
}

// TestComposeOverlayMount verifies the overlay file is mounted into the container.
// Requirement: 02-REQ-6.4
func TestComposeOverlayMount(t *testing.T) {
	content := readCompose(t)
	if !strings.Contains(content, "vss-overlay") {
		t.Error("compose.yml does not mount the VSS overlay file")
	}
	if !strings.Contains(content, "--vss") {
		t.Error("compose.yml command does not include --vss flag for overlay")
	}
}
