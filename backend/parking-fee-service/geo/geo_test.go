package geo_test

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// munichCentralPolygon is the square polygon from the Munich Central zone demo data.
// Vertices (lat/lon): (48.14,11.555) → (48.14,11.565) → (48.135,11.565) → (48.135,11.555)
var munichCentralPolygon = []model.Coordinate{
	{Lat: 48.1400, Lon: 11.5550},
	{Lat: 48.1400, Lon: 11.5650},
	{Lat: 48.1350, Lon: 11.5650},
	{Lat: 48.1350, Lon: 11.5550},
}

var munichCentralZone = model.Zone{
	ID:      "munich-central",
	Name:    "Munich Central Station Area",
	Polygon: munichCentralPolygon,
}

// TS-05-2: PointInPolygon returns true for a point inside the polygon.
func TestPointInPolygonInside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1375, Lon: 11.5600} // centre of the square
	if !geo.PointInPolygon(point, munichCentralPolygon) {
		t.Errorf("PointInPolygon(%v, polygon) = false, want true", point)
	}
}

// TS-05-2: PointInPolygon returns false for a point outside the polygon.
func TestPointInPolygonOutside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1500, Lon: 11.5800} // clearly outside
	if geo.PointInPolygon(point, munichCentralPolygon) {
		t.Errorf("PointInPolygon(%v, polygon) = true, want false", point)
	}
}

// TS-05-3: FindMatchingZones includes zones whose edge is within the threshold.
func TestProximityMatchingWithinThreshold(t *testing.T) {
	// The north edge of munich-central is at lat=48.140.
	// A point ~111m north of that edge should match with a 500m threshold.
	// 1 degree lat ≈ 111 km, so 0.001° ≈ 111 m.
	nearPoint := model.Coordinate{Lat: 48.1410, Lon: 11.5600}
	zones := []model.Zone{munichCentralZone}
	result := geo.FindMatchingZones(nearPoint, zones, 500.0)
	if !containsZone(result, "munich-central") {
		t.Errorf("FindMatchingZones(%v, ..., 500) did not include munich-central; got %v", nearPoint, result)
	}
}

// TS-05-11: Points within the configured threshold match; points beyond it do not.
func TestProximityThresholdUsed(t *testing.T) {
	zones := []model.Zone{munichCentralZone}

	// Point ~50 m north of the north edge (lat 48.140).
	// 0.001° lat ≈ 111 m, so 0.0005° ≈ 55 m.
	near50m := model.Coordinate{Lat: 48.14045, Lon: 11.5600}
	nearResult := geo.FindMatchingZones(near50m, zones, 100.0)
	if !containsZone(nearResult, "munich-central") {
		t.Errorf("point ~50m outside: expected munich-central in result with 100m threshold, got %v", nearResult)
	}

	// Point ~200 m north of the north edge.
	// 0.002° lat ≈ 222 m.
	far200m := model.Coordinate{Lat: 48.1420, Lon: 11.5600}
	farResult := geo.FindMatchingZones(far200m, zones, 100.0)
	if containsZone(farResult, "munich-central") {
		t.Errorf("point ~200m outside: did not expect munich-central in result with 100m threshold, got %v", farResult)
	}
}

// TS-05-P1: Point-in-polygon correctness — table-driven with known inside/outside points.
func TestPropertyPointInPolygon(t *testing.T) {
	cases := []struct {
		name   string
		point  model.Coordinate
		inside bool
	}{
		{"centre", model.Coordinate{Lat: 48.1375, Lon: 11.5600}, true},
		{"north-west corner inside", model.Coordinate{Lat: 48.1398, Lon: 11.5552}, true},
		{"south-east corner inside", model.Coordinate{Lat: 48.1352, Lon: 11.5648}, true},
		{"far north", model.Coordinate{Lat: 48.1500, Lon: 11.5600}, false},
		{"far south", model.Coordinate{Lat: 48.1200, Lon: 11.5600}, false},
		{"far east", model.Coordinate{Lat: 48.1375, Lon: 11.5800}, false},
		{"far west", model.Coordinate{Lat: 48.1375, Lon: 11.5400}, false},
		{"Gulf of Guinea (0,0)", model.Coordinate{Lat: 0.0, Lon: 0.0}, false},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := geo.PointInPolygon(tc.point, munichCentralPolygon)
			if got != tc.inside {
				t.Errorf("PointInPolygon(%v) = %v, want %v", tc.point, got, tc.inside)
			}
		})
	}
}

// TS-05-P2: Proximity matching — points near the polygon edge are included.
func TestPropertyProximityMatching(t *testing.T) {
	zones := []model.Zone{munichCentralZone}
	threshold := 200.0 // metres

	// Points just outside each edge — within 200 m.
	nearPoints := []model.Coordinate{
		{Lat: 48.1402, Lon: 11.5600}, // ~22 m north of north edge
		{Lat: 48.1348, Lon: 11.5600}, // ~22 m south of south edge
		{Lat: 48.1375, Lon: 11.5655}, // ~37 m east of east edge
		{Lat: 48.1375, Lon: 11.5545}, // ~37 m west of west edge
	}
	for _, p := range nearPoints {
		result := geo.FindMatchingZones(p, zones, threshold)
		if !containsZone(result, "munich-central") {
			t.Errorf("point %v within %.0fm threshold: expected munich-central in %v", p, threshold, result)
		}
	}

	// Points far outside — beyond 200 m.
	farPoints := []model.Coordinate{
		{Lat: 48.1430, Lon: 11.5600}, // ~330 m north
		{Lat: 48.1320, Lon: 11.5600}, // ~330 m south
	}
	for _, p := range farPoints {
		result := geo.FindMatchingZones(p, zones, threshold)
		if containsZone(result, "munich-central") {
			t.Errorf("point %v beyond %.0fm threshold: did not expect munich-central in %v", p, threshold, result)
		}
	}
}

// HaversineDistance sanity check: distance between the same point should be ~0.
func TestHaversineDistanceSamePoint(t *testing.T) {
	p := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	d := geo.HaversineDistance(p, p)
	if d != 0 {
		t.Errorf("HaversineDistance(p, p) = %v, want 0", d)
	}
}

// HaversineDistance: known distance between two Munich landmarks (~2 km apart).
func TestHaversineDistanceKnown(t *testing.T) {
	// Munich Central Station ↔ Marienplatz: ~1.5–2 km
	central := model.Coordinate{Lat: 48.1402, Lon: 11.5602}
	marien := model.Coordinate{Lat: 48.1374, Lon: 11.5755}
	d := geo.HaversineDistance(central, marien)
	if d < 1000 || d > 3000 {
		t.Errorf("HaversineDistance(central, marienplatz) = %.1f m, expected 1000–3000 m", d)
	}
}

// containsZone reports whether id is present in ids.
func containsZone(ids []string, id string) bool {
	for _, s := range ids {
		if s == id {
			return true
		}
	}
	return false
}
