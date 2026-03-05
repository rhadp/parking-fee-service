package main

import (
	_ "embed"
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
	// Stub: not yet implemented
	return nil, nil
}
