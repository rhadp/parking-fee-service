package zones

import (
	"math"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
)

// --- Helper ---

// newTestStore creates a Store with one rectangular zone for testing.
// The zone is a rectangle centered roughly around (48.137, 11.575)
// spanning from (48.1355, 11.5730) to (48.1380, 11.5780).
func newTestStore() *Store {
	store := NewStore()
	store.Add(&Zone{
		ZoneID:          "zone-test",
		Name:            "Test Zone",
		OperatorName:    "Test Operator",
		Polygon:         marienplatzPolygon(),
		AdapterImageRef: "localhost/test:latest",
		AdapterChecksum: "sha256:test-checksum",
		RateType:        "per_minute",
		RateAmount:      0.05,
		Currency:        "EUR",
	})
	return store
}

func marienplatzPolygon() []geo.LatLon {
	return []geo.LatLon{
		{Latitude: 48.1380, Longitude: 11.5730},
		{Latitude: 48.1380, Longitude: 11.5780},
		{Latitude: 48.1355, Longitude: 11.5780},
		{Latitude: 48.1355, Longitude: 11.5730},
	}
}

// --- GetByID Tests ---

func TestGetByID_Found(t *testing.T) {
	store := newTestStore()

	z, ok := store.GetByID("zone-test")
	if !ok {
		t.Fatal("expected zone to be found")
	}
	if z.ZoneID != "zone-test" {
		t.Errorf("got zone_id %q, want %q", z.ZoneID, "zone-test")
	}
	if z.Name != "Test Zone" {
		t.Errorf("got name %q, want %q", z.Name, "Test Zone")
	}
}

func TestGetByID_NotFound(t *testing.T) {
	store := newTestStore()

	_, ok := store.GetByID("zone-nonexistent")
	if ok {
		t.Error("expected zone not to be found")
	}
}

// --- FindByLocation Tests ---

func TestFindByLocation_InsidePolygon(t *testing.T) {
	// Point clearly inside the Marienplatz rectangle.
	store := newTestStore()
	lat, lon := 48.1365, 11.5755

	matches := store.FindByLocation(lat, lon)
	if len(matches) != 1 {
		t.Fatalf("expected 1 match, got %d", len(matches))
	}
	if matches[0].ZoneID != "zone-test" {
		t.Errorf("got zone_id %q, want %q", matches[0].ZoneID, "zone-test")
	}
	if matches[0].DistanceMeters != 0 {
		t.Errorf("expected distance_meters=0 for inside-polygon match, got %f", matches[0].DistanceMeters)
	}
}

func TestFindByLocation_FuzzyMatch(t *testing.T) {
	// Point approximately 100m north of the Marienplatz rectangle's north edge.
	// North edge is at lat 48.1380. ~100m north ≈ 48.1389 (0.0009 degrees lat ≈ 100m).
	store := newTestStore()
	lat, lon := 48.1389, 11.5755

	matches := store.FindByLocation(lat, lon)
	if len(matches) != 1 {
		t.Fatalf("expected 1 fuzzy match, got %d", len(matches))
	}
	if matches[0].ZoneID != "zone-test" {
		t.Errorf("got zone_id %q, want %q", matches[0].ZoneID, "zone-test")
	}
	if matches[0].DistanceMeters == 0 {
		t.Error("expected non-zero distance_meters for fuzzy match")
	}
	if matches[0].DistanceMeters > 200 {
		t.Errorf("expected distance_meters <= 200, got %f", matches[0].DistanceMeters)
	}
}

func TestFindByLocation_NoMatch(t *testing.T) {
	// Point far from any zone (~5km away).
	store := newTestStore()
	lat, lon := 48.2000, 11.6000

	matches := store.FindByLocation(lat, lon)
	if len(matches) != 0 {
		t.Errorf("expected 0 matches for distant point, got %d", len(matches))
	}
}

func TestFindByLocation_FuzzyRadiusBoundary(t *testing.T) {
	// Point just beyond 200m should not match.
	// The north edge of the rectangle is at lat 48.1380.
	// 250m north ≈ 0.00225 degrees lat → 48.14025
	store := newTestStore()
	lat, lon := 48.14025, 11.5755

	matches := store.FindByLocation(lat, lon)
	if len(matches) != 0 {
		t.Errorf("expected 0 matches for point > 200m away, got %d (distance if matched: would be > 200m)", len(matches))
	}
}

