package main

import (
	"os"
	"testing"
)

func TestParseGlobalFlagsDefaults(t *testing.T) {
	os.Unsetenv("GATEWAY_ADDR")
	os.Unsetenv("VIN")
	os.Unsetenv("TOKEN")

	remaining, err := parseGlobalFlags([]string{"status"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if gatewayAddr != "http://localhost:8081" {
		t.Errorf("expected default gateway-addr 'http://localhost:8081', got %q", gatewayAddr)
	}
	if token != "demo-token" {
		t.Errorf("expected default token 'demo-token', got %q", token)
	}
	if vin != "" {
		t.Errorf("expected empty vin by default, got %q", vin)
	}
	if len(remaining) != 1 || remaining[0] != "status" {
		t.Errorf("expected remaining [status], got %v", remaining)
	}
}

func TestParseGlobalFlagsCustom(t *testing.T) {
	os.Unsetenv("GATEWAY_ADDR")
	os.Unsetenv("VIN")
	os.Unsetenv("TOKEN")

	remaining, err := parseGlobalFlags([]string{
		"--gateway-addr", "http://10.0.0.1:8081",
		"--vin", "WBA12345678901234",
		"--token", "my-secret-token",
		"lock",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if gatewayAddr != "http://10.0.0.1:8081" {
		t.Errorf("expected gateway-addr 'http://10.0.0.1:8081', got %q", gatewayAddr)
	}
	if vin != "WBA12345678901234" {
		t.Errorf("expected vin 'WBA12345678901234', got %q", vin)
	}
	if token != "my-secret-token" {
		t.Errorf("expected token 'my-secret-token', got %q", token)
	}
	if len(remaining) != 1 || remaining[0] != "lock" {
		t.Errorf("expected remaining [lock], got %v", remaining)
	}
}

func TestParseGlobalFlagsFromEnv(t *testing.T) {
	t.Setenv("GATEWAY_ADDR", "http://envhost:8081")
	t.Setenv("VIN", "ENVVIN123456789")
	t.Setenv("TOKEN", "env-token")

	remaining, err := parseGlobalFlags([]string{"unlock"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if gatewayAddr != "http://envhost:8081" {
		t.Errorf("expected gateway-addr from env 'http://envhost:8081', got %q", gatewayAddr)
	}
	if vin != "ENVVIN123456789" {
		t.Errorf("expected vin from env 'ENVVIN123456789', got %q", vin)
	}
	if token != "env-token" {
		t.Errorf("expected token from env 'env-token', got %q", token)
	}
	if len(remaining) != 1 || remaining[0] != "unlock" {
		t.Errorf("expected remaining [unlock], got %v", remaining)
	}
}

func TestRunNoArgs(t *testing.T) {
	os.Unsetenv("GATEWAY_ADDR")
	os.Unsetenv("VIN")
	os.Unsetenv("TOKEN")

	err := run(nil)
	if err == nil {
		t.Fatal("expected error for no args")
	}
	if err.Error() != "no command specified" {
		t.Errorf("expected 'no command specified', got %q", err.Error())
	}
}

func TestRunVINRequired(t *testing.T) {
	os.Unsetenv("VIN")

	err := run([]string{"lock"})
	if err == nil {
		t.Fatal("expected error when --vin is missing")
	}
	if err.Error() != "--vin is required" {
		t.Errorf("expected '--vin is required', got %q", err.Error())
	}
}

func TestRunUnknownCommand(t *testing.T) {
	os.Unsetenv("VIN")

	err := run([]string{"--vin", "TEST123", "nonexistent"})
	if err == nil {
		t.Fatal("expected error for unknown command")
	}
	expected := "unknown command: nonexistent"
	if err.Error() != expected {
		t.Errorf("expected %q, got %q", expected, err.Error())
	}
}

func TestRunHelp(t *testing.T) {
	os.Unsetenv("VIN")

	err := run([]string{"help"})
	if err != nil {
		t.Fatalf("unexpected error for help command: %v", err)
	}
}

func TestEnvOrDefault(t *testing.T) {
	t.Setenv("TEST_COMPANION_VAR", "custom")
	if v := envOrDefault("TEST_COMPANION_VAR", "default"); v != "custom" {
		t.Errorf("expected 'custom', got %q", v)
	}

	os.Unsetenv("TEST_COMPANION_VAR_UNSET")
	if v := envOrDefault("TEST_COMPANION_VAR_UNSET", "fallback"); v != "fallback" {
		t.Errorf("expected 'fallback', got %q", v)
	}
}

// TestAllSubcommandsRecognized verifies that all expected subcommands are
// dispatched correctly (they will fail at the HTTP request level, but
// command parsing should work).
func TestAllSubcommandsRecognized(t *testing.T) {
	subcommands := []string{"lock", "unlock", "status"}

	for _, cmd := range subcommands {
		t.Run(cmd, func(t *testing.T) {
			os.Unsetenv("VIN")
			os.Unsetenv("GATEWAY_ADDR")
			os.Unsetenv("TOKEN")

			// Use an unreachable address so requests fail fast.
			err := run([]string{
				"--gateway-addr", "http://localhost:1",
				"--vin", "TESTVIN123456789",
				cmd,
			})
			// We expect a connection error, not an "unknown command" error.
			if err == nil {
				return
			}
			if err.Error() == "unknown command: "+cmd {
				t.Errorf("command %q was not recognized", cmd)
			}
		})
	}
}
