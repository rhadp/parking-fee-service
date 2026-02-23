// Package config handles environment-based configuration for the parking fee service.
package config

import (
	"os"
	"strconv"
	"strings"
)

// Config holds the service configuration.
type Config struct {
	Port                string
	OperatorsConfigPath string
	FuzzinessMeters     float64
	AuthTokens          []string
}

// LoadConfig reads configuration from environment variables and applies
// defaults.
//
// Environment variables:
//   - PORT: HTTP listen port (default "8080")
//   - OPERATORS_CONFIG: path to JSON config file (default "" = use embedded data)
//   - FUZZINESS_METERS: near-zone fuzziness buffer in meters (default 100)
//   - AUTH_TOKENS: comma-separated list of valid bearer tokens (default "demo-token-1")
func LoadConfig() Config {
	cfg := Config{
		Port:            "8080",
		FuzzinessMeters: 100,
		AuthTokens:      []string{"demo-token-1"},
	}

	if port := os.Getenv("PORT"); port != "" {
		cfg.Port = port
	}

	if configPath := os.Getenv("OPERATORS_CONFIG"); configPath != "" {
		cfg.OperatorsConfigPath = configPath
	}

	if fuzz := os.Getenv("FUZZINESS_METERS"); fuzz != "" {
		if v, err := strconv.ParseFloat(fuzz, 64); err == nil {
			cfg.FuzzinessMeters = v
		}
	}

	if tokens := os.Getenv("AUTH_TOKENS"); tokens != "" {
		parts := strings.Split(tokens, ",")
		trimmed := make([]string, 0, len(parts))
		for _, p := range parts {
			t := strings.TrimSpace(p)
			if t != "" {
				trimmed = append(trimmed, t)
			}
		}
		if len(trimmed) > 0 {
			cfg.AuthTokens = trimmed
		}
	}

	return cfg
}
