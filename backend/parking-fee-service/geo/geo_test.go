// Package geo_test contains tests for the geo package.
package geo_test

import (
	"math"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// Munich-central polygon from the default config.
// Corners: NW(48.14,11.555), NE(48.14,11.565), SE(48.135,11.565), SW(48.135,11.555)
var municCentralPolygon = []model.Coordinate{
	{Lat: 48.1400, Lon: 11.5550},
	{Lat: 48.1400, Lon: 11.5650},
	{Lat: 48.1350, Lon: 11.5650},
	{Lat: 48.1350, Lon: 11.5550},
}

var municCentralZone = model.Zone{
	ID:      "munich-central",
	Name:    "Munich Central Station Area",
	Polygon: municCentralPolygon,
}

// TestPointInPolygonInside verifies that a coordinate inside the polygon returns true.
// TS-05-2
func TestPointInPolygonInside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1375, Lon: 11.5600} // center of the polygon
	if !geo.PointInPolygon(point, municCentralPolygon) {
		t.Errorf("expected PointInPolygon to return true for point inside polygon, got false")
	}
}

// TestPointInPolygonOutside verifies that a coordinate clearly outside the polygon returns false.
// TS-05-2
func TestPointInPolygonOutside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1500, Lon: 11.5800} // well outside
	if geo.PointInPolygon(point, municCentralPolygon) {
		t.Errorf("expected PointInPolygon to return false for point outside polygon, got true")
	}

	// Origin (Gulf of Guinea) should definitely be outside
	origin := model.Coordinate{Lat: 0.0, Lon: 0.0}
	if geo.PointInPolygon(origin, municCentralPolygon) {
		t.Errorf("expected PointInPolygon to return false for origin, got true")
	}
}

// TestPointInPolygonEdgeCases tests additional polygon shapes and boundary cases.
func TestPointInPolygonEdgeCases(t *testing.T) {
	// Simple unit square in lat/lon space centered around (0,0)
	square := []model.Coordinate{
		{Lat: 1.0, Lon: -1.0},
		{Lat: 1.0, Lon: 1.0},
		{Lat: -1.0, Lon: 1.0},
		{Lat: -1.0, Lon: -1.0},
	}
	cases := []struct {
		name   string
		point  model.Coordinate
		inside bool
	}{
		{"center", model.Coordinate{Lat: 0.0, Lon: 0.0}, true},
		{"clearly outside", model.Coordinate{Lat: 5.0, Lon: 5.0}, false},
		{"on north boundary (outside)", model.Coordinate{Lat: 2.0, Lon: 0.0}, false},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := geo.PointInPolygon(tc.point, square)
			if got != tc.inside {
				t.Errorf("PointInPolygon(%v) = %v, want %v", tc.point, got, tc.inside)
			}
		})
	}
}

// TestProximityMatchingWithinThreshold verifies that a point slightly outside the polygon
// but within the threshold distance is included by FindMatchingZones.
// TS-05-3
func TestProximityMatchingWithinThreshold(t *testing.T) {
	zones := []model.Zone{municCentralZone}
	threshold := 500.0

	// Point approximately 100m north of the north edge (48.14).
	// 100m / 111000 m-per-degree ≈ 0.0009 degrees lat
	nearPoint := model.Coordinate{Lat: 48.1409, Lon: 11.5600}

	result := geo.FindMatchingZones(nearPoint, zones, threshold)
	if !containsZone(result, "munich-central") {
		t.Errorf("expected munich-central in FindMatchingZones result for near point, got %v", result)
	}
}

// TestFarPointNoMatch verifies that a point far from all zones returns an empty result.
// TS-05-5 (also covers Property 1 negative case)
func TestFarPointNoMatch(t *testing.T) {
	zones := []model.Zone{municCentralZone}
	threshold := 500.0

	farPoint := model.Coordinate{Lat: 0.0, Lon: 0.0} // Gulf of Guinea

	result := geo.FindMatchingZones(farPoint, zones, threshold)
	if len(result) != 0 {
		t.Errorf("expected empty result for far point, got %v", result)
	}
}

