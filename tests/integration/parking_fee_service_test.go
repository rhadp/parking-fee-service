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
// Requirement: (integration)
// Description: Verify the mock PARKING_APP CLI `lookup` command calls
// PARKING_FEE_SERVICE and displays results.
// ===========================================================================

func TestIntegration_CLILookup(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	// Start an in-process PARKING_FEE_SERVICE on a random port.
	pfsURL := startPFS(t)

	// Build the mock CLI binary.
	binary := cliBinary(t)

	// Run lookup command with coordinates inside Munich City Center zone.
	stdout, stderr, exitCode := execCommand(t, binary,
		"lookup",
		"--lat=48.1351",
		"--lon=11.5750",
		fmt.Sprintf("--pfs-url=%s", pfsURL),
		"--token=demo-token-1",
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}
	if !strings.Contains(stdout, "Munich City Parking") {
		t.Errorf("expected output to contain 'Munich City Parking', got:\n%s", stdout)
	}
	if !strings.Contains(stdout, "2.50") {
		t.Errorf("expected output to contain rate '2.50', got:\n%s", stdout)
	}
	if !strings.Contains(stdout, "op-munich-01") {
		t.Errorf("expected output to contain operator ID 'op-munich-01', got:\n%s", stdout)
	}
	if !strings.Contains(stdout, "EUR") {
		t.Errorf("expected output to contain currency 'EUR', got:\n%s", stdout)
	}
}

// ===========================================================================
// TS-05-I2: Mock CLI adapter metadata retrieval
// Requirement: (integration)
// Description: Verify the mock PARKING_APP CLI `adapter` command calls
// PARKING_FEE_SERVICE and displays adapter metadata.
// ===========================================================================

func TestIntegration_CLIAdapter(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	// Start an in-process PARKING_FEE_SERVICE on a random port.
	pfsURL := startPFS(t)

	// Build the mock CLI binary.
	binary := cliBinary(t)

	// Run adapter command for a known operator.
	stdout, stderr, exitCode := execCommand(t, binary,
		"adapter",
		"--operator-id=op-munich-01",
		fmt.Sprintf("--pfs-url=%s", pfsURL),
		"--token=demo-token-1",
	)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout: %s\nstderr: %s", exitCode, stdout, stderr)
	}
	if !strings.Contains(stdout, "us-docker.pkg.dev") {
		t.Errorf("expected output to contain OCI image reference 'us-docker.pkg.dev', got:\n%s", stdout)
	}
	if !strings.Contains(stdout, "1.0.0") {
		t.Errorf("expected output to contain version '1.0.0', got:\n%s", stdout)
	}
	if !strings.Contains(stdout, "sha256:") {
		t.Errorf("expected output to contain checksum prefix 'sha256:', got:\n%s", stdout)
	}
	if !strings.Contains(stdout, "op-munich-01") {
		t.Errorf("expected output to contain operator ID 'op-munich-01', got:\n%s", stdout)
	}
}

// ===========================================================================
// TS-05-I3: Full discovery flow (HTTP-level integration)
// Requirement: (integration)
// Description: Verify the end-to-end flow: lookup operators by location,
// then retrieve adapter metadata for a discovered operator.
// ===========================================================================

func TestIntegration_FullDiscoveryFlow(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping integration test in short mode")
	}

	// Start an in-process PARKING_FEE_SERVICE on a random port.
	pfsURL := startPFS(t)
	token := "demo-token-1"

	// Step 1: Lookup operators by location (Munich City Center).
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
		body, _ := io.ReadAll(resp1.Body)
		t.Fatalf("expected status 200 for lookup, got %d: %s", resp1.StatusCode, string(body))
	}

	body1, err := io.ReadAll(resp1.Body)
	if err != nil {
		t.Fatalf("failed to read lookup response body: %v", err)
	}

	var lookupResult struct {
		Operators []struct {
			OperatorID string `json:"operator_id"`
			Name       string `json:"name"`
			Zone       struct {
				ZoneID string `json:"zone_id"`
				Name   string `json:"name"`
			} `json:"zone"`
			Rate struct {
				AmountPerHour float64 `json:"amount_per_hour"`
				Currency      string  `json:"currency"`
			} `json:"rate"`
		} `json:"operators"`
	}
	if err := json.Unmarshal(body1, &lookupResult); err != nil {
		t.Fatalf("failed to parse lookup response: %v\nbody: %s", err, string(body1))
	}

	if len(lookupResult.Operators) < 1 {
		t.Fatal("expected at least 1 operator from lookup")
	}

	// Verify the discovered operator has expected fields.
	op := lookupResult.Operators[0]
	if op.OperatorID == "" {
		t.Error("operator_id is empty in lookup response")
	}
	if op.Name == "" {
		t.Error("name is empty in lookup response")
	}
	if op.Zone.ZoneID == "" {
		t.Error("zone_id is empty in lookup response")
	}
	if op.Rate.AmountPerHour <= 0 {
		t.Errorf("expected positive rate, got %f", op.Rate.AmountPerHour)
	}
	if op.Rate.Currency == "" {
		t.Error("currency is empty in lookup response")
	}

	// Step 2: Get adapter metadata for the first discovered operator.
	opID := op.OperatorID
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
		body, _ := io.ReadAll(resp2.Body)
		t.Fatalf("expected status 200 for adapter, got %d: %s", resp2.StatusCode, string(body))
	}

	body2, err := io.ReadAll(resp2.Body)
	if err != nil {
		t.Fatalf("failed to read adapter response body: %v", err)
	}

	var adapterResult struct {
		OperatorID     string `json:"operator_id"`
		ImageRef       string `json:"image_ref"`
		ChecksumSHA256 string `json:"checksum_sha256"`
		Version        string `json:"version"`
	}
	if err := json.Unmarshal(body2, &adapterResult); err != nil {
		t.Fatalf("failed to parse adapter response: %v\nbody: %s", err, string(body2))
	}

	// Verify the adapter metadata is complete and consistent.
	if adapterResult.OperatorID != opID {
		t.Errorf("expected operator_id %q in adapter response, got %q", opID, adapterResult.OperatorID)
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

	// Step 3: Verify health endpoint is accessible (bonus — confirms full stack).
	healthURL := fmt.Sprintf("%s/health", pfsURL)
	resp3, err := http.Get(healthURL)
	if err != nil {
		t.Fatalf("failed to send health request: %v", err)
	}
	defer resp3.Body.Close()

	if resp3.StatusCode != http.StatusOK {
		t.Fatalf("expected status 200 for health, got %d", resp3.StatusCode)
	}

	body3, _ := io.ReadAll(resp3.Body)
	if !strings.Contains(string(body3), "ok") {
		t.Errorf("expected health response to contain 'ok', got: %s", string(body3))
	}
}
