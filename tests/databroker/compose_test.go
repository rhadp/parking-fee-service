// Static compose.yml configuration tests for the DATA_BROKER service.
// These tests verify the compose.yml structure without requiring a running container.
// Tests in this file will FAIL until task group 2 updates deployments/compose.yml
// for dual listeners, UDS volume, and the correct overlay flags.
package databroker

import (
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
