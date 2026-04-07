package geo

import (
	"math"
	"math/rand"
	"testing"

	"parking-fee-service/backend/parking-fee-service/model"
)

// Munich-central polygon from default config.
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

// TS-05-2: PointInPolygon inside returns true.
func TestPointInPolygonInside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	if !PointInPolygon(point, munichCentralPolygon) {
		t.Errorf("expected point (48.1375, 11.5600) to be inside polygon, got false")
	}
}

// TS-05-2: PointInPolygon outside returns false.
func TestPointInPolygonOutside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1500, Lon: 11.5800}
	if PointInPolygon(point, munichCentralPolygon) {
		t.Errorf("expected point (48.1500, 11.5800) to be outside polygon, got true")
	}
}

// TS-05-3: FindMatchingZones includes zones where the point is outside
// the polygon but within the proximity threshold.
func TestProximityMatchingWithinThreshold(t *testing.T) {
	zones := []model.Zone{munichCentralZone}
	// Point slightly north of the polygon (~55m outside the top edge).
	nearPoint := model.Coordinate{Lat: 48.1405, Lon: 11.5600}
	result := FindMatchingZones(nearPoint, zones, 500.0)

	found := false
	for _, id := range result {
		if id == "munich-central" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected 'munich-central' in matching zones for near point, got %v", result)
	}
}

// TS-05-11: Proximity threshold is respected — points within threshold match,
// points beyond threshold do not.
func TestProximityThresholdUsed(t *testing.T) {
	zones := []model.Zone{munichCentralZone}

	// Point ~55m outside the polygon (should match with 100m threshold).
	point50m := model.Coordinate{Lat: 48.1405, Lon: 11.5600}
	near := FindMatchingZones(point50m, zones, 100.0)
	if len(near) != 1 {
		t.Errorf("expected 1 matching zone for point ~55m outside with 100m threshold, got %d: %v", len(near), near)
	}

	// Point ~2km outside the polygon (should NOT match with 100m threshold).
	point2km := model.Coordinate{Lat: 48.1600, Lon: 11.5600}
	far := FindMatchingZones(point2km, zones, 100.0)
	if len(far) != 0 {
		t.Errorf("expected 0 matching zones for point ~2km outside with 100m threshold, got %d: %v", len(far), far)
	}
}