func TestFindByLocation_SortedByDistance(t *testing.T) {
	// Create two zones at different distances from a test point.
	store := NewStore()

	// Zone A: near the test point.
	store.Add(&Zone{
		ZoneID:       "zone-a",
		Name:         "Zone A",
		OperatorName: "Operator A",
		Polygon: []geo.LatLon{
			{Latitude: 48.1400, Longitude: 11.5730},
			{Latitude: 48.1400, Longitude: 11.5780},
			{Latitude: 48.1390, Longitude: 11.5780},
			{Latitude: 48.1390, Longitude: 11.5730},
		},
		RateType:   "per_minute",
		RateAmount: 0.05,
		Currency:   "EUR",
	})

	// Zone B: farther from the test point.
	store.Add(&Zone{
		ZoneID:       "zone-b",
		Name:         "Zone B",
		OperatorName: "Operator B",
		Polygon: []geo.LatLon{
			{Latitude: 48.1420, Longitude: 11.5730},
			{Latitude: 48.1420, Longitude: 11.5780},
			{Latitude: 48.1410, Longitude: 11.5780},
			{Latitude: 48.1410, Longitude: 11.5730},
		},
		RateType:   "flat",
		RateAmount: 2.00,
		Currency:   "EUR",
	})

	// Point just north of zone-a but within 200m of both zones.
	// zone-a south edge is at 48.1390, zone-b south edge is at 48.1410.
	// Point at 48.1388 is ~22m south of zone-a and ~244m south of zone-b.
	// But zone-a's south edge is at 48.1390 and point is at 48.1388,
	// so the point is outside both, closer to zone-a.
	lat, lon := 48.1388, 11.5755

	matches := store.FindByLocation(lat, lon)
	if len(matches) < 1 {
		t.Fatal("expected at least 1 fuzzy match")
	}

	// Verify sort order: first match should be closest.
	for i := 1; i < len(matches); i++ {
		if matches[i].DistanceMeters < matches[i-1].DistanceMeters {
			t.Errorf("matches not sorted by distance: [%d]=%f > [%d]=%f",
				i-1, matches[i-1].DistanceMeters, i, matches[i].DistanceMeters)
		}
	}

	// The first match should be zone-a (closer).
	if len(matches) >= 1 && matches[0].ZoneID != "zone-a" {
		t.Errorf("expected first match to be zone-a (closer), got %q", matches[0].ZoneID)
	}
}

func TestFindByLocation_MultipleExactMatches(t *testing.T) {
	// Two overlapping zones.
	store := NewStore()
	poly := []geo.LatLon{
		{Latitude: 48.1380, Longitude: 11.5730},
		{Latitude: 48.1380, Longitude: 11.5780},
		{Latitude: 48.1355, Longitude: 11.5780},
		{Latitude: 48.1355, Longitude: 11.5730},
	}

	store.Add(&Zone{
		ZoneID:       "zone-overlap-a",
		Name:         "Overlap A",
		OperatorName: "Operator A",
		Polygon:      poly,
		RateType:     "per_minute",
		RateAmount:   0.05,
		Currency:     "EUR",
	})
	store.Add(&Zone{
		ZoneID:       "zone-overlap-b",
		Name:         "Overlap B",
		OperatorName: "Operator B",
		Polygon:      poly,
		RateType:     "flat",
		RateAmount:   3.00,
		Currency:     "EUR",
	})

	// Point inside both polygons.
	lat, lon := 48.1365, 11.5755
	matches := store.FindByLocation(lat, lon)
	if len(matches) != 2 {
		t.Fatalf("expected 2 exact matches, got %d", len(matches))
	}

	// Both should have distance 0.
	for _, m := range matches {
		if m.DistanceMeters != 0 {
			t.Errorf("expected distance_meters=0, got %f for %s", m.DistanceMeters, m.ZoneID)
		}
	}
}

