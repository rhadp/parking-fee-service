package databroker_test

import (
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// expectedImageRef is the pinned Kuksa Databroker image reference.
//
// NOTE: The spec (02-REQ-1.1) originally specified tag 0.5.1, but that tag
// does not exist on ghcr.io. The actual published tag is 0.5.0. This constant
// reflects the corrected value; see docs/errata/02_databroker_image_tag.md.
const expectedImageRef = "ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0"

// ---------------------------------------------------------------------------
// TS-02-3: Pinned image version
// Requirement: 02-REQ-1.1, 02-REQ-1.2
// ---------------------------------------------------------------------------

// TestPinnedImageVersionInComposeFile verifies that deployments/compose.yml
// specifies exactly the pinned Kuksa Databroker image reference.
func TestPinnedImageVersionInComposeFile(t *testing.T) {
	root := findRepoRoot(t)
	composePath := filepath.Join(root, "deployments", "compose.yml")

	content, err := readFile(composePath)
	if err != nil {
		t.Fatalf("TS-02-3: failed to read %s: %v", composePath, err)
	}

	if !strings.Contains(content, expectedImageRef) {
		t.Errorf("TS-02-3: compose.yml does not contain expected image reference %q\n"+
			"  hint: pin the image with: image: %s",
			expectedImageRef, expectedImageRef)
	}
}

// TestPinnedImageVersionRunningContainer verifies that the running
// kuksa-databroker container uses the expected pinned image.
// This test is skipped if podman is not available on the PATH.
func TestPinnedImageVersionRunningContainer(t *testing.T) {
	if _, err := exec.LookPath("podman"); err != nil {
		t.Skip("podman not available on PATH; skipping container image inspection")
	}

	// "podman inspect" the running container named "kuksa-databroker".
	out, err := exec.Command(
		"podman", "inspect",
		"--format", "{{.ImageName}}",
		"kuksa-databroker",
	).CombinedOutput()
	if err != nil {
		t.Skipf("TS-02-3: kuksa-databroker container not running (podman inspect: %v); "+
			"start it with: cd deployments && podman compose up -d kuksa-databroker", err)
	}

	imageName := strings.TrimSpace(string(out))
	if !strings.Contains(imageName, "kuksa-databroker:0.5.0") {
		t.Errorf("TS-02-3: running container image %q does not match expected %q",
			imageName, expectedImageRef)
	}
}

// readFile is a small helper to read an entire file as a string.
func readFile(path string) (string, error) {
	data, err := exec.Command("cat", path).Output()
	if err != nil {
		return "", err
	}
	return string(data), nil
}
