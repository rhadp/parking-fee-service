package store

import (
	"testing"

	"parking-fee-service/backend/parking-fee-service/model"
)

func testOperators() []model.Operator {
	return []model.Operator{
		{
			ID:     "op1",
			Name:   "Operator One",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/op1:v1",
				ChecksumSHA256: "sha256:abc123",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op2",
			Name:   "Operator Two",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/op2:v1",
				ChecksumSHA256: "sha256:def456",
				Version:        "2.0.0",
			},
		},
		{
			ID:     "op3",
			Name:   "Operator Three",
			ZoneID: "z2",
			Rate:   model.Rate{Type: "per-hour", Amount: 3.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/op3:v1",
				ChecksumSHA256: "sha256:ghi789",
				Version:        "1.5.0",
			},
		},
	}
}

func testZones() []model.Zone {
	return []model.Zone{
		{
			ID:   "z1",
			Name: "Zone One",
			Polygon: []model.Coordinate{
				{Lat: 10, Lon: 20}, {Lat: 10, Lon: 21},
				{Lat: 11, Lon: 21}, {Lat: 11, Lon: 20},
			},
		},
		{
			ID:   "z2",
			Name: "Zone Two",
			Polygon: []model.Coordinate{
				{Lat: 20, Lon: 30}, {Lat: 20, Lon: 31},
				{Lat: 21, Lon: 31}, {Lat: 21, Lon: 30},
			},
		},
	}
}

// TS-05-4: Multiple operators with same zone_id are all returned.
func TestMultipleOperatorsReturned(t *testing.T) {
	s := NewStore(testZones(), testOperators())
	result := s.GetOperatorsByZoneIDs([]string{"z1"})
	if len(result) != 2 {
		t.Errorf("expected 2 operators for zone z1, got %d", len(result))
	}
}

// TS-05-P3: Property — GetOperatorsByZoneIDs returns all and only matching operators.
func TestPropertyOperatorZoneAssociation(t *testing.T) {
	ops := testOperators()
	s := NewStore(testZones(), ops)

	// Test various subsets of zone IDs.
	testCases := []struct {
		zoneIDs  []string
		expected int
	}{
		{[]string{"z1"}, 2},
		{[]string{"z2"}, 1},
		{[]string{"z1", "z2"}, 3},
		{[]string{"z3"}, 0},       // nonexistent zone
		{[]string{}, 0},           // empty input
	}

	for _, tc := range testCases {
		result := s.GetOperatorsByZoneIDs(tc.zoneIDs)
		if len(result) != tc.expected {
			t.Errorf("GetOperatorsByZoneIDs(%v): expected %d operators, got %d",
				tc.zoneIDs, tc.expected, len(result))
		}
		// Verify all returned operators have zone_id in the input set.
		zoneSet := make(map[string]bool)
		for _, z := range tc.zoneIDs {
			zoneSet[z] = true
		}
		for _, op := range result {
			if !zoneSet[op.ZoneID] {
				t.Errorf("operator %q has zone_id %q which is not in input %v",
					op.ID, op.ZoneID, tc.zoneIDs)
			}
		}
	}
}

// TS-05-P5: Property — all operators have non-empty adapter metadata.
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
			t.Errorf("operator %q: image_ref is empty", op.ID)
		}
		if got.Adapter.ChecksumSHA256 == "" {
			t.Errorf("operator %q: checksum_sha256 is empty", op.ID)
		}
		if got.Adapter.Version == "" {
			t.Errorf("operator %q: version is empty", op.ID)
		}
	}
}

// Additional: GetOperator returns false for unknown IDs.
func TestGetOperatorNotFound(t *testing.T) {
	s := NewStore(testZones(), testOperators())
	_, found := s.GetOperator("nonexistent")
	if found {
		t.Error("expected GetOperator to return false for nonexistent ID")
	}
}