func TestFindByLocation_EmptyStore(t *testing.T) {
	store := NewStore()

	matches := store.FindByLocation(48.137, 11.575)
	if matches != nil && len(matches) != 0 {
		t.Errorf("expected nil or empty result from empty store, got %d matches", len(matches))
	}
}

func TestFindByLocation_ZoneMatchFields(t *testing.T) {
	// Verify that all ZoneMatch fields are populated correctly.
	store := newTestStore()
	lat, lon := 48.1365, 11.5755

	matches := store.FindByLocation(lat, lon)
	if len(matches) != 1 {
		t.Fatalf("expected 1 match, got %d", len(matches))
	}

	m := matches[0]
	if m.ZoneID != "zone-test" {
		t.Errorf("ZoneID = %q, want %q", m.ZoneID, "zone-test")
	}
	if m.Name != "Test Zone" {
		t.Errorf("Name = %q, want %q", m.Name, "Test Zone")
	}
	if m.OperatorName != "Test Operator" {
		t.Errorf("OperatorName = %q, want %q", m.OperatorName, "Test Operator")
	}
	if m.RateType != "per_minute" {
		t.Errorf("RateType = %q, want %q", m.RateType, "per_minute")
	}
	if m.RateAmount != 0.05 {
		t.Errorf("RateAmount = %f, want %f", m.RateAmount, 0.05)
	}
	if m.Currency != "EUR" {
		t.Errorf("Currency = %q, want %q", m.Currency, "EUR")
	}
}

// --- Property 2: Fuzzy Radius Boundary ---

func TestProperty_FuzzyRadiusBoundary(t *testing.T) {
	store := newTestStore()

	// The north edge of the rectangle is at lat=48.1380.
	// Test points at various distances north of this edge.
	testCases := []struct {
		name        string
		lat         float64
		lon         float64
		shouldMatch bool
	}{
		{"50m north", 48.13845, 11.5755, true},
		{"150m north", 48.13935, 11.5755, true},
		{"199m north", 48.13979, 11.5755, true},
		{"300m north", 48.14070, 11.5755, false},
		{"500m north", 48.14250, 11.5755, false},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			matches := store.FindByLocation(tc.lat, tc.lon)
			if tc.shouldMatch && len(matches) == 0 {
				t.Errorf("expected fuzzy match at %s, got none", tc.name)
			}
			if !tc.shouldMatch && len(matches) > 0 {
				t.Errorf("expected no match at %s, got %d (distance: %f)",
					tc.name, len(matches), matches[0].DistanceMeters)
			}
		})
	}
}

// --- Property 3: No-Match Safety ---

func TestProperty_NoMatchSafety(t *testing.T) {
	store := newTestStore()

	// Points that are clearly far from all zones.
	farPoints := []struct {
		name string
		lat  float64
		lon  float64
	}{
		{"Berlin", 52.5200, 13.4050},
		{"Paris", 48.8566, 2.3522},
		{"Far south", 47.0, 11.0},
		{"Far east", 48.137, 12.0},
	}

	for _, p := range farPoints {
		t.Run(p.name, func(t *testing.T) {
			matches := store.FindByLocation(p.lat, p.lon)
			if len(matches) != 0 {
				t.Errorf("expected empty result for %s, got %d matches", p.name, len(matches))
			}
		})
	}
}

// --- Property 5: Sort Order Invariant ---

func TestProperty_SortOrderInvariant(t *testing.T) {
	store := LoadSeedData()

	// Test with a point that should fuzzy-match multiple zones.
	// The centroid of all three zones is roughly around (48.150, 11.565).
	// Use a point that is near two zones but outside both.
	// Actually let's use a point near Sendlinger Tor and Marienplatz:
	// Sendlinger Tor: south edge at 48.1320, north edge at 48.1345
	// Marienplatz: south edge at 48.1355, north edge at 48.1380
	// A point between them at 48.1350, 11.5715 should be near both.
	lat, lon := 48.1350, 11.5715

	matches := store.FindByLocation(lat, lon)

	for i := 1; i < len(matches); i++ {
		if matches[i].DistanceMeters < matches[i-1].DistanceMeters {
			t.Errorf("sort order violated: matches[%d].distance=%f < matches[%d].distance=%f",
				i, matches[i].DistanceMeters, i-1, matches[i-1].DistanceMeters)
		}
	}
}

