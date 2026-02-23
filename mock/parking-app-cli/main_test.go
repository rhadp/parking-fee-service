package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/spf13/cobra"
)

func TestRootCmdExists(t *testing.T) {
	if rootCmd == nil {
		t.Fatal("expected rootCmd to be defined")
	}
	if rootCmd.Use != "parking-app-cli" {
		t.Errorf("expected rootCmd.Use to be 'parking-app-cli', got %q", rootCmd.Use)
	}
}

// TestCLI_CommandsRegistered verifies all expected commands are registered on
// the root command.
func TestCLI_CommandsRegistered(t *testing.T) {
	expected := []string{
		"lookup", "adapter", "install", "watch", "list", "status",
		"start-session", "stop-session", "get-status", "get-rate",
	}

	commands := rootCmd.Commands()
	names := make(map[string]bool)
	for _, cmd := range commands {
		names[cmd.Use] = true
	}

	for _, name := range expected {
		if !names[name] {
			t.Errorf("expected command %q to be registered", name)
		}
	}
}

// TestCLI_InstallFlags verifies the install command has the expected flags.
func TestCLI_InstallFlags(t *testing.T) {
	f := installCmd.Flags()
	if f.Lookup("image-ref") == nil {
		t.Error("expected install command to have --image-ref flag")
	}
	if f.Lookup("checksum") == nil {
		t.Error("expected install command to have --checksum flag")
	}
}

// TestCLI_LookupFlags verifies the lookup command has the expected flags.
func TestCLI_LookupFlags(t *testing.T) {
	f := lookupCmd.Flags()
	if f.Lookup("lat") == nil {
		t.Error("expected lookup command to have --lat flag")
	}
	if f.Lookup("lon") == nil {
		t.Error("expected lookup command to have --lon flag")
	}
}

// TestCLI_AdapterFlags verifies the adapter command has the expected flags.
func TestCLI_AdapterFlags(t *testing.T) {
	f := adapterCmd.Flags()
	if f.Lookup("operator-id") == nil {
		t.Error("expected adapter command to have --operator-id flag")
	}
}

// TestCLI_GlobalFlags verifies the global flags exist on rootCmd.
func TestCLI_GlobalFlags(t *testing.T) {
	f := rootCmd.PersistentFlags()
	for _, name := range []string{"pfs-url", "token", "update-addr", "adaptor-addr"} {
		if f.Lookup(name) == nil {
			t.Errorf("expected global flag --%s to be defined", name)
		}
	}
}

// TestCLI_StartSessionFlags verifies the start-session command has the expected flags.
func TestCLI_StartSessionFlags(t *testing.T) {
	f := startSessionCmd.Flags()
	if f.Lookup("vehicle-id") == nil {
		t.Error("expected start-session command to have --vehicle-id flag")
	}
	if f.Lookup("zone-id") == nil {
		t.Error("expected start-session command to have --zone-id flag")
	}
}

// TestCLI_StopSessionFlags verifies the stop-session command has the expected flags.
func TestCLI_StopSessionFlags(t *testing.T) {
	f := stopSessionCmd.Flags()
	if f.Lookup("session-id") == nil {
		t.Error("expected stop-session command to have --session-id flag")
	}
}

// TestCLI_SilenceSettings verifies that commands silence usage and errors
// to allow main() to handle error printing.
func TestCLI_SilenceSettings(t *testing.T) {
	cmds := []*cobra.Command{
		lookupCmd, adapterCmd,
		installCmd, watchCmd, listCmd, statusCmd,
		startSessionCmd, stopSessionCmd, getStatusCmd, getRateCmd,
	}
	for _, cmd := range cmds {
		if !cmd.SilenceUsage {
			t.Errorf("expected %q to have SilenceUsage=true", cmd.Use)
		}
		if !cmd.SilenceErrors {
			t.Errorf("expected %q to have SilenceErrors=true", cmd.Use)
		}
	}
}

// --- Lookup command tests ---

// withPFSState saves and restores package-level PFS state for tests.
func withPFSState(t *testing.T, url, token string) {
	t.Helper()
	origPfsURL := pfsURL
	origToken := pfsToken
	origLat := lookupLat
	origLon := lookupLon
	origOpID := adapterOperatorID
	t.Cleanup(func() {
		pfsURL = origPfsURL
		pfsToken = origToken
		lookupLat = origLat
		lookupLon = origLon
		adapterOperatorID = origOpID
	})
	pfsURL = url
	pfsToken = token
}

