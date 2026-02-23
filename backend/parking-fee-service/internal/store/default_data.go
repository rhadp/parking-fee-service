package store

import (
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/model"
)

// defaultOperators returns the embedded default operator dataset.
// Includes two realistic demo operators in Munich, Germany.
func defaultOperators() []model.Operator {
	return []model.Operator{
		{
			ID:   "op-munich-01",
			Name: "Munich City Parking",
			Zone: model.Zone{
				ID:   "zone-munich-center",
				Name: "Munich City Center",
				Polygon: []model.Point{
					{Lat: 48.1400, Lon: 11.5600},
					{Lat: 48.1400, Lon: 11.5900},
					{Lat: 48.1300, Lon: 11.5900},
					{Lat: 48.1300, Lon: 11.5600},
				},
			},
			Rate: model.Rate{
				AmountPerHour: 2.50,
				Currency:      "EUR",
			},
			Adapter: model.Adapter{
				ImageRef:       "us-docker.pkg.dev/rhadp-demo/adapters/munich-parking:v1.0.0",
				ChecksumSHA256: "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
				Version:        "1.0.0",
			},
		},
		{
			ID:   "op-munich-02",
			Name: "Airport Parking Munich",
			Zone: model.Zone{
				ID:   "zone-munich-airport",
				Name: "Munich Airport",
				Polygon: []model.Point{
					{Lat: 48.3570, Lon: 11.7700},
					{Lat: 48.3570, Lon: 11.8100},
					{Lat: 48.3480, Lon: 11.8100},
					{Lat: 48.3480, Lon: 11.7700},
				},
			},
			Rate: model.Rate{
				AmountPerHour: 4.00,
				Currency:      "EUR",
			},
			Adapter: model.Adapter{
				ImageRef:       "us-docker.pkg.dev/rhadp-demo/adapters/airport-parking:v1.0.0",
				ChecksumSHA256: "sha256:b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3",
				Version:        "1.0.0",
			},
		},
	}
}
