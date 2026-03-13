package store_test

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// testZones and testOperators provide a reusable fixture.
var testZones = []model.Zone{
	{ID: "z1", Name: "Zone 1", Polygon: []model.Coordinate{{Lat: 1, Lon: 1}, {Lat: 1, Lon: 2}, {Lat: 2, Lon: 1}}},
	{ID: "z2", Name: "Zone 2", Polygon: []model.Coordinate{{Lat: 3, Lon: 3}, {Lat: 3, Lon: 4}, {Lat: 4, Lon: 3}}},
}

var testOperators = []model.Operator{
	{
		ID: "op1", Name: "Operator 1", ZoneID: "z1",
		Rate:    model.Rate{Type: "per-hour", Amount: 2.0, Currency: "EUR"},
		Adapter: model.AdapterMeta{ImageRef: "reg/op1:v1", ChecksumSHA256: "sha256:aaa", Version: "1.0.0"},
	},
	{
		ID: "op2", Name: "Operator 2", ZoneID: "z1",
		Rate:    model.Rate{Type: "flat-fee", Amount: 5.0, Currency: "EUR"},
		Adapter: model.AdapterMeta{ImageRef: "reg/op2:v1", ChecksumSHA256: "sha256:bbb", Version: "1.0.0"},
	},
	{
		ID: "op3", Name: "Operator 3", ZoneID: "z2",
		Rate:    model.Rate{Type: "per-hour", Amount: 3.0, Currency: "EUR"},
		Adapter: model.AdapterMeta{ImageRef: "reg/op3:v1", ChecksumSHA256: "sha256:ccc", Version: "1.0.0"},
	},
}

// TS-05-4: When multiple operators serve matching zones, all are returned.
func TestMultipleOperatorsReturned(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	result := s.GetOperatorsByZoneIDs([]string{"z1"})
	if len(result) != 2 {
		t.Errorf("GetOperatorsByZoneIDs([z1]) returned %d operators, want 2", len(result))
	}
}

// GetZone: returns the correct zone for a known ID.
func TestGetZoneKnown(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	z, ok := s.GetZone("z1")
	if !ok {
		t.Fatal("GetZone(z1) returned ok=false, want true")
	}
	if z.ID != "z1" {
		t.Errorf("GetZone(z1).ID = %q, want z1", z.ID)
	}
}

// GetZone: returns false for an unknown ID.
func TestGetZoneUnknown(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	_, ok := s.GetZone("nonexistent")
	if ok {
		t.Error("GetZone(nonexistent) returned ok=true, want false")
	}
}

// GetOperator: returns the correct operator for a known ID.
func TestGetOperatorKnown(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	op, ok := s.GetOperator("op1")
	if !ok {
		t.Fatal("GetOperator(op1) returned ok=false, want true")
	}
	if op.ID != "op1" {
		t.Errorf("GetOperator(op1).ID = %q, want op1", op.ID)
	}
}

// GetOperator: returns false for an unknown ID.
func TestGetOperatorUnknown(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	_, ok := s.GetOperator("nonexistent")
	if ok {
		t.Error("GetOperator(nonexistent) returned ok=true, want false")
	}
}

// GetOperatorsByZoneIDs: returns operators for a single zone.
func TestGetOperatorsByZoneIDsSingle(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	result := s.GetOperatorsByZoneIDs([]string{"z2"})
	if len(result) != 1 {
		t.Errorf("GetOperatorsByZoneIDs([z2]) = %d operators, want 1", len(result))
	}
	if len(result) > 0 && result[0].ID != "op3" {
		t.Errorf("GetOperatorsByZoneIDs([z2])[0].ID = %q, want op3", result[0].ID)
	}
}

// GetOperatorsByZoneIDs: returns operators for multiple zones.
func TestGetOperatorsByZoneIDsMultiple(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	result := s.GetOperatorsByZoneIDs([]string{"z1", "z2"})
	if len(result) != 3 {
		t.Errorf("GetOperatorsByZoneIDs([z1,z2]) = %d operators, want 3", len(result))
	}
}

// GetOperatorsByZoneIDs: returns empty slice for unknown zone.
func TestGetOperatorsByZoneIDsUnknown(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	result := s.GetOperatorsByZoneIDs([]string{"unknown-zone"})
	if len(result) != 0 {
		t.Errorf("GetOperatorsByZoneIDs([unknown-zone]) = %d operators, want 0", len(result))
	}
}

// TS-05-P3: Operator-zone association property.
// For any subset of zone IDs, GetOperatorsByZoneIDs returns all and only operators
// whose ZoneID is in the subset.
func TestPropertyOperatorZoneAssociation(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	allZoneIDs := []string{"z1", "z2"}
	subsets := [][]string{
		{},
		{"z1"},
		{"z2"},
		{"z1", "z2"},
	}

	for _, zoneIDs := range subsets {
		zoneSet := make(map[string]bool)
		for _, id := range zoneIDs {
			zoneSet[id] = true
		}

		result := s.GetOperatorsByZoneIDs(zoneIDs)

		// Every returned operator must have ZoneID in the requested set.
		for _, op := range result {
			if !zoneSet[op.ZoneID] {
				t.Errorf("GetOperatorsByZoneIDs(%v) returned operator %q with ZoneID %q not in set",
					zoneIDs, op.ID, op.ZoneID)
			}
		}

		// Every operator whose ZoneID is in the set must appear in the result.
		resultIDs := make(map[string]bool)
		for _, op := range result {
			resultIDs[op.ID] = true
		}
		for _, op := range testOperators {
			if zoneSet[op.ZoneID] && !resultIDs[op.ID] {
				t.Errorf("GetOperatorsByZoneIDs(%v) missing operator %q (ZoneID=%q)",
					zoneIDs, op.ID, op.ZoneID)
			}
		}

		_ = allZoneIDs // suppress "declared and not used"
	}
}

// TS-05-P5: Adapter metadata completeness.
// For every valid operator ID, GetOperator returns an operator with non-empty adapter fields.
func TestPropertyAdapterCompleteness(t *testing.T) {
	s := store.NewStore(testZones, testOperators)
	for _, want := range testOperators {
		op, found := s.GetOperator(want.ID)
		if !found {
			t.Errorf("GetOperator(%q) returned found=false", want.ID)
			continue
		}
		if op.Adapter.ImageRef == "" {
			t.Errorf("operator %q: Adapter.ImageRef is empty", want.ID)
		}
		if op.Adapter.ChecksumSHA256 == "" {
			t.Errorf("operator %q: Adapter.ChecksumSHA256 is empty", want.ID)
		}
		if op.Adapter.Version == "" {
			t.Errorf("operator %q: Adapter.Version is empty", want.ID)
		}
	}
}