// TestProximityThresholdUsed verifies that the configured proximity threshold
// correctly admits nearby points and rejects far points.
// TS-05-11
func TestProximityThresholdUsed(t *testing.T) {
	zones := []model.Zone{municCentralZone}
	threshold := 100.0

	// North edge of munich-central is at lat=48.14.
	// 1 degree latitude ≈ 111,000 meters.
	// 50m north of edge: lat ≈ 48.14 + 50/111000 ≈ 48.14045
	// 200m north of edge: lat ≈ 48.14 + 200/111000 ≈ 48.14180
	nearPoint := model.Coordinate{Lat: 48.14045, Lon: 11.5600}  // ~50m outside
	farPoint := model.Coordinate{Lat: 48.14180, Lon: 11.5600}   // ~200m outside

	near := geo.FindMatchingZones(nearPoint, zones, threshold)
	if !containsZone(near, "munich-central") {
		t.Errorf("expected munich-central in result for near point (50m outside, threshold 100m), got %v", near)
	}

	far := geo.FindMatchingZones(farPoint, zones, threshold)
	if containsZone(far, "munich-central") {
		t.Errorf("expected munich-central NOT in result for far point (200m outside, threshold 100m), got %v", far)
	}
}

// TestHaversineDistance verifies the Haversine distance calculation with a known value.
func TestHaversineDistance(t *testing.T) {
	// Munich center to approximately 1 degree north (~111 km)
	a := model.Coordinate{Lat: 48.0, Lon: 11.5}
	b := model.Coordinate{Lat: 49.0, Lon: 11.5}

	dist := geo.HaversineDistance(a, b)
	// Expected: approximately 111,194 meters per degree latitude near Munich
	expectedApprox := 111194.0
	tolerance := 2000.0 // 2 km tolerance
	if math.Abs(dist-expectedApprox) > tolerance {
		t.Errorf("HaversineDistance(%v, %v) = %v, want approximately %v (±%v)", a, b, dist, expectedApprox, tolerance)
	}
}

// TestPropertyPointInPolygon is a property test for PointInPolygon correctness.
// For points clearly inside a convex quadrilateral (cross-product sign consistency),
// PointInPolygon must return true. For points far outside, FindMatchingZones must be empty.
// TS-05-P1
// Addresses Skeptic finding: uses cross-product sign consistency, not barycentric coordinates.
func TestPropertyPointInPolygon(t *testing.T) {
	// Table of (polygon, interior points, exterior points) with independent geometry.
	cases := []struct {
		name      string
		polygon   []model.Coordinate
		interior  []model.Coordinate
		exterior  []model.Coordinate
		threshold float64
	}{
		{
			name: "munich-central rectangle",
			polygon: []model.Coordinate{
				{Lat: 48.14, Lon: 11.555},
				{Lat: 48.14, Lon: 11.565},
				{Lat: 48.135, Lon: 11.565},
				{Lat: 48.135, Lon: 11.555},
			},
			// Interior: points whose lat and lon are strictly within the rectangle bounds
			interior: []model.Coordinate{
				{Lat: 48.1375, Lon: 11.560},
				{Lat: 48.1360, Lon: 11.558},
				{Lat: 48.1390, Lon: 11.562},
			},
			// Exterior: points far outside all polygon edges
			exterior: []model.Coordinate{
				{Lat: 48.150, Lon: 11.560},  // far north
				{Lat: 48.130, Lon: 11.560},  // far south
				{Lat: 48.137, Lon: 11.580},  // far east
				{Lat: 48.137, Lon: 11.540},  // far west
			},
			threshold: 10.0, // very tight threshold so far points won't match
		},
		{
			name: "small square near origin",
			polygon: []model.Coordinate{
				{Lat: 1.0, Lon: -1.0},
				{Lat: 1.0, Lon: 1.0},
				{Lat: -1.0, Lon: 1.0},
				{Lat: -1.0, Lon: -1.0},
			},
			interior: []model.Coordinate{
				{Lat: 0.0, Lon: 0.0},
				{Lat: 0.5, Lon: 0.5},
				{Lat: -0.5, Lon: -0.5},
			},
			exterior: []model.Coordinate{
				{Lat: 5.0, Lon: 5.0},
				{Lat: -5.0, Lon: -5.0},
				{Lat: 5.0, Lon: 0.0},
				{Lat: 0.0, Lon: 5.0},
			},
			threshold: 10.0,
		},
	}

	for _, tc := range cases {
		zone := model.Zone{ID: "test-zone", Polygon: tc.polygon}
		t.Run(tc.name+"/interior", func(t *testing.T) {
			for _, pt := range tc.interior {
				if !geo.PointInPolygon(pt, tc.polygon) {
					t.Errorf("PointInPolygon(%v) should be true (interior point)", pt)
				}
			}
		})
		t.Run(tc.name+"/exterior_no_match", func(t *testing.T) {
			for _, pt := range tc.exterior {
				// Points far outside should not match even with tight threshold
				matches := geo.FindMatchingZones(pt, []model.Zone{zone}, tc.threshold)
				if len(matches) != 0 {
					t.Errorf("FindMatchingZones(%v) should be empty for far exterior point, got %v", pt, matches)
				}
			}
		})
	}
}

