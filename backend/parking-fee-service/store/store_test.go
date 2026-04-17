package store_test

import (
	"testing"

	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/model"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/store"
)

// testZones returns a set of zones for testing.
func testZones() []model.Zone {
	return []model.Zone{
		{
			ID:   "zone-1",
			Name: "Zone One",
			Polygon: []model.Coordinate{
				{Lat: 48.140, Lon: 11.555},
				{Lat: 48.140, Lon: 11.565},
				{Lat: 48.135, Lon: 11.565},
				{Lat: 48.135, Lon: 11.555},
			},
		},
		{
			ID:   "zone-2",
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

// testOperators returns a set of operators for testing.
func testOperators() []model.Operator {
	return []model.Operator{
		{
			ID:     "op-1",
			Name:   "Operator One",
			ZoneID: "zone-1",
			Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/op-1:v1.0.0",
				ChecksumSHA256: "sha256:aabbcc",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op-2",
			Name:   "Operator Two",
			ZoneID: "zone-1",
			Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/op-2:v1.0.0",
				ChecksumSHA256: "sha256:ddeeff",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op-3",
			Name:   "Operator Three",
			ZoneID: "zone-2",
			Rate:   model.Rate{Type: "per-hour", Amount: 3.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "registry/op-3:v1.0.0",
				ChecksumSHA256: "sha256:112233",
				Version:        "1.0.0",
			},
		},
	}
}

// TS-05-4: When multiple operators are assigned to the same zone, all are returned.
func TestMultipleOperatorsReturned(t *testing.T) {
	t.Helper()
	s := store.NewStore(testZones(), testOperators())

	result := s.GetOperatorsByZoneIDs([]string{"zone-1"})
	if len(result) != 2 {
		t.Errorf("GetOperatorsByZoneIDs([zone-1]): want 2 operators, got %d: %v",
			len(result), result)
	}

	// Verify both op-1 and op-2 are present.
	ids := make(map[string]bool)
	for _, op := range result {
		ids[op.ID] = true
	}
	for _, wantID := range []string{"op-1", "op-2"} {
		if !ids[wantID] {
			t.Errorf("GetOperatorsByZoneIDs([zone-1]): expected %q in results %v", wantID, result)
		}
	}
}

// GetOperator returns the correct operator when it exists.
func TestGetOperatorFound(t *testing.T) {
	t.Helper()
	s := store.NewStore(testZones(), testOperators())

	op, ok := s.GetOperator("op-1")
	if !ok {
		t.Fatal("GetOperator(op-1): want found=true, got false")
	}
	if op.ID != "op-1" {
		t.Errorf("GetOperator(op-1): want ID='op-1', got %q", op.ID)
	}
}

// GetOperator returns false for an unknown operator ID.
func TestGetOperatorNotFound(t *testing.T) {
	t.Helper()
	s := store.NewStore(testZones(), testOperators())

	_, ok := s.GetOperator("nonexistent")
	if ok {
		t.Error("GetOperator(nonexistent): want found=false, got true")
	}
}

// GetOperatorsByZoneIDs returns only operators in the requested zones.
func TestGetOperatorsByZoneIDsFiltered(t *testing.T) {
	t.Helper()
	s := store.NewStore(testZones(), testOperators())

	// Only zone-2 is requested; op-3 is the only zone-2 operator.
	result := s.GetOperatorsByZoneIDs([]string{"zone-2"})
	if len(result) != 1 {
		t.Fatalf("GetOperatorsByZoneIDs([zone-2]): want 1 operator, got %d: %v",
			len(result), result)
	}
	if result[0].ID != "op-3" {
		t.Errorf("GetOperatorsByZoneIDs([zone-2])[0].ID: want 'op-3', got %q", result[0].ID)
	}
}

// GetOperatorsByZoneIDs returns empty for unrecognised zone IDs.
func TestGetOperatorsByZoneIDsEmpty(t *testing.T) {
	t.Helper()
	s := store.NewStore(testZones(), testOperators())

	result := s.GetOperatorsByZoneIDs([]string{"no-such-zone"})
	if len(result) != 0 {
		t.Errorf("GetOperatorsByZoneIDs([no-such-zone]): want empty, got %v", result)
	}
}

// TS-05-P3: Property — GetOperatorsByZoneIDs returns all and only operators
// whose zone_id is in the requested set.
func TestPropertyOperatorZoneAssociation(t *testing.T) {
	t.Helper()
	operators := testOperators()
	s := store.NewStore(testZones(), operators)

	subsets := [][]string{
		{"zone-1"},
		{"zone-2"},
		{"zone-1", "zone-2"},
		{},
	}

	for _, zoneIDs := range subsets {
		zoneSet := make(map[string]bool, len(zoneIDs))
		for _, id := range zoneIDs {
			zoneSet[id] = true
		}

		result := s.GetOperatorsByZoneIDs(zoneIDs)

		// Every returned operator must have zone_id in the input set.
		for _, op := range result {
			if !zoneSet[op.ZoneID] {
				t.Errorf("GetOperatorsByZoneIDs(%v): returned operator %q with zone_id=%q not in set",
					zoneIDs, op.ID, op.ZoneID)
			}
		}

		// Every operator with zone_id in the set must be returned.
		resultIDs := make(map[string]bool, len(result))
		for _, op := range result {
			resultIDs[op.ID] = true
		}
		for _, op := range operators {
			if zoneSet[op.ZoneID] && !resultIDs[op.ID] {
				t.Errorf("GetOperatorsByZoneIDs(%v): operator %q (zone_id=%q) expected but missing",
					zoneIDs, op.ID, op.ZoneID)
			}
		}
	}
}

// TS-05-P5: Property — for any valid operator ID, GetOperator returns an
// operator with non-empty image_ref, checksum_sha256, and version.
func TestPropertyAdapterCompleteness(t *testing.T) {
	t.Helper()
	operators := testOperators()
	s := store.NewStore(testZones(), operators)

	for _, want := range operators {
		op, ok := s.GetOperator(want.ID)
		if !ok {
			t.Errorf("GetOperator(%q): want found=true, got false", want.ID)
			continue
		}
		if op.Adapter.ImageRef == "" {
			t.Errorf("GetOperator(%q).Adapter.ImageRef: want non-empty, got empty", want.ID)
		}
		if op.Adapter.ChecksumSHA256 == "" {
			t.Errorf("GetOperator(%q).Adapter.ChecksumSHA256: want non-empty, got empty", want.ID)
		}
		if op.Adapter.Version == "" {
			t.Errorf("GetOperator(%q).Adapter.Version: want non-empty, got empty", want.ID)
		}
	}
}
