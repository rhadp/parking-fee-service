package cmd

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// TS-09-25: --help should be handled before dispatch.
// Verifying that "help" and "--help" are NOT valid subcommands
// (they are handled in main.go before dispatch, exiting with code 0).
func TestHelpIsNotASubcommand(t *testing.T) {
	helpVariants := []string{"help", "--help", "-h"}
	for _, h := range helpVariants {
		if IsValidSubcommand(h) {
			t.Errorf("%q should not be a valid subcommand (handled in main)", h)
		}
	}
}

// TS-09-E8: Unknown subcommand should produce an error.
func TestUnknownSubcommandError(t *testing.T) {
	unknowns := []string{"foobar", "delete", "", "LOOKUP", "Install"}
	for _, subcmd := range unknowns {
		err := Dispatch(subcmd, nil, "", "", "")
		if err == nil {
			t.Errorf("expected error for unknown subcommand %q", subcmd)
		}
	}
}

// TS-09-E9: Missing required flags produce errors.
func TestMissingRequiredFlags(t *testing.T) {
	tests := []struct {
		name   string
		fn     func() error
		expect string
	}{
		{
			"lookup missing lat/lon",
			func() error { return RunLookup(nil, "http://localhost:8080") },
			"lat",
		},
		{
			"adapter-info missing operator-id",
			func() error { return RunAdapterInfo(nil, "http://localhost:8080") },
			"operator",
		},
		{
			"install missing image-ref",
			func() error { return RunInstall(nil, "localhost:50052") },
			"image",
		},
		{
			"remove missing adapter-id",
			func() error { return RunRemove(nil, "localhost:50052") },
			"adapter",
		},
		{
			"status missing adapter-id",
			func() error { return RunStatus(nil, "localhost:50052") },
			"adapter",
		},
		{
			"start-session missing zone-id",
			func() error { return RunStartSession(nil, "localhost:50053") },
			"zone",
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			err := tc.fn()
			if err == nil {
				t.Fatalf("expected error for %s", tc.name)
			}
			errMsg := strings.ToLower(err.Error())
			if !strings.Contains(errMsg, tc.expect) && !strings.Contains(errMsg, "required") {
				t.Errorf("error should mention %q or 'required', got: %v", tc.expect, err)
			}
		})
	}
}

// TS-09-26: Connection error messages include target address.
func TestConnectionErrorMessage_ParkingApp(t *testing.T) {
	err := RunLookup([]string{"--lat=48.0", "--lon=11.0"}, "http://localhost:19999")
	if err == nil {
		t.Fatal("expected error when service is unreachable")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "19999") && !strings.Contains(strings.ToLower(errMsg), "connection") {
		t.Errorf("error should contain address or 'connection', got: %v", err)
	}
}

// TS-09-27: Upstream error responses are reported.
func TestUpstreamErrorResponse_ParkingApp(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusForbidden)
		w.Write([]byte(`{"error": "access denied"}`))
	}))
	defer ts.Close()

	err := RunLookup([]string{"--lat=48.0", "--lon=11.0"}, ts.URL)
	if err == nil {
		t.Fatal("expected error when server returns 403")
	}
	errMsg := err.Error()
	if !strings.Contains(errMsg, "403") {
		t.Errorf("error should mention HTTP status code 403, got: %v", err)
	}
}

// TS-09-P5: Error Exit Code Consistency property test for parking-app-cli.
// All error conditions produce errors (which translate to exit code 1 in main).
func TestPropertyErrorExitCode(t *testing.T) {
	errorScenarios := []struct {
		name string
		fn   func() error
	}{
		{"unknown subcommand", func() error { return Dispatch("foobar", nil, "", "", "") }},
		{"lookup missing flags", func() error { return RunLookup(nil, "http://localhost:8080") }},
		{"adapter-info missing flags", func() error { return RunAdapterInfo(nil, "http://localhost:8080") }},
		{"install missing flags", func() error { return RunInstall(nil, "localhost:50052") }},
		{"remove missing flags", func() error { return RunRemove(nil, "localhost:50052") }},
		{"status missing flags", func() error { return RunStatus(nil, "localhost:50052") }},
		{"start-session missing flags", func() error { return RunStartSession(nil, "localhost:50053") }},
		{"lookup unreachable", func() error {
			return RunLookup([]string{"--lat=48.0", "--lon=11.0"}, "http://localhost:19999")
		}},
	}

	for _, tc := range errorScenarios {
		t.Run(tc.name, func(t *testing.T) {
			err := tc.fn()
			if err == nil {
				t.Errorf("expected error for scenario %q", tc.name)
			}
		})
	}
}
