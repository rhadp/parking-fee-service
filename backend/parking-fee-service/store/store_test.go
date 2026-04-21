// Package store_test contains tests for the store package.
package store_test

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/store"
)

// testZones returns a set of zones for use in tests.
func testZones() []model.Zone {
	return []model.Zone{
		{
			ID:   "zone-1",
			Name: "Zone One",
			Polygon: []model.Coordinate{
				{Lat: 48.14, Lon: 11.555},
				{Lat: 48.14, Lon: 11.565},
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

// testOperators returns a set of operators for use in tests.
func testOperators() []model.Operator {
	return []model.Operator{
		{
			ID:     "op-1",
			Name:   "Operator One",
			ZoneID: "zone-1",
			Rate:   model.Rate{Type: "per-hour", Amount: 2.50, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "example.com/op1:v1",
				ChecksumSHA256: "sha256:op1sum",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op-2",
			Name:   "Operator Two",
			ZoneID: "zone-1",
			Rate:   model.Rate{Type: "flat-fee", Amount: 5.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "example.com/op2:v1",
				ChecksumSHA256: "sha256:op2sum",
				Version:        "1.0.0",
			},
		},
		{
			ID:     "op-3",
			Name:   "Operator Three",
			ZoneID: "zone-2",
			Rate:   model.Rate{Type: "per-hour", Amount: 3.00, Currency: "EUR"},
			Adapter: model.AdapterMeta{
				ImageRef:       "example.com/op3:v1",
				ChecksumSHA256: "sha256:op3sum",
				Version:        "1.0.0",
			},
		},
	}
}

// TestMultipleOperatorsReturned verifies that GetOperatorsByZoneIDs returns all
// operators whose zone_id matches any of the given IDs.
// TS-05-4
func TestMultipleOperatorsReturned(t *testing.T) {
	zones := testZones()
	operators := testOperators()
	st := store.NewStore(zones, operators)

	// Both op-1 and op-2 are in zone-1.
	result := st.GetOperatorsByZoneIDs([]string{"zone-1"})
	if len(result) != 2 {
		t.Errorf("expected 2 operators for zone-1, got %d: %v", len(result), result)
	}

	// Verify both operators are present.
	ids := make(map[string]bool)
	for _, op := range result {
		ids[op.ID] = true
	}
	if !ids["op-1"] {
		t.Errorf("expected op-1 in result")
	}
	if !ids["op-2"] {
		t.Errorf("expected op-2 in result")
	}
}

// TestGetOperatorFound verifies GetOperator returns a known operator.
func TestGetOperatorFound(t *testing.T) {
	st := store.NewStore(testZones(), testOperators())

	op, found := st.GetOperator("op-1")
	if !found {
		t.Fatalf("expected to find op-1, got not found")
	}
	if op.ID != "op-1" {
		t.Errorf("expected ID=op-1, got %s", op.ID)
	}
}

// TestGetOperatorNotFound verifies GetOperator returns false for unknown IDs.
func TestGetOperatorNotFound(t *testing.T) {
	st := store.NewStore(testZones(), testOperators())

	_, found := st.GetOperator("nonexistent-operator")
	if found {
		t.Error("expected not found for nonexistent-operator, got found")
	}
}

// TestGetZoneFound verifies GetZone returns a known zone.
func TestGetZoneFound(t *testing.T) {
	st := store.NewStore(testZones(), testOperators())

	zone, found := st.GetZone("zone-1")
	if !found {
		t.Fatalf("expected to find zone-1, got not found")
	}
	if zone.ID != "zone-1" {
		t.Errorf("expected ID=zone-1, got %s", zone.ID)
	}
}

// TestGetOperatorsByZoneIDsMultipleZones verifies GetOperatorsByZoneIDs works
// with multiple zone IDs.
func TestGetOperatorsByZoneIDsMultipleZones(t *testing.T) {
	st := store.NewStore(testZones(), testOperators())

	result := st.GetOperatorsByZoneIDs([]string{"zone-1", "zone-2"})
	if len(result) != 3 {
		t.Errorf("expected 3 operators for zone-1 and zone-2, got %d", len(result))
	}
}

// TestGetOperatorsByZoneIDsEmpty verifies empty zone ID list returns no operators.
func TestGetOperatorsByZoneIDsEmpty(t *testing.T) {
	st := store.NewStore(testZones(), testOperators())

	result := st.GetOperatorsByZoneIDs([]string{})
	if len(result) != 0 {
		t.Errorf("expected 0 operators for empty zone IDs, got %d", len(result))
	}
}

// TestPropertyOperatorZoneAssociation is a property test verifying that
// GetOperatorsByZoneIDs returns all and only operators whose zone_id is in the set.
// TS-05-P3
func TestPropertyOperatorZoneAssociation(t *testing.T) {
	operators := testOperators()
	st := store.NewStore(testZones(), operators)

	// Test all subsets of zone IDs.
	subsets := [][]string{
		{},
		{"zone-1"},
		{"zone-2"},
		{"zone-1", "zone-2"},
	}

	for _, zoneIDs := range subsets {
		zoneSet := make(map[string]bool)
		for _, z := range zoneIDs {
			zoneSet[z] = true
		}

		result := st.GetOperatorsByZoneIDs(zoneIDs)

		// Every returned operator must have zone_id in the set.
		for _, op := range result {
			if !zoneSet[op.ZoneID] {
				t.Errorf("GetOperatorsByZoneIDs(%v): returned operator %s with zone_id %s not in set",
					zoneIDs, op.ID, op.ZoneID)
			}
		}

		// Every operator with zone_id in the set must be in the result.
		resultIDs := make(map[string]bool)
		for _, op := range result {
			resultIDs[op.ID] = true
		}
		for _, op := range operators {
			if zoneSet[op.ZoneID] && !resultIDs[op.ID] {
				t.Errorf("GetOperatorsByZoneIDs(%v): operator %s (zone_id=%s) missing from result",
					zoneIDs, op.ID, op.ZoneID)
			}
		}
	}
}

// TestPropertyAdapterCompleteness is a property test verifying that for every
// valid operator ID, GetOperator returns an operator with non-empty adapter fields.
// TS-05-P5
func TestPropertyAdapterCompleteness(t *testing.T) {
	operators := testOperators()
	st := store.NewStore(testZones(), operators)

	for _, expected := range operators {
		t.Run(expected.ID, func(t *testing.T) {
			op, found := st.GetOperator(expected.ID)
			if !found {
				t.Fatalf("GetOperator(%q) returned not found", expected.ID)
			}
			if op.Adapter.ImageRef == "" {
				t.Errorf("operator %s has empty image_ref", op.ID)
			}
			if op.Adapter.ChecksumSHA256 == "" {
				t.Errorf("operator %s has empty checksum_sha256", op.ID)
			}
			if op.Adapter.Version == "" {
				t.Errorf("operator %s has empty version", op.ID)
			}
		})
	}
}