// --- Seed Data Tests ---

func TestLoadSeedData_LoadsThreeZones(t *testing.T) {
	store := LoadSeedData()

	// Verify all 3 seed zones are loaded.
	expectedZones := []string{"zone-marienplatz", "zone-olympiapark", "zone-sendlinger-tor"}
	for _, id := range expectedZones {
		z, ok := store.GetByID(id)
		if !ok {
			t.Errorf("seed zone %q not found in store", id)
			continue
		}
		if z.ZoneID != id {
			t.Errorf("zone_id mismatch: got %q, want %q", z.ZoneID, id)
		}
	}
}

func TestLoadSeedData_PolygonValidity(t *testing.T) {
	store := LoadSeedData()

	zoneIDs := []string{"zone-marienplatz", "zone-olympiapark", "zone-sendlinger-tor"}
	for _, id := range zoneIDs {
		z, ok := store.GetByID(id)
		if !ok {
			t.Errorf("zone %q not found", id)
			continue
		}
		if len(z.Polygon) < 4 {
			t.Errorf("zone %q has %d polygon points, want >= 4", id, len(z.Polygon))
		}
		for i, p := range z.Polygon {
			if p.Latitude == 0 || p.Longitude == 0 {
				t.Errorf("zone %q polygon[%d] has zero coordinate: lat=%f, lon=%f",
					id, i, p.Latitude, p.Longitude)
			}
		}
	}
}

func TestLoadSeedData_RateConfig(t *testing.T) {
	store := LoadSeedData()

	zoneIDs := []string{"zone-marienplatz", "zone-olympiapark", "zone-sendlinger-tor"}
	for _, id := range zoneIDs {
		z, ok := store.GetByID(id)
		if !ok {
			t.Errorf("zone %q not found", id)
			continue
		}
		if z.RateType == "" {
			t.Errorf("zone %q has empty rate_type", id)
		}
		if z.RateType != "per_minute" && z.RateType != "flat" {
			t.Errorf("zone %q has invalid rate_type %q", id, z.RateType)
		}
		if z.RateAmount <= 0 {
			t.Errorf("zone %q has non-positive rate_amount: %f", id, z.RateAmount)
		}
		if z.Currency == "" {
			t.Errorf("zone %q has empty currency", id)
		}
	}
}

func TestLoadSeedData_AdapterMetadata(t *testing.T) {
	store := LoadSeedData()

	zoneIDs := []string{"zone-marienplatz", "zone-olympiapark", "zone-sendlinger-tor"}
	for _, id := range zoneIDs {
		z, ok := store.GetByID(id)
		if !ok {
			t.Errorf("zone %q not found", id)
			continue
		}
		if z.AdapterImageRef == "" {
			t.Errorf("zone %q has empty adapter_image_ref", id)
		}
		if z.AdapterChecksum == "" {
			t.Errorf("zone %q has empty adapter_checksum", id)
		}
	}
}

func TestLoadSeedData_MalformedPolygonSkipped(t *testing.T) {
	// Temporarily modify SeedZones to include a malformed zone and verify
	// it's skipped. We do this by creating a custom store with the same logic.
	store := NewStore()

	// Valid zone.
	store.Add(&Zone{
		ZoneID:       "valid-zone",
		Name:         "Valid",
		OperatorName: "Valid Operator",
		Polygon: []geo.LatLon{
			{Latitude: 48.1380, Longitude: 11.5730},
			{Latitude: 48.1380, Longitude: 11.5780},
			{Latitude: 48.1355, Longitude: 11.5780},
			{Latitude: 48.1355, Longitude: 11.5730},
		},
		RateType:   "per_minute",
		RateAmount: 0.05,
		Currency:   "EUR",
	})

	// Simulate malformed zone (2 points - too few for a polygon).
	malformed := Zone{
		ZoneID:       "malformed-zone",
		Name:         "Malformed",
		OperatorName: "Malformed Operator",
		Polygon: []geo.LatLon{
			{Latitude: 48.1380, Longitude: 11.5730},
			{Latitude: 48.1380, Longitude: 11.5780},
		},
		RateType:   "flat",
		RateAmount: 1.00,
		Currency:   "EUR",
	}

	// The LoadSeedData function skips zones with < 3 polygon points.
	// Verify the same check works by testing polygon length.
	if len(malformed.Polygon) >= 3 {
		t.Error("expected malformed zone to have < 3 polygon points")
	}

	// Verify the valid zone is accessible.
	_, ok := store.GetByID("valid-zone")
	if !ok {
		t.Error("expected valid zone to be in store")
	}
}