// TS-05-P1: Property test — PointInPolygon correctness for convex polygons.
func TestPropertyPointInPolygon(t *testing.T) {
	// Generate a simple square polygon and test points known to be inside/outside.
	testCases := []struct {
		name    string
		point   model.Coordinate
		polygon []model.Coordinate
		inside  bool
	}{
		{
			name:  "center of unit square",
			point: model.Coordinate{Lat: 0.5, Lon: 0.5},
			polygon: []model.Coordinate{
				{Lat: 0, Lon: 0}, {Lat: 0, Lon: 1},
				{Lat: 1, Lon: 1}, {Lat: 1, Lon: 0},
			},
			inside: true,
		},
		{
			name:  "outside unit square",
			point: model.Coordinate{Lat: 2.0, Lon: 2.0},
			polygon: []model.Coordinate{
				{Lat: 0, Lon: 0}, {Lat: 0, Lon: 1},
				{Lat: 1, Lon: 1}, {Lat: 1, Lon: 0},
			},
			inside: false,
		},
		{
			name:  "inside munich-central",
			point: model.Coordinate{Lat: 48.1375, Lon: 11.5600},
			polygon: munichCentralPolygon,
			inside: true,
		},
		{
			name:  "far outside munich-central",
			point: model.Coordinate{Lat: 49.0, Lon: 12.0},
			polygon: munichCentralPolygon,
			inside: false,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			got := PointInPolygon(tc.point, tc.polygon)
			if got != tc.inside {
				t.Errorf("PointInPolygon(%v, polygon) = %v, want %v", tc.point, got, tc.inside)
			}
		})
	}

	// Randomised property: for random convex quads and interior points,
	// PointInPolygon must return true.
	rng := rand.New(rand.NewSource(42))
	for i := 0; i < 50; i++ {
		// Generate a random convex quad (axis-aligned rectangle for simplicity).
		minLat := rng.Float64()*180 - 90
		maxLat := minLat + rng.Float64()*10
		if maxLat > 90 {
			maxLat = 90
		}
		minLon := rng.Float64()*360 - 180
		maxLon := minLon + rng.Float64()*10
		if maxLon > 180 {
			maxLon = 180
		}
		if minLat >= maxLat || minLon >= maxLon {
			continue
		}
		polygon := []model.Coordinate{
			{Lat: minLat, Lon: minLon},
			{Lat: minLat, Lon: maxLon},
			{Lat: maxLat, Lon: maxLon},
			{Lat: maxLat, Lon: minLon},
		}
		// Point at center should be inside.
		center := model.Coordinate{
			Lat: (minLat + maxLat) / 2,
			Lon: (minLon + maxLon) / 2,
		}
		if !PointInPolygon(center, polygon) {
			t.Errorf("iteration %d: center (%v) should be inside polygon %v", i, center, polygon)
		}
	}

	// Randomised property: points far outside should not match via FindMatchingZones.
	for i := 0; i < 20; i++ {
		polygon := []model.Coordinate{
			{Lat: 10, Lon: 10}, {Lat: 10, Lon: 11},
			{Lat: 11, Lon: 11}, {Lat: 11, Lon: 10},
		}
		zone := model.Zone{ID: "test", Polygon: polygon}
		// Point very far away.
		farPoint := model.Coordinate{Lat: -50, Lon: -50}
		result := FindMatchingZones(farPoint, []model.Zone{zone}, 1000.0)
		if len(result) != 0 {
			t.Errorf("iteration %d: far point should not match, got %v", i, result)
		}
	}
}

// TS-05-P2: Property test — proximity matching.
func TestPropertyProximityMatching(t *testing.T) {
	// For a known polygon, generate points near edges and verify they match.
	polygon := []model.Coordinate{
		{Lat: 10.0, Lon: 20.0},
		{Lat: 10.0, Lon: 20.01},
		{Lat: 10.01, Lon: 20.01},
		{Lat: 10.01, Lon: 20.0},
	}
	zone := model.Zone{ID: "near-test", Polygon: polygon}

	// Points very slightly outside each edge (< 100m).
	nearPoints := []model.Coordinate{
		{Lat: 9.9995, Lon: 20.005},   // just south of bottom edge
		{Lat: 10.0105, Lon: 20.005},  // just north of top edge
		{Lat: 10.005, Lon: 19.9995},  // just west of left edge
		{Lat: 10.005, Lon: 20.0105},  // just east of right edge
	}

	threshold := 5000.0 // 5km - generous threshold to ensure these near points match.
	for i, pt := range nearPoints {
		result := FindMatchingZones(pt, []model.Zone{zone}, threshold)
		found := false
		for _, id := range result {
			if id == "near-test" {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("near point %d (%v) should match zone within %.0fm threshold, got %v", i, pt, threshold, result)
		}
	}
}

// Helper to check approximate equality of floats.
func approxEqual(a, b, epsilon float64) bool {
	return math.Abs(a-b) < epsilon
}

// Basic Haversine sanity test.
func TestHaversineDistanceSanity(t *testing.T) {
	// Distance from a point to itself should be 0.
	p := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	d := HaversineDistance(p, p)
	if !approxEqual(d, 0, 0.01) {
		t.Errorf("distance from point to itself should be ~0, got %f", d)
	}

	// Known distance: Munich (48.137, 11.575) to Berlin (52.520, 13.405) is ~504km.
	munich := model.Coordinate{Lat: 48.137, Lon: 11.575}
	berlin := model.Coordinate{Lat: 52.520, Lon: 13.405}
	dist := HaversineDistance(munich, berlin)
	if dist < 400000 || dist > 600000 {
		t.Errorf("Munich-Berlin distance should be ~504km, got %.0f m", dist)
	}
}
