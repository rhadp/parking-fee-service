package config

import (
	"testing"
)

// --- TS-05-10: Default fuzziness is 100 meters ---

func TestConfig_DefaultFuzziness(t *testing.T) {
	t.Setenv("FUZZINESS_METERS", "")
	cfg := LoadConfig()
	if cfg.FuzzinessMeters != 100 {
		t.Errorf("expected default fuzziness=100, got %f", cfg.FuzzinessMeters)
	}
}

// --- TS-05-11: Fuzziness configurable via environment variable ---

func TestConfig_FuzzinessEnvVar(t *testing.T) {
	t.Setenv("FUZZINESS_METERS", "250")
	cfg := LoadConfig()
	if cfg.FuzzinessMeters != 250 {
		t.Errorf("expected fuzziness=250, got %f", cfg.FuzzinessMeters)
	}
}

// --- TS-05-19: Config file path via environment variable ---

func TestConfig_OperatorsConfigEnvVar(t *testing.T) {
	t.Setenv("OPERATORS_CONFIG", "testdata/operators.json")
	cfg := LoadConfig()
	if cfg.OperatorsConfigPath != "testdata/operators.json" {
		t.Errorf("expected OperatorsConfigPath='testdata/operators.json', got %q", cfg.OperatorsConfigPath)
	}
}

// --- TS-05-23: Auth tokens configurable via environment ---

func TestConfig_AuthTokensEnvVar(t *testing.T) {
	t.Setenv("AUTH_TOKENS", "token-a,token-b")
	cfg := LoadConfig()
	if len(cfg.AuthTokens) != 2 {
		t.Fatalf("expected 2 auth tokens, got %d", len(cfg.AuthTokens))
	}
	if cfg.AuthTokens[0] != "token-a" {
		t.Errorf("expected first token='token-a', got %q", cfg.AuthTokens[0])
	}
	if cfg.AuthTokens[1] != "token-b" {
		t.Errorf("expected second token='token-b', got %q", cfg.AuthTokens[1])
	}
}
