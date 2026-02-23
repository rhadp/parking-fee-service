//go:build integration

package integration

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"testing"
)

// ===========================================================================
// TS-05-I1: Mock CLI operator lookup
// ===========================================================================

func TestIntegration_CLILookup(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires PARKING_FEE_SERVICE running and mock CLI implemented — will be enabled in task group 7")

	binary := cliBinary(t)

	// The test server URL would normally come from startTestPFS(t)
	pfsURL := "http://localhost:8080"

	stdout, _, exitCode := execCommand(t, binary,
		"lookup",
		"--lat=48.1351",
		"--lon=11.5750",
		fmt.Sprintf("--pfs-url=%s", pfsURL),
		"--token=demo-token-1",
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if !strings.Contains(stdout, "Munich City Parking") {
		t.Error("expected output to contain 'Munich City Parking'")
	}
	if !strings.Contains(stdout, "2.50") {
		t.Error("expected output to contain rate '2.50'")
	}
}

// ===========================================================================
// TS-05-I2: Mock CLI adapter metadata retrieval
// ===========================================================================

func TestIntegration_CLIAdapter(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires PARKING_FEE_SERVICE running and mock CLI implemented — will be enabled in task group 7")

	binary := cliBinary(t)

	pfsURL := "http://localhost:8080"

	stdout, _, exitCode := execCommand(t, binary,
		"adapter",
		"--operator-id=op-munich-01",
		fmt.Sprintf("--pfs-url=%s", pfsURL),
		"--token=demo-token-1",
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}
	if !strings.Contains(stdout, "us-docker.pkg.dev") {
		t.Error("expected output to contain OCI image reference")
	}
	if !strings.Contains(stdout, "1.0.0") {
		t.Error("expected output to contain version '1.0.0'")
	}
}

// ===========================================================================
// TS-05-I3: Full discovery flow (HTTP-level integration)
// ===========================================================================

func TestIntegration_FullDiscoveryFlow(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	t.Skip("requires PARKING_FEE_SERVICE running — will be enabled in task group 7")

	pfsURL := "http://localhost:8080"
	token := "demo-token-1"

	// Step 1: Lookup operators by location
	lookupURL := fmt.Sprintf("%s/operators?lat=48.1351&lon=11.5750", pfsURL)
	req1, err := http.NewRequest("GET", lookupURL, nil)
	if err != nil {
		t.Fatalf("failed to create request: %v", err)
	}
	req1.Header.Set("Authorization", "Bearer "+token)

	resp1, err := http.DefaultClient.Do(req1)
	if err != nil {
		t.Fatalf("failed to send lookup request: %v", err)
	}
	defer resp1.Body.Close()

	if resp1.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200 for lookup, got %d", resp1.StatusCode)
	}

	body1, _ := io.ReadAll(resp1.Body)
	var lookupResult struct {
		Operators []struct {
			OperatorID string `json:"operator_id"`
		} `json:"operators"`
	}
	if err := json.Unmarshal(body1, &lookupResult); err != nil {
		t.Fatalf("failed to parse lookup response: %v", err)
	}

	if len(lookupResult.Operators) < 1 {
		t.Fatal("expected at least 1 operator from lookup")
	}

	// Step 2: Get adapter metadata for the first discovered operator
	opID := lookupResult.Operators[0].OperatorID
	adapterURL := fmt.Sprintf("%s/operators/%s/adapter", pfsURL, opID)
	req2, err := http.NewRequest("GET", adapterURL, nil)
	if err != nil {
		t.Fatalf("failed to create adapter request: %v", err)
	}
	req2.Header.Set("Authorization", "Bearer "+token)

	resp2, err := http.DefaultClient.Do(req2)
	if err != nil {
		t.Fatalf("failed to send adapter request: %v", err)
	}
	defer resp2.Body.Close()

	if resp2.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200 for adapter, got %d", resp2.StatusCode)
	}

	body2, _ := io.ReadAll(resp2.Body)
	var adapterResult struct {
		ImageRef       string `json:"image_ref"`
		ChecksumSHA256 string `json:"checksum_sha256"`
		Version        string `json:"version"`
	}
	if err := json.Unmarshal(body2, &adapterResult); err != nil {
		t.Fatalf("failed to parse adapter response: %v", err)
	}

	if adapterResult.ImageRef == "" {
		t.Error("image_ref is empty in adapter response")
	}
	if adapterResult.ChecksumSHA256 == "" {
		t.Error("checksum_sha256 is empty in adapter response")
	}
	if adapterResult.Version == "" {
		t.Error("version is empty in adapter response")
	}
}
