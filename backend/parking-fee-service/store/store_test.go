package store_test

import (
	"math/rand"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

func defaultZones() []model.Zone {
	return []model.Zone{
		{
			ID:   "z1",
			Name: "Zone One",
			Polygon: []model.Coordinate{
				{Lat: 48.14, Lon: 11.555},
				{Lat: 48.14, Lon: 11.565},
				{Lat: 48.135, Lon: 11.565},
				{Lat: 48.135, Lon: 11.555},
			},
		},
		{
			ID:   "z2",
			Name: "Zone Two",
			Polygon: []model.Coordinate{
				{Lat: 48.138, Lon: 11.573},
				{Lat: 48.138, Lon: 11.579},
				{Lat: 48.135, Lon: 11.579},
				{Lat: 48.135, Lon: 11.573},
			},
		},
	}
}

func defaultOperators() []model.Operator {
	return []model.Operator{
		{
			ID:     "op1",
			Name:   "Operator One",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry.example.com/op1:v1.0.0",
				ChecksumSHA256: "sha256:op1hash",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op2",
			Name:   "Operator Two",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry.example.com/op2:v1.0.0",
				ChecksumSHA256: "sha256:op2hash",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op3",
			Name:   "Operator Three",
			ZoneID: "z2",
			Rate:   model.Rate{Type: "per-hour", Amount: 3.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry.example.com/op3:v1.0.0",
				ChecksumSHA256: "sha256:op3hash",
				Version:        "1.0.0",
			},
		},
	}
}

// TS-05-4: When multiple operators serve matching zones, all are returned.
func TestMultipleOperatorsReturned(t *testing.T) {
	s := store.NewStore(defaultZones(), defaultOperators())
	result := s.GetOperatorsByZoneIDs([]string{"z1"})
	if len(result) != 2 {
		t.Errorf("GetOperatorsByZoneIDs([z1]) returned %d operators, want 2", len(result))
	}
}

// TS-05-P3: Property test for operator-zone association.
// For any subset of zone IDs, GetOperatorsByZoneIDs returns all and only
// operators whose zone_id is in the set.
func TestPropertyOperatorZoneAssociation(t *testing.T) {
	operators := defaultOperators()
	s := store.NewStore(defaultZones(), operators)

	allZoneIDs := []string{"z1", "z2"}
	rng := rand.New(rand.NewSource(42))

	for i := 0; i < 50; i++ {
		// Generate random subset of zone IDs
		var subset []string
		for _, zid := range allZoneIDs {
			if rng.Intn(2) == 0 {
				subset = append(subset, zid)
			}
		}

		result := s.GetOperatorsByZoneIDs(subset)
		zoneIDSet := make(map[string]bool)
		for _, zid := range subset {
			zoneIDSet[zid] = true
		}

		// Every returned operator must have zone_id in the subset
		for _, op := range result {
			if !zoneIDSet[op.ZoneID] {
				t.Errorf("iteration %d: returned operator %q with zone_id %q not in subset %v",
					i, op.ID, op.ZoneID, subset)
			}
		}

		// Every operator with zone_id in the subset must be returned
		resultIDs := make(map[string]bool)
		for _, op := range result {
			resultIDs[op.ID] = true
		}
		for _, op := range operators {
			if zoneIDSet[op.ZoneID] && !resultIDs[op.ID] {
				t.Errorf("iteration %d: operator %q with zone_id %q not in result, subset %v",
					i, op.ID, op.ZoneID, subset)
			}
		}
	}
}

// TS-05-P5: Property test for adapter metadata completeness.
// For all operator IDs, GetOperator returns an operator with non-empty adapter fields.
func TestPropertyAdapterCompleteness(t *testing.T) {
	operators := defaultOperators()
	s := store.NewStore(defaultZones(), operators)

	for _, op := range operators {
		got, found := s.GetOperator(op.ID)
		if !found {
			t.Errorf("GetOperator(%q) not found", op.ID)
			continue
		}
		if got.Adapter.ImageRef == "" {
			t.Errorf("operator %q: Adapter.ImageRef is empty", op.ID)
		}
		if got.Adapter.ChecksumSHA256 == "" {
			t.Errorf("operator %q: Adapter.ChecksumSHA256 is empty", op.ID)
		}
		if got.Adapter.Version == "" {
			t.Errorf("operator %q: Adapter.Version is empty", op.ID)
		}
	}
}
