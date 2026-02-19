package zones

import (
	"log/slog"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
)

// SeedZones contains the hardcoded demo parking zones with realistic Munich
// coordinates. Each zone has a geofence polygon, rate configuration, and
// adapter metadata.
var SeedZones = []Zone{
	{
		ZoneID:       "zone-marienplatz",
		Name:         "Marienplatz Central",
		OperatorName: "München Parking GmbH",
		Polygon: []geo.LatLon{
			{Latitude: 48.1380, Longitude: 11.5730},
			{Latitude: 48.1380, Longitude: 11.5780},
			{Latitude: 48.1355, Longitude: 11.5780},
			{Latitude: 48.1355, Longitude: 11.5730},
		},
		AdapterImageRef: "localhost/parking-operator-adaptor:latest",
		AdapterChecksum: "sha256:demo-checksum-marienplatz",
		RateType:        "per_minute",
		RateAmount:      0.05,
		Currency:        "EUR",
	},
	{
		ZoneID:       "zone-olympiapark",
		Name:         "Olympiapark",
		OperatorName: "Olympiapark Parking Services",
		Polygon: []geo.LatLon{
			{Latitude: 48.1770, Longitude: 11.5490},
			{Latitude: 48.1770, Longitude: 11.5580},
			{Latitude: 48.1720, Longitude: 11.5580},
			{Latitude: 48.1720, Longitude: 11.5490},
		},
		AdapterImageRef: "localhost/parking-operator-adaptor:latest",
		AdapterChecksum: "sha256:demo-checksum-olympiapark",
		RateType:        "per_minute",
		RateAmount:      0.04,
		Currency:        "EUR",
	},
	{
		ZoneID:       "zone-sendlinger-tor",
		Name:         "Sendlinger Tor",
		OperatorName: "City Parking Munich",
		Polygon: []geo.LatLon{
			{Latitude: 48.1345, Longitude: 11.5650},
			{Latitude: 48.1345, Longitude: 11.5700},
			{Latitude: 48.1320, Longitude: 11.5700},
			{Latitude: 48.1320, Longitude: 11.5650},
		},
		AdapterImageRef: "localhost/parking-operator-adaptor:latest",
		AdapterChecksum: "sha256:demo-checksum-sendlinger-tor",
		RateType:        "flat",
		RateAmount:      2.50,
		Currency:        "EUR",
	},
}

// LoadSeedData creates a new Store and populates it with the hardcoded demo
// zones. Zones with malformed polygons (fewer than 3 points) are skipped
// with a warning log.
func LoadSeedData() *Store {
	store := NewStore()

	for i := range SeedZones {
		z := &SeedZones[i]

		if len(z.Polygon) < 3 {
			slog.Warn("skipping zone with malformed polygon",
				"zone_id", z.ZoneID,
				"polygon_points", len(z.Polygon),
			)
			continue
		}

		store.Add(z)
	}

	slog.Info("loaded seed zones", "count", len(store.zones))
	return store
}
