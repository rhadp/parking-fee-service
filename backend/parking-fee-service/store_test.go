package main

import (
	"testing"
)

func newTestStore() *Store {
	cfg := &Config{
		Settings: Settings{
			Port:                     8080,
			ProximityThresholdMeters: 500,
		},
		Zones: []Zone{
			{
				ID:   "zone-muc-central",
				Name: "Munich Central Station Area",
				Polygon: []LatLon{
					{Lat: 48.1420, Lon: 11.5550},
					{Lat: 48.1420, Lon: 11.5700},
					{Lat: 48.1370, Lon: 11.5700},
					{Lat: 48.1370, Lon: 11.5550},
				},
			},
			{
				ID:   "zone-muc-airport",
				Name: "Munich Airport Area",
				Polygon: []LatLon{
					{Lat: 48.3570, Lon: 11.7750},
					{Lat: 48.3570, Lon: 11.7950},
					{Lat: 48.3480, Lon: 11.7950},
					{Lat: 48.3480, Lon: 11.7750},
				},
			},
		},
		Operators: []OperatorConfig{
			{
				Operator: Operator{
					ID:           "muc-central",
					Name:         "Munich Central Parking",
					ZoneID:       "zone-muc-central",
					RateType:     RatePerHour,
					RateAmount:   2.50,
					RateCurrency: "EUR",
				},
				Adapter: AdapterMetadata{
					ImageRef:       "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-central:v1.0.0",
					ChecksumSHA256: "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
					Version:        "v1.0.0",
				},
			},
			{
				Operator: Operator{
					ID:           "muc-airport",
					Name:         "Munich Airport Parking",
					ZoneID:       "zone-muc-airport",
					RateType:     RateFlatFee,
					RateAmount:   5.00,
					RateCurrency: "EUR",
				},
				Adapter: AdapterMetadata{
					ImageRef:       "europe-west1-docker.pkg.dev/rhadp-parking-demo/adapters/muc-airport:v1.0.0",
					ChecksumSHA256: "sha256:f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5d4c3b2a1f6e5",
					Version:        "v1.0.0",
				},
			},
		},
	}
	return NewStore(cfg)
}

func TestFindOperatorsByLocation(t *testing.T) {
	store := newTestStore()
	if store == nil {
		t.Fatal("NewStore returned nil")
	}

	// Point inside muc-central zone
	ops := store.FindOperatorsByLocation(48.1395, 11.5625)
	if len(ops) == 0 {
		t.Fatal("expected at least one operator for location inside muc-central zone")
	}

	found := false
	for _, op := range ops {
		if op.ID == "muc-central" {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected muc-central operator in results")
	}

	// Point far away -- should return empty
	ops = store.FindOperatorsByLocation(52.5200, 13.4050)
	if len(ops) != 0 {
		t.Errorf("expected no operators for remote location, got %d", len(ops))
	}
}

func TestGetAdapterMetadata(t *testing.T) {
	store := newTestStore()
	if store == nil {
		t.Fatal("NewStore returned nil")
	}

	meta, ok := store.GetAdapterMetadata("muc-central")
	if !ok {
		t.Fatal("expected to find adapter metadata for muc-central")
	}
	if meta == nil {
		t.Fatal("adapter metadata is nil")
	}
	if meta.ImageRef == "" {
		t.Error("expected non-empty image_ref")
	}
	if meta.Version != "v1.0.0" {
		t.Errorf("expected version v1.0.0, got %s", meta.Version)
	}
}

func TestGetAdapterMetadataUnknown(t *testing.T) {
	store := newTestStore()
	if store == nil {
		t.Fatal("NewStore returned nil")
	}

	_, ok := store.GetAdapterMetadata("nonexistent")
	if ok {
		t.Error("expected not found for unknown operator ID")
	}
}