// TestCLI_Lookup_Success verifies the lookup command sends a correct HTTP
// request and prints operator results in human-readable format.
func TestCLI_Lookup_Success(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/operators" {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}
		if r.URL.Query().Get("lat") != "48.1351" {
			t.Errorf("unexpected lat: %s", r.URL.Query().Get("lat"))
		}
		if r.URL.Query().Get("lon") != "11.575" {
			t.Errorf("unexpected lon: %s", r.URL.Query().Get("lon"))
		}
		auth := r.Header.Get("Authorization")
		if auth != "Bearer test-token" {
			t.Errorf("unexpected auth header: %s", auth)
		}

		resp := operatorsResponse{
			Operators: []operatorResult{
				{
					OperatorID: "op-munich-01",
					Name:       "Munich City Parking",
					Zone: zoneResult{
						ZoneID: "zone-munich-center",
						Name:   "Munich City Center",
					},
					Rate: rateResult{
						AmountPerHour: 2.50,
						Currency:      "EUR",
					},
				},
			},
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}))
	defer srv.Close()

	withPFSState(t, srv.URL, "test-token")
	lookupLat = 48.1351
	lookupLon = 11.575

	var buf bytes.Buffer
	lookupCmd.SetOut(&buf)
	t.Cleanup(func() { lookupCmd.SetOut(nil) })

	err := runLookup(lookupCmd, nil)
	if err != nil {
		t.Fatalf("lookup failed: %v", err)
	}

	output := buf.String()
	if !strings.Contains(output, "Found 1 operator(s)") {
		t.Errorf("expected output to contain 'Found 1 operator(s)', got: %s", output)
	}
	if !strings.Contains(output, "Munich City Parking") {
		t.Errorf("expected output to contain 'Munich City Parking', got: %s", output)
	}
	if !strings.Contains(output, "op-munich-01") {
		t.Errorf("expected output to contain 'op-munich-01', got: %s", output)
	}
	if !strings.Contains(output, "Munich City Center") {
		t.Errorf("expected output to contain 'Munich City Center', got: %s", output)
	}
	if !strings.Contains(output, "2.50") {
		t.Errorf("expected output to contain '2.50', got: %s", output)
	}
	if !strings.Contains(output, "EUR") {
		t.Errorf("expected output to contain 'EUR', got: %s", output)
	}
}

// TestCLI_Lookup_NoMatches verifies lookup with no matching operators prints
// a count of zero.
func TestCLI_Lookup_NoMatches(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(operatorsResponse{Operators: []operatorResult{}})
	}))
	defer srv.Close()

	withPFSState(t, srv.URL, "demo-token-1")
	lookupLat = 0
	lookupLon = 0

	var buf bytes.Buffer
	lookupCmd.SetOut(&buf)
	t.Cleanup(func() { lookupCmd.SetOut(nil) })

	err := runLookup(lookupCmd, nil)
	if err != nil {
		t.Fatalf("lookup failed: %v", err)
	}

	output := buf.String()
	if !strings.Contains(output, "Found 0 operator(s)") {
		t.Errorf("expected output to contain 'Found 0 operator(s)', got: %s", output)
	}
}

// TestCLI_Lookup_MultipleOperators verifies lookup with multiple matching
// operators prints all of them.
func TestCLI_Lookup_MultipleOperators(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		resp := operatorsResponse{
			Operators: []operatorResult{
				{
					OperatorID: "op-1",
					Name:       "Operator One",
					Zone:       zoneResult{ZoneID: "z1", Name: "Zone 1"},
					Rate:       rateResult{AmountPerHour: 1.50, Currency: "EUR"},
				},
				{
					OperatorID: "op-2",
					Name:       "Operator Two",
					Zone:       zoneResult{ZoneID: "z2", Name: "Zone 2"},
					Rate:       rateResult{AmountPerHour: 3.00, Currency: "EUR"},
				},
			},
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}))
	defer srv.Close()

	withPFSState(t, srv.URL, "demo-token-1")
	lookupLat = 48.135
	lookupLon = 11.575

	var buf bytes.Buffer
	lookupCmd.SetOut(&buf)
	t.Cleanup(func() { lookupCmd.SetOut(nil) })

	err := runLookup(lookupCmd, nil)
	if err != nil {
		t.Fatalf("lookup failed: %v", err)
	}

	output := buf.String()
	if !strings.Contains(output, "Found 2 operator(s)") {
		t.Errorf("expected output to contain 'Found 2 operator(s)', got: %s", output)
	}
	if !strings.Contains(output, "Operator One") {
		t.Errorf("expected output to contain 'Operator One', got: %s", output)
	}
	if !strings.Contains(output, "Operator Two") {
		t.Errorf("expected output to contain 'Operator Two', got: %s", output)
	}
}

// TestCLI_Lookup_HTTPError verifies lookup returns an error when the server
// responds with an error status.
func TestCLI_Lookup_HTTPError(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		json.NewEncoder(w).Encode(pfsErrorResponse{Error: "invalid token"})
	}))
	defer srv.Close()

	withPFSState(t, srv.URL, "wrong-token")
	lookupLat = 48.135
	lookupLon = 11.575

	err := runLookup(lookupCmd, nil)
	if err == nil {
		t.Fatal("expected lookup to fail with HTTP error")
	}
	if !strings.Contains(err.Error(), "invalid token") {
		t.Errorf("expected error to contain 'invalid token', got: %v", err)
	}
}

