package store

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// testZones returns a set of zones for testing.
func testZones() []model.Zone {
	return []model.Zone{
		{
			ID:   "z1",
			Name: "Zone 1",
			Polygon: []model.Coordinate{
				{Lat: 0, Lon: 0}, {Lat: 0, Lon: 1}, {Lat: 1, Lon: 0},
			},
		},
		{
			ID:   "z2",
			Name: "Zone 2",
			Polygon: []model.Coordinate{
				{Lat: 2, Lon: 2}, {Lat: 2, Lon: 3}, {Lat: 3, Lon: 2},
			},
		},
	}
}

// testOperators returns a set of operators for testing. Two operators share
// zone "z1" to test multi-operator zone lookup.
func testOperators() []model.Operator {
	return []model.Operator{
		{
			ID:     "op1",
			Name:   "Operator 1",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/op1:v1",
				ChecksumSHA256: "sha256:abc",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op2",
			Name:   "Operator 2",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/op2:v1",
				ChecksumSHA256: "sha256:def",
				Version:        "2.0.0",
			},
		},
		{
			ID:     "op3",
			Name:   "Operator 3",
			ZoneID: "z2",
			Rate:   model.Rate{Type: "per-hour", Amount: 3.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/op3:v1",
				ChecksumSHA256: "sha256:ghi",
				Version:        "1.0.0",
			},
		},
	}
}

// ---------------------------------------------------------------------------
// TS-05-4: Multiple Operators Returned
// ---------------------------------------------------------------------------

// TestMultipleOperatorsReturned verifies that when multiple operators serve
// the same zone, all are returned by GetOperatorsByZoneIDs.
func TestMultipleOperatorsReturned(t *testing.T) {
	s := NewStore(testZones(), testOperators())
	result := s.GetOperatorsByZoneIDs([]string{"z1"})
	if len(result) != 2 {
		t.Errorf("GetOperatorsByZoneIDs([\"z1\"]) returned %d operators, want 2",
			len(result))
	}
}

// ---------------------------------------------------------------------------
// TS-05-P3: Property – Operator-Zone Association
// ---------------------------------------------------------------------------

// TestPropertyOperatorZoneAssociation verifies that for any subset of zone
// IDs, GetOperatorsByZoneIDs returns all and only operators whose zone_id
// is in the set.
func TestPropertyOperatorZoneAssociation(t *testing.T) {
	ops := testOperators()
	s := NewStore(testZones(), ops)

	subsets := [][]string{
		{"z1"},
		{"z2"},
		{"z1", "z2"},
		{},
		{"nonexistent"},
	}

	for _, zoneIDs := range subsets {
		result := s.GetOperatorsByZoneIDs(zoneIDs)

		// Build zone ID set for lookup.
		zoneSet := make(map[string]bool)
		for _, id := range zoneIDs {
			zoneSet[id] = true
		}

		// Every returned operator must have zone_id in the set.
		for _, op := range result {
			if !zoneSet[op.ZoneID] {
				t.Errorf("GetOperatorsByZoneIDs(%v) returned operator %q with zone_id %q not in set",
					zoneIDs, op.ID, op.ZoneID)
			}
		}

		// Every operator with zone_id in the set must be returned.
		resultSet := make(map[string]bool)
		for _, op := range result {
			resultSet[op.ID] = true
		}
		for _, op := range ops {
			if zoneSet[op.ZoneID] && !resultSet[op.ID] {
				t.Errorf("GetOperatorsByZoneIDs(%v) missing operator %q with zone_id %q",
					zoneIDs, op.ID, op.ZoneID)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// TS-05-P5: Property – Adapter Metadata Completeness
// ---------------------------------------------------------------------------

// TestPropertyAdapterCompleteness verifies that for every operator stored,
// GetOperator returns an operator with non-empty adapter metadata fields.
func TestPropertyAdapterCompleteness(t *testing.T) {
	ops := testOperators()
	s := NewStore(testZones(), ops)

	for _, op := range ops {
		got, found := s.GetOperator(op.ID)
		if !found {
			t.Errorf("GetOperator(%q) not found", op.ID)
			continue
		}
		if got.Adapter.ImageRef == "" {
			t.Errorf("operator %q has empty image_ref", op.ID)
		}
		if got.Adapter.ChecksumSHA256 == "" {
			t.Errorf("operator %q has empty checksum_sha256", op.ID)
		}
		if got.Adapter.Version == "" {
			t.Errorf("operator %q has empty version", op.ID)
		}
	}
}
