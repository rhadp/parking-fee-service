package store

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// Test data shared across store tests.
func testOperators() []model.Operator {
	return []model.Operator{
		{
			ID:     "op-1",
			Name:   "Operator One",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/repo/op1:v1",
				ChecksumSHA256: "sha256:aaa111",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op-2",
			Name:   "Operator Two",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/repo/op2:v1",
				ChecksumSHA256: "sha256:bbb222",
				Version:        "2.0.0",
			},
		},
		{
			ID:     "op-3",
			Name:   "Operator Three",
			ZoneID: "z2",
			Rate:   model.Rate{Type: "per-hour", Amount: 3.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/repo/op3:v1",
				ChecksumSHA256: "sha256:ccc333",
				Version:        "3.0.0",
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
				{Lat: 48.14, Lon: 11.55},
				{Lat: 48.14, Lon: 11.56},
				{Lat: 48.13, Lon: 11.56},
				{Lat: 48.13, Lon: 11.55},
			},
		},
		{
			ID:   "z2",
			Name: "Zone Two",
			Polygon: []model.Coordinate{
				{Lat: 49.00, Lon: 12.00},
				{Lat: 49.00, Lon: 12.01},
				{Lat: 48.99, Lon: 12.01},
				{Lat: 48.99, Lon: 12.00},
			},
		},
	}
}

// TS-05-4: When multiple operators serve matching zones, all are returned.
func TestMultipleOperatorsReturned(t *testing.T) {
	s := NewStore(testZones(), testOperators())
	result := s.GetOperatorsByZoneIDs([]string{"z1"})
	if len(result) != 2 {
		t.Errorf("expected 2 operators for zone z1, got %d", len(result))
	}

	// Verify both operators are present.
	ids := make(map[string]bool)
	for _, op := range result {
		ids[op.ID] = true
	}
	if !ids["op-1"] {
		t.Error("expected op-1 in results")
	}
	if !ids["op-2"] {
		t.Error("expected op-2 in results")
	}
}

// TS-05-P3: Property test — for any set of zone IDs, GetOperatorsByZoneIDs
// returns all and only operators whose zone_id is in the set.
func TestPropertyOperatorZoneAssociation(t *testing.T) {
	operators := testOperators()
	s := NewStore(testZones(), operators)

	// Test various subsets of zone IDs.
	testCases := []struct {
		name    string
		zoneIDs []string
	}{
		{"empty set", []string{}},
		{"z1 only", []string{"z1"}},
		{"z2 only", []string{"z2"}},
		{"both zones", []string{"z1", "z2"}},
		{"nonexistent zone", []string{"z999"}},
		{"mixed existing and nonexistent", []string{"z1", "z999"}},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			result := s.GetOperatorsByZoneIDs(tc.zoneIDs)

			zoneSet := make(map[string]bool)
			for _, zid := range tc.zoneIDs {
				zoneSet[zid] = true
			}

			// Every returned operator must have zone_id in the input set.
			for _, op := range result {
				if !zoneSet[op.ZoneID] {
					t.Errorf("returned operator %s has zone_id %s not in input set %v",
						op.ID, op.ZoneID, tc.zoneIDs)
				}
			}

			// Every operator with zone_id in the set must be returned.
			resultIDs := make(map[string]bool)
			for _, op := range result {
				resultIDs[op.ID] = true
			}
			for _, op := range operators {
				if zoneSet[op.ZoneID] && !resultIDs[op.ID] {
					t.Errorf("operator %s (zone_id %s) should be in results but was not",
						op.ID, op.ZoneID)
				}
			}
		})
	}
}

// TS-05-P5: Property test — for any valid operator ID, GetOperator returns an
// operator with non-empty image_ref, checksum_sha256, and version.
func TestPropertyAdapterCompleteness(t *testing.T) {
	operators := testOperators()
	s := NewStore(testZones(), operators)

	for _, op := range operators {
		t.Run(op.ID, func(t *testing.T) {
			result, found := s.GetOperator(op.ID)
			if !found {
				t.Fatalf("expected operator %s to be found", op.ID)
			}
			if result.Adapter.ImageRef == "" {
				t.Error("expected non-empty image_ref")
			}
			if result.Adapter.ChecksumSHA256 == "" {
				t.Error("expected non-empty checksum_sha256")
			}
			if result.Adapter.Version == "" {
				t.Error("expected non-empty version")
			}
		})
	}
}
