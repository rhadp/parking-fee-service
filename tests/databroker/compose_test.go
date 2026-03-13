package databroker

import (
	"strings"
	"testing"
)

// TS-02-1: TCP Listener Configuration
// Requirement: 02-REQ-1.1
//
// Note: the test_spec.md pseudocode checks for "--address" in command args.
// The actual kuksa-databroker flag is indeed "--address", consistent with spec.
func TestComposeTCPListener(t *testing.T) {
	content := readCompose(t)

	// Port mapping 55556:55555 must be present.
	if !strings.Contains(content, "55556:55555") {
		t.Error("compose.yml: missing port mapping 55556:55555 for TCP listener")
	}

	// --address flag must appear in databroker command args.
	if !strings.Contains(content, "--address") {
		t.Error("compose.yml: missing --address flag in databroker command")
	}

	// The address value must be 0.0.0.0:55555.
	if !strings.Contains(content, "0.0.0.0:55555") {
		t.Error("compose.yml: databroker command does not include 0.0.0.0:55555")
	}
}

// TS-02-2: UDS Listener Configuration
// Requirement: 02-REQ-1.2
//
// Errata: the spec test_spec.md checks for "--uds-path" but the actual
// kuksa-databroker flag is "--unix-socket". See docs/errata/02_data_broker_compose_flags.md.
func TestComposeUDSListener(t *testing.T) {
	content := readCompose(t)

	// --unix-socket flag configures the UDS listener path in kuksa-databroker
	// v0.5.0+ (the spec incorrectly named this --uds-path).
	if !strings.Contains(content, "--unix-socket") {
		t.Error("compose.yml: missing --unix-socket flag in databroker command (spec erratum: spec says --uds-path)")
	}

	// The UDS socket must be placed at /tmp/kuksa-databroker.sock inside the container.
	if !strings.Contains(content, "/tmp/kuksa-databroker.sock") {
		t.Error("compose.yml: databroker command does not include UDS socket path /tmp/kuksa-databroker.sock")
	}
}

// TS-02-3: UDS Volume Mount
// Requirement: 02-REQ-1.3
func TestComposeUDSVolume(t *testing.T) {
	content := readCompose(t)

	// The compose file must expose the UDS socket directory to the host so that
	// same-host consumers can connect. We accept either:
	//   - a named volume "kuksa-uds" with a bind-mount device
	//   - a direct host-path volume mapping
	// In either case, the host path /tmp/kuksa must be referenced.
	if !strings.Contains(content, "kuksa-uds") && !strings.Contains(content, "/tmp/kuksa") {
		t.Error("compose.yml: no UDS volume (named volume 'kuksa-uds' or /tmp/kuksa host mount) found")
	}

	// A volume must be mapped to /tmp inside the container (where the socket lives).
	if !strings.Contains(content, ":/tmp") && !strings.Contains(content, "/tmp\n") && !strings.Contains(content, "/tmp\"") {
		t.Error("compose.yml: no volume mapped to container path /tmp for UDS socket access")
	}
}

// TS-02-5: Image Version Pinning
// Requirement: 02-REQ-2.1
//
// Errata: the spec requires :0.5.1 but that tag does not exist in the registry.
// The nearest available version is :0.5.0. See docs/errata/02_data_broker_compose_flags.md.
func TestComposeImageVersion(t *testing.T) {
	content := readCompose(t)

	// The image must be pinned to a specific version (not :latest).
	// The nearest available 0.5.x release to the spec's :0.5.1 is :0.5.0.
	const pinnedImage = "ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0"
	if !strings.Contains(content, pinnedImage) {
		t.Errorf("compose.yml: databroker image is not pinned to %q (spec erratum: spec says :0.5.1 which does not exist)", pinnedImage)
	}

	// Ensure :latest is not used for the databroker image.
	if strings.Contains(content, "kuksa-databroker:latest") {
		t.Error("compose.yml: databroker image must not use :latest tag")
	}
}
