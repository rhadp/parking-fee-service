package store_test

import (
	"testing"

	"parking-fee-service/backend/parking-fee-service/model"
	"parking-fee-service/backend/parking-fee-service/store"
)

func makeTestZones() []model.Zone {
	return []model.Zone{
		{
			ID:   "z1",
			Name: "Zone 1",
			Polygon: []model.Coordinate{
				{Lat: 1.0, Lon: 1.0},
				{Lat: 1.0, Lon: 2.0},
				{Lat: 2.0, Lon: 2.0},
				{Lat: 2.0, Lon: 1.0},
			},
		},
		{
			ID:   "z2",
			Name: "Zone 2",
			Polygon: []model.Coordinate{
				{Lat: 3.0, Lon: 3.0},
				{Lat: 3.0, Lon: 4.0},
				{Lat: 4.0, Lon: 4.0},
				{Lat: 4.0, Lon: 3.0},
			},
		},
	}
}

func makeTestOperators() []model.Operator {
	return []model.Operator{
		{
			ID:     "op1",
			Name:   "Operator 1",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "example.com/op1:v1",
				ChecksumSHA256: "sha256:aaa",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op2",
			Name:   "Operator 2",
			ZoneID: "z1",
			Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "example.com/op2:v1",
				ChecksumSHA256: "sha256:bbb",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op3",
			Name:   "Operator 3",
			ZoneID: "z2",
			Rate:   model.Rate{Type: "per-hour", Amount: 1.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "example.com/op3:v1",
				ChecksumSHA256: "sha256:ccc",
				Version:        "2.0.0",
			},
		},
	}
}

// TestMultipleOperatorsReturned verifies GetOperatorsByZoneIDs returns all
// operators for a zone when multiple operators share that zone (TS-05-4).
func TestMultipleOperatorsReturned(t *testing.T) {
	s := store.NewStore(makeTestZones(), makeTestOperators())
	ops := s.GetOperatorsByZoneIDs([]string{"z1"})
	if len(ops) != 2 {
		t.Errorf("want 2 operators for z1, got %d", len(ops))
	}
}

// TestGetOperator verifies a known operator can be retrieved by ID.
func TestGetOperator(t *testing.T) {
	s := store.NewStore(makeTestZones(), makeTestOperators())
	op, found := s.GetOperator("op1")
	if !found {
		t.Fatal("GetOperator: op1 not found")
	}
	if op.ZoneID != "z1" {
		t.Errorf("op1 ZoneID: want z1, got %q", op.ZoneID)
	}
}

// TestGetOperatorNotFound verifies that GetOperator returns false for an
// unknown ID.
func TestGetOperatorNotFound(t *testing.T) {
	s := store.NewStore(makeTestZones(), makeTestOperators())
	_, found := s.GetOperator("nonexistent")
	if found {
		t.Error("expected not found for nonexistent operator")
	}
}

// TestGetZone verifies a known zone can be retrieved.
func TestGetZone(t *testing.T) {
	s := store.NewStore(makeTestZones(), makeTestOperators())
	z, found := s.GetZone("z2")
	if !found {
		t.Fatal("GetZone: z2 not found")
	}
	if z.ID != "z2" {
		t.Errorf("zone ID: want z2, got %q", z.ID)
	}
}

// TestGetOperatorsByZoneIDsEmpty verifies that no operators are returned when
// the zone ID set is empty.
func TestGetOperatorsByZoneIDsEmpty(t *testing.T) {
	s := store.NewStore(makeTestZones(), makeTestOperators())
	ops := s.GetOperatorsByZoneIDs([]string{})
	if len(ops) != 0 {
		t.Errorf("want 0 operators for empty zone set, got %d", len(ops))
	}
}

// TestGetOperatorsByZoneIDsMultipleZones verifies operators from multiple
// zones are all returned.
func TestGetOperatorsByZoneIDsMultipleZones(t *testing.T) {
	s := store.NewStore(makeTestZones(), makeTestOperators())
	ops := s.GetOperatorsByZoneIDs([]string{"z1", "z2"})
	if len(ops) != 3 {
		t.Errorf("want 3 operators (z1+z2), got %d", len(ops))
	}
}

// TestPropertyOperatorZoneAssociation verifies Property 3: every returned
// operator has a zone_id in the input set, and every operator with a
// matching zone_id is returned (TS-05-P3).
func TestPropertyOperatorZoneAssociation(t *testing.T) {
	zones := makeTestZones()
	operators := makeTestOperators()
	s := store.NewStore(zones, operators)

	testSets := [][]string{
		{"z1"},
		{"z2"},
		{"z1", "z2"},
		{},
	}
	for _, zoneIDs := range testSets {
		inSet := make(map[string]bool)
		for _, id := range zoneIDs {
			inSet[id] = true
		}
		result := s.GetOperatorsByZoneIDs(zoneIDs)
		// Every returned operator must have a zone_id in the set
		for _, op := range result {
			if !inSet[op.ZoneID] {
				t.Errorf("returned operator %q has ZoneID %q not in input set %v",
					op.ID, op.ZoneID, zoneIDs)
			}
		}
		// Every operator whose zone_id is in the set must be returned
		resultIDs := make(map[string]bool)
		for _, op := range result {
			resultIDs[op.ID] = true
		}
		for _, op := range operators {
			if inSet[op.ZoneID] && !resultIDs[op.ID] {
				t.Errorf("operator %q (zone %q) should be in result for set %v but is missing",
					op.ID, op.ZoneID, zoneIDs)
			}
		}
	}
}

// TestPropertyAdapterCompleteness verifies Property 5: for any valid operator
// ID, GetOperator returns an operator with non-empty adapter fields (TS-05-P5).
func TestPropertyAdapterCompleteness(t *testing.T) {
	s := store.NewStore(makeTestZones(), makeTestOperators())
	for _, id := range []string{"op1", "op2", "op3"} {
		op, found := s.GetOperator(id)
		if !found {
			t.Errorf("operator %q not found", id)
			continue
		}
		if op.Adapter.ImageRef == "" {
			t.Errorf("operator %q: empty image_ref", id)
		}
		if op.Adapter.ChecksumSHA256 == "" {
			t.Errorf("operator %q: empty checksum_sha256", id)
		}
		if op.Adapter.Version == "" {
			t.Errorf("operator %q: empty version", id)
		}
	}
}
