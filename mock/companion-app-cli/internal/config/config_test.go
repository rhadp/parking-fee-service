package config

import (
	"os"
	"testing"
)

// TS-09-22: companion-app-cli defaults to CLOUD_GATEWAY_URL=http://localhost:8081.
func TestCloudGatewayURL_Default(t *testing.T) {
	os.Unsetenv("CLOUD_GATEWAY_URL")
	url := CloudGatewayURL()
	if url != "http://localhost:8081" {
		t.Errorf("expected default http://localhost:8081, got %q", url)
	}
}

// TS-09-22: CLOUD_GATEWAY_URL env var overrides default.
func TestCloudGatewayURL_Override(t *testing.T) {
	os.Setenv("CLOUD_GATEWAY_URL", "http://custom:9999")
	defer os.Unsetenv("CLOUD_GATEWAY_URL")
	url := CloudGatewayURL()
	if url != "http://custom:9999" {
		t.Errorf("expected http://custom:9999, got %q", url)
	}
}

// TS-09-22: BearerToken reads from CLOUD_GATEWAY_TOKEN.
func TestBearerToken_FromEnv(t *testing.T) {
	os.Unsetenv("CLOUD_GATEWAY_TOKEN")
	token := BearerToken()
	if token != "" {
		t.Errorf("expected empty token when env not set, got %q", token)
	}

	os.Setenv("CLOUD_GATEWAY_TOKEN", "my-secret-token")
	defer os.Unsetenv("CLOUD_GATEWAY_TOKEN")
	token = BearerToken()
	if token != "my-secret-token" {
		t.Errorf("expected my-secret-token, got %q", token)
	}
}

// TS-09-P4: Configuration defaults property test.
func TestPropertyConfigDefaults(t *testing.T) {
	// Clear env vars
	os.Unsetenv("CLOUD_GATEWAY_URL")
	os.Unsetenv("CLOUD_GATEWAY_TOKEN")

	// Check defaults
	if url := CloudGatewayURL(); url != DefaultCloudGatewayURL {
		t.Errorf("expected default %q, got %q", DefaultCloudGatewayURL, url)
	}
	if token := BearerToken(); token != "" {
		t.Errorf("expected empty default token, got %q", token)
	}

	// Check overrides
	customURL := "http://override:1234"
	os.Setenv("CLOUD_GATEWAY_URL", customURL)
	defer os.Unsetenv("CLOUD_GATEWAY_URL")
	if url := CloudGatewayURL(); url != customURL {
		t.Errorf("expected override %q, got %q", customURL, url)
	}
}