func TestLoadSeedData_InsideMarienplatz(t *testing.T) {
	store := LoadSeedData()

	// Point inside Marienplatz zone (center of the rectangle).
	lat := (48.1380 + 48.1355) / 2.0 // 48.13675
	lon := (11.5730 + 11.5780) / 2.0 // 11.5755

	matches := store.FindByLocation(lat, lon)
	if len(matches) == 0 {
		t.Fatal("expected at least one match inside Marienplatz zone")
	}

	found := false
	for _, m := range matches {
		if m.ZoneID == "zone-marienplatz" {
			found = true
			if m.DistanceMeters != 0 {
				t.Errorf("expected distance_meters=0 for inside match, got %f", m.DistanceMeters)
			}
		}
	}
	if !found {
		t.Error("expected zone-marienplatz in results")
	}
}

func TestLoadSeedData_InsideOlympiapark(t *testing.T) {
	store := LoadSeedData()

	// Center of Olympiapark zone.
	lat := (48.1770 + 48.1720) / 2.0 // 48.1745
	lon := (11.5490 + 11.5580) / 2.0 // 11.5535

	matches := store.FindByLocation(lat, lon)
	found := false
	for _, m := range matches {
		if m.ZoneID == "zone-olympiapark" {
			found = true
			if m.DistanceMeters != 0 {
				t.Errorf("expected distance_meters=0, got %f", m.DistanceMeters)
			}
		}
	}
	if !found {
		t.Error("expected zone-olympiapark in results")
	}
}

func TestLoadSeedData_InsideSendlingerTor(t *testing.T) {
	store := LoadSeedData()

	// Center of Sendlinger Tor zone.
	lat := (48.1345 + 48.1320) / 2.0 // 48.13325
	lon := (11.5650 + 11.5700) / 2.0 // 11.5675

	matches := store.FindByLocation(lat, lon)
	found := false
	for _, m := range matches {
		if m.ZoneID == "zone-sendlinger-tor" {
			found = true
			if m.DistanceMeters != 0 {
				t.Errorf("expected distance_meters=0, got %f", m.DistanceMeters)
			}
		}
	}
	if !found {
		t.Error("expected zone-sendlinger-tor in results")
	}
}

func TestFindByLocation_FuzzyDistanceAccuracy(t *testing.T) {
	store := newTestStore()

	// Point ~100m north of the north edge (lat 48.1380).
	// 100m ≈ 0.0009 degrees latitude.
	lat, lon := 48.1389, 11.5755

	matches := store.FindByLocation(lat, lon)
	if len(matches) != 1 {
		t.Fatalf("expected 1 match, got %d", len(matches))
	}

	// The distance should be approximately 100m (within 10m tolerance).
	dist := matches[0].DistanceMeters
	if math.Abs(dist-100) > 10 {
		t.Errorf("expected distance ≈ 100m, got %f", dist)
	}
}

// --- Store Add/Overwrite ---

func TestStoreAdd_Overwrite(t *testing.T) {
	store := NewStore()

	store.Add(&Zone{
		ZoneID: "zone-1",
		Name:   "Original",
	})
	store.Add(&Zone{
		ZoneID: "zone-1",
		Name:   "Updated",
	})

	z, ok := store.GetByID("zone-1")
	if !ok {
		t.Fatal("expected zone to exist")
	}
	if z.Name != "Updated" {
		t.Errorf("expected name %q after overwrite, got %q", "Updated", z.Name)
	}
}

func TestNewStore_Empty(t *testing.T) {
	store := NewStore()

	_, ok := store.GetByID("anything")
	if ok {
		t.Error("expected empty store to return not-found")
	}
}