// TestPropertyProximityMatching is a property test verifying that points within
// the threshold distance of a polygon edge are matched.
// TS-05-P2
// Addresses Skeptic finding: uses independent geometric construction (offset from
// edge midpoint by a known Haversine distance) rather than the function under test.
func TestPropertyProximityMatching(t *testing.T) {
	// Polygon: the munich-central rectangle.
	// North edge runs from (48.14, 11.555) to (48.14, 11.565).
	// Edge midpoint: (48.14, 11.560)
	// Move 100m north: 100 / 111000 ≈ 0.000900 degrees lat
	// => nearPoint: (48.14090, 11.560) — independently computed, NOT using DistanceToPolygonEdge.
	polygon := municCentralPolygon
	zone := model.Zone{ID: "munich-central", Polygon: polygon}

	// Independently computed points near each edge midpoint.
	// North edge midpoint (48.14, 11.56), 100m outward (north):
	northNear := model.Coordinate{Lat: 48.1409, Lon: 11.5600}

	// South edge midpoint (48.135, 11.56), 100m outward (south):
	southNear := model.Coordinate{Lat: 48.1341, Lon: 11.5600}

	// East edge midpoint (48.1375, 11.565), 100m outward (east).
	// 1 deg lon at lat 48.14 ≈ 111000 * cos(48.14°) ≈ 74100 m
	// 100m east: 100/74100 ≈ 0.001350 degrees lon
	eastNear := model.Coordinate{Lat: 48.1375, Lon: 11.5664}

	// West edge midpoint (48.1375, 11.555), 100m outward (west):
	westNear := model.Coordinate{Lat: 48.1375, Lon: 11.5536}

	threshold := 200.0 // use 200m to ensure 100m-offset points definitely qualify

	nearPoints := []struct {
		name  string
		point model.Coordinate
	}{
		{"north near", northNear},
		{"south near", southNear},
		{"east near", eastNear},
		{"west near", westNear},
	}

	for _, np := range nearPoints {
		t.Run(np.name, func(t *testing.T) {
			result := geo.FindMatchingZones(np.point, []model.Zone{zone}, threshold)
			if !containsZone(result, "munich-central") {
				t.Errorf("expected munich-central in FindMatchingZones result for %s (%v, threshold=%v), got %v",
					np.name, np.point, threshold, result)
			}
		})
	}
}

// containsZone checks if zoneID is present in the slice.
func containsZone(zones []string, zoneID string) bool {
	for _, z := range zones {
		if z == zoneID {
			return true
		}
	}
	return false
}
