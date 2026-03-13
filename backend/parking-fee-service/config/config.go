// Package config provides configuration loading and default data for the parking-fee-service.
package config

import (
	"fmt"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// LoadConfig reads the JSON config file at path.
// If the file does not exist it returns DefaultConfig and no error.
// If the file contains invalid JSON it returns a non-nil error.
// STUB: not yet implemented — returns an error for all paths.
func LoadConfig(path string) (*model.Config, error) {
	return nil, fmt.Errorf("LoadConfig: not implemented")
}

// DefaultConfig returns the built-in Munich demo configuration.
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
				Name:   "Parkhaus München GmbH",
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
				Name:   "CityPark München",
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
