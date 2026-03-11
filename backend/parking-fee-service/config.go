package main

import (
	_ "embed"
	"encoding/json"
	"fmt"
	"os"
)

//go:embed config.json
var defaultConfigJSON []byte

// Config represents the full service configuration.
type Config struct {
	Settings  Settings         `json:"settings"`
	Zones     []Zone           `json:"zones"`
	Operators []OperatorConfig `json:"operators"`
}

// Settings contains service-level configuration.
type Settings struct {
	Port                     int     `json:"port"`
	ProximityThresholdMeters float64 `json:"proximity_threshold_meters"`
}

// OperatorConfig extends Operator with adapter metadata (used in config file).
type OperatorConfig struct {
	Operator
	Adapter AdapterMetadata `json:"adapter"`
}

// LoadConfig loads configuration from filePath, or falls back to embedded default if filePath is empty.
func LoadConfig(filePath string) (*Config, error) {
	var data []byte

	if filePath == "" {
		data = defaultConfigJSON
	} else {
		var err error
		data, err = os.ReadFile(filePath)
		if err != nil {
			return nil, fmt.Errorf("failed to read config file %s: %w", filePath, err)
		}
	}

	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("failed to parse config JSON: %w", err)
	}

	return &cfg, nil
}
