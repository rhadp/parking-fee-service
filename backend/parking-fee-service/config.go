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

// Settings holds service-level configuration.
type Settings struct {
	Port                     int     `json:"port"`
	ProximityThresholdMeters float64 `json:"proximity_threshold_meters"`
}

// OperatorConfig extends Operator with adapter metadata.
type OperatorConfig struct {
	Operator
	Adapter AdapterMetadata `json:"adapter"`
}

// LoadConfig loads configuration from a file path. If filePath is empty,
// the embedded default config is used.
func LoadConfig(filePath string) (*Config, error) {
	var data []byte
	var err error

	if filePath == "" {
		data = defaultConfigJSON
	} else {
		data, err = os.ReadFile(filePath)
		if err != nil {
			return nil, fmt.Errorf("reading config file %s: %w", filePath, err)
		}
	}

	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("parsing config JSON: %w", err)
	}

	return &cfg, nil
}
