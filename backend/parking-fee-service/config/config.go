// Package config handles loading and defaulting service configuration.
package config

import (
	"encoding/json"
	"errors"
	"log/slog"
	"os"

	"parking-fee-service/backend/parking-fee-service/model"
)

// LoadConfig reads configuration from the specified file path.
// If the file does not exist, it returns DefaultConfig and logs a warning.
// If the file contains invalid JSON, it returns an error.
func LoadConfig(path string) (*model.Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			slog.Warn("config file not found, using defaults", "path", path)
			return DefaultConfig(), nil
		}
		// Also treat other file errors (permission, etc.) as missing → defaults
		slog.Warn("config file not readable, using defaults", "path", path, "error", err)
		return DefaultConfig(), nil
	}

	var cfg model.Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}

// DefaultConfig returns built-in Munich demo configuration.
func DefaultConfig() *model.Config {
	return &model.Config{
		Port:               8080,
		ProximityThreshold: 500.0,
		Zones: []model.Zone{
			{
				ID:   "munich-central",
				Name: "Munich Central Station Area",
				Polygon: []model.Coordinate{
					{Lat: 48.1400, Lon: 11.5550},
					{Lat: 48.1400, Lon: 11.5650},
					{Lat: 48.1350, Lon: 11.5650},
					{Lat: 48.1350, Lon: 11.5550},
				},
			},
			{
				ID:   "munich-marienplatz",
				Name: "Marienplatz Area",
				Polygon: []model.Coordinate{
					{Lat: 48.1380, Lon: 11.5730},
					{Lat: 48.1380, Lon: 11.5790},
					{Lat: 48.1350, Lon: 11.5790},
					{Lat: 48.1350, Lon: 11.5730},
				},
			},
		},
		Operators: []model.Operator{
			{
				ID:     "parkhaus-munich",
				Name:   "Parkhaus Muenchen GmbH",
				ZoneID: "munich-central",
				Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
				Adapter: model.AdapterMeta{
					ImageRef:       "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
					ChecksumSHA256: "sha256:abc123def456",
					Version:        "1.0.0",
				},
			},
			{
				ID:     "city-park-munich",
				Name:   "CityPark Muenchen",
				ZoneID: "munich-marienplatz",
				Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
				Adapter: model.AdapterMeta{
					ImageRef:       "us-docker.pkg.dev/sdv-demo/adapters/citypark-munich:v1.0.0",
					ChecksumSHA256: "sha256:789ghi012jkl",
					Version:        "1.0.0",
				},
			},
		},
	}
}