// TestCLI_Lookup_AuthHeaderSent verifies the lookup command includes the
// authorization header with the configured token.
func TestCLI_Lookup_AuthHeaderSent(t *testing.T) {
	var receivedAuth string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedAuth = r.Header.Get("Authorization")
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(operatorsResponse{Operators: []operatorResult{}})
	}))
	defer srv.Close()

	withPFSState(t, srv.URL, "my-secret-token")
	lookupLat = 0
	lookupLon = 0

	var buf bytes.Buffer
	lookupCmd.SetOut(&buf)
	t.Cleanup(func() { lookupCmd.SetOut(nil) })

	err := runLookup(lookupCmd, nil)
	if err != nil {
		t.Fatalf("lookup failed: %v", err)
	}
	if receivedAuth != "Bearer my-secret-token" {
		t.Errorf("expected auth header 'Bearer my-secret-token', got: %s", receivedAuth)
	}
}

// --- Adapter command tests ---

// TestCLI_Adapter_Success verifies the adapter command sends a correct HTTP
// request and prints adapter metadata.
func TestCLI_Adapter_Success(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/operators/op-munich-01/adapter" {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}
		auth := r.Header.Get("Authorization")
		if auth != "Bearer test-token" {
			t.Errorf("unexpected auth header: %s", auth)
		}

		resp := adapterMetadataResponse{
			OperatorID:     "op-munich-01",
			ImageRef:       "us-docker.pkg.dev/rhadp-demo/adapters/munich-parking:v1.0.0",
			ChecksumSHA256: "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
			Version:        "1.0.0",
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}))
	defer srv.Close()

	withPFSState(t, srv.URL, "test-token")
	adapterOperatorID = "op-munich-01"

	var buf bytes.Buffer
	adapterCmd.SetOut(&buf)
	t.Cleanup(func() { adapterCmd.SetOut(nil) })

	err := runAdapter(adapterCmd, nil)
	if err != nil {
		t.Fatalf("adapter failed: %v", err)
	}

	output := buf.String()
	if !strings.Contains(output, "Adapter metadata for operator op-munich-01") {
		t.Errorf("expected output to contain header, got: %s", output)
	}
	if !strings.Contains(output, "us-docker.pkg.dev/rhadp-demo/adapters/munich-parking:v1.0.0") {
		t.Errorf("expected output to contain image ref, got: %s", output)
	}
	if !strings.Contains(output, "sha256:a1b2c3d4") {
		t.Errorf("expected output to contain checksum, got: %s", output)
	}
	if !strings.Contains(output, "1.0.0") {
		t.Errorf("expected output to contain version, got: %s", output)
	}
}

// TestCLI_Adapter_NotFound verifies the adapter command returns an error when
// the operator is not found.
func TestCLI_Adapter_NotFound(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(pfsErrorResponse{Error: "operator not found: op-nonexistent"})
	}))
	defer srv.Close()

	withPFSState(t, srv.URL, "demo-token-1")
	adapterOperatorID = "op-nonexistent"

	err := runAdapter(adapterCmd, nil)
	if err == nil {
		t.Fatal("expected adapter to fail for unknown operator")
	}
	if !strings.Contains(err.Error(), "operator not found") {
		t.Errorf("expected error to contain 'operator not found', got: %v", err)
	}
}

// TestCLI_Adapter_MissingOperatorID verifies the adapter command returns an
// error when --operator-id is not provided.
func TestCLI_Adapter_MissingOperatorID(t *testing.T) {
	withPFSState(t, "http://localhost:9999", "demo-token-1")
	adapterOperatorID = ""

	err := runAdapter(adapterCmd, nil)
	if err == nil {
		t.Fatal("expected adapter to fail without --operator-id")
	}
	if !strings.Contains(err.Error(), "--operator-id is required") {
		t.Errorf("expected error about missing --operator-id, got: %v", err)
	}
}

// TestCLI_Adapter_AuthHeaderSent verifies the adapter command includes the
// authorization header with the configured token.
func TestCLI_Adapter_AuthHeaderSent(t *testing.T) {
	var receivedAuth string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		receivedAuth = r.Header.Get("Authorization")
		resp := adapterMetadataResponse{
			OperatorID:     "op-test",
			ImageRef:       "image:v1",
			ChecksumSHA256: "sha256:abc",
			Version:        "1.0.0",
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}))
	defer srv.Close()

	withPFSState(t, srv.URL, "auth-token-42")
	adapterOperatorID = "op-test"

	var buf bytes.Buffer
	adapterCmd.SetOut(&buf)
	t.Cleanup(func() { adapterCmd.SetOut(nil) })

	err := runAdapter(adapterCmd, nil)
	if err != nil {
		t.Fatalf("adapter failed: %v", err)
	}
	if receivedAuth != "Bearer auth-token-42" {
		t.Errorf("expected auth header 'Bearer auth-token-42', got: %s", receivedAuth)
	}
}
