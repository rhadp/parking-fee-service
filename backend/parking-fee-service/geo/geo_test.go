package geo

import (
	"math"
	"testing"
)

// ---------- Haversine Distance Tests ----------

// TestHaversine_MunichToBerlin verifies Haversine with a well-known city pair.
// Munich (48.1351, 11.5820) to Berlin (52.5200, 13.4050) ≈ 504 km.
func TestHaversine_MunichToBerlin(t *testing.T) {
	dist := HaversineDistance(48.1351, 11.5820, 52.5200, 13.4050)
	expected := 504_000.0 // approximately 504 km
	tolerance := 5_000.0  // within 5 km

	if math.Abs(dist-expected) > tolerance {
		t.Errorf("Munich to Berlin: got %.0f m, want ≈ %.0f m (±%.0f m)",
			dist, expected, tolerance)
	}
}

// TestHaversine_SamePoint verifies that distance to itself is zero.
func TestHaversine_SamePoint(t *testing.T) {
	dist := HaversineDistance(48.1351, 11.5820, 48.1351, 11.5820)
	if dist != 0 {
		t.Errorf("same point distance: got %f m, want 0", dist)
	}
}

// TestHaversine_Symmetry verifies Property 6: HaversineDistance(A,B) == HaversineDistance(B,A).
func TestHaversine_Symmetry(t *testing.T) {
	testCases := []struct {
		name             string
		lat1, lon1       float64
		lat2, lon2       float64
	}{
		{"Munich-Berlin", 48.1351, 11.5820, 52.5200, 13.4050},
		{"London-Paris", 51.5074, -0.1278, 48.8566, 2.3522},
		{"NewYork-LosAngeles", 40.7128, -74.0060, 34.0522, -118.2437},
		{"NorthPole-SouthPole", 90.0, 0.0, -90.0, 0.0},
		{"Equator-NorthPole", 0.0, 0.0, 90.0, 0.0},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			d1 := HaversineDistance(tc.lat1, tc.lon1, tc.lat2, tc.lon2)
			d2 := HaversineDistance(tc.lat2, tc.lon2, tc.lat1, tc.lon1)
			tolerance := 0.001 // 1 mm floating-point tolerance

			if math.Abs(d1-d2) > tolerance {
				t.Errorf("symmetry violation: d(A,B)=%.6f, d(B,A)=%.6f, diff=%.6f",
					d1, d2, math.Abs(d1-d2))
			}
		})
	}
}

// TestHaversine_ShortDistance verifies accuracy for a short distance (~111 m).
// 0.001 degrees of latitude ≈ 111.2 m.
func TestHaversine_ShortDistance(t *testing.T) {
	dist := HaversineDistance(48.1350, 11.5820, 48.1360, 11.5820)
	expected := 111.2
	tolerance := 2.0 // within 2 m

	if math.Abs(dist-expected) > tolerance {
		t.Errorf("short distance: got %.1f m, want ≈ %.1f m (±%.0f m)",
			dist, expected, tolerance)
	}
}

// TestHaversine_AntipodalPoints verifies maximum distance (half circumference).
func TestHaversine_AntipodalPoints(t *testing.T) {
	dist := HaversineDistance(0, 0, 0, 180)
	expected := math.Pi * earthRadius
	tolerance := 1.0

	if math.Abs(dist-expected) > tolerance {
		t.Errorf("antipodal distance: got %.0f m, want ≈ %.0f m", dist, expected)
	}
}

// ---------- Point-in-Polygon Tests ----------

// A simple rectangle around Marienplatz, Munich.
var marienplatzRect = []LatLon{
	{48.1380, 11.5730},
	{48.1380, 11.5780},
	{48.1355, 11.5780},
	{48.1355, 11.5730},
}

// TestPointInPolygon_InsideRectangle verifies Property 1: a point clearly
// inside a polygon is detected as inside.
func TestPointInPolygon_InsideRectangle(t *testing.T) {
	// Center of the Marienplatz rectangle.
	inside := PointInPolygon(48.13675, 11.5755, marienplatzRect)
	if !inside {
		t.Error("center of rectangle: got false, want true")
	}
}

// TestPointInPolygon_OutsideRectangle verifies a point clearly outside
// a polygon is detected as outside.
func TestPointInPolygon_OutsideRectangle(t *testing.T) {
	// Point well outside the rectangle.
	inside := PointInPolygon(48.15, 11.60, marienplatzRect)
	if inside {
		t.Error("point outside rectangle: got true, want false")
	}
}

// TestPointInPolygon_MultipleInOutCases tests several points inside and outside.
func TestPointInPolygon_MultipleInOutCases(t *testing.T) {
	testCases := []struct {
		name   string
		lat    float64
		lon    float64
		expect bool
	}{
		{"inside center", 48.13675, 11.5755, true},
		{"inside near top", 48.1379, 11.5755, true},
		{"inside near bottom", 48.1356, 11.5755, true},
		{"inside near left", 48.13675, 11.5731, true},
		{"inside near right", 48.13675, 11.5779, true},
		{"outside north", 48.1400, 11.5755, false},
		{"outside south", 48.1340, 11.5755, false},
		{"outside east", 48.13675, 11.5800, false},
		{"outside west", 48.13675, 11.5710, false},
		{"far away", 52.5200, 13.4050, false}, // Berlin
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			result := PointInPolygon(tc.lat, tc.lon, marienplatzRect)
			if result != tc.expect {
				t.Errorf("PointInPolygon(%f, %f): got %v, want %v",
					tc.lat, tc.lon, result, tc.expect)
			}
		})
	}
}

// TestPointInPolygon_Triangle tests with a non-rectangular polygon.
func TestPointInPolygon_Triangle(t *testing.T) {
	triangle := []LatLon{
		{48.0, 11.0},
		{48.0, 12.0},
		{49.0, 11.5},
	}

	testCases := []struct {
		name   string
		lat    float64
		lon    float64
		expect bool
	}{
		{"inside centroid", 48.33, 11.5, true},
		{"inside near base", 48.05, 11.5, true},
		{"outside above", 49.5, 11.5, false},
		{"outside left", 48.5, 10.5, false},
		{"outside right", 48.5, 12.5, false},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			result := PointInPolygon(tc.lat, tc.lon, triangle)
			if result != tc.expect {
				t.Errorf("PointInPolygon(%f, %f) in triangle: got %v, want %v",
					tc.lat, tc.lon, result, tc.expect)
			}
		})
	}
}

// TestPointInPolygon_ConcavePolygon tests with an L-shaped polygon.
func TestPointInPolygon_ConcavePolygon(t *testing.T) {
	// L-shaped polygon (concave):
	//   (0,0)---(0,2)
	//     |       |
	//   (1,0)---(1,1)
	//             |
	//   (2,1)---(2,2) -- wait, let me draw properly
	// Using lat as Y, lon as X for clarity:
	lShape := []LatLon{
		{0, 0}, // bottom-left
		{2, 0}, // top-left
		{2, 1}, // top-middle
		{1, 1}, // inner corner
		{1, 2}, // bottom-right top
		{0, 2}, // bottom-right
	}

	testCases := []struct {
		name   string
		lat    float64
		lon    float64
		expect bool
	}{
		{"inside left arm", 1.5, 0.5, true},
		{"inside bottom arm", 0.5, 1.5, true},
		{"outside concavity", 1.5, 1.5, false},
		{"outside entirely", 3.0, 3.0, false},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			result := PointInPolygon(tc.lat, tc.lon, lShape)
			if result != tc.expect {
				t.Errorf("PointInPolygon(%f, %f) in L-shape: got %v, want %v",
					tc.lat, tc.lon, result, tc.expect)
			}
		})
	}
}

// TestPointInPolygon_DegeneratePolygons tests edge cases with too-few points.
func TestPointInPolygon_DegeneratePolygons(t *testing.T) {
	// Empty polygon.
	if PointInPolygon(0, 0, nil) {
		t.Error("nil polygon: got true, want false")
	}

	// Single point.
	if PointInPolygon(0, 0, []LatLon{{0, 0}}) {
		t.Error("single point polygon: got true, want false")
	}

	// Two points (line segment).
	if PointInPolygon(0, 0, []LatLon{{0, 0}, {1, 1}}) {
		t.Error("two-point polygon: got true, want false")
	}
}

// ---------- Distance-to-Polygon Tests ----------

// TestDistanceToPolygon_PointNearEdge tests distance from a point near a polygon edge.
func TestDistanceToPolygon_PointNearEdge(t *testing.T) {
	// Point directly north of the Marienplatz rectangle (above the top edge).
	// Top edge is at lat 48.1380, so a point at lat 48.1390 is ~111 m north.
	dist := DistanceToPolygon(48.1390, 11.5755, marienplatzRect)
	expected := 111.2
	tolerance := 5.0

	if math.Abs(dist-expected) > tolerance {
		t.Errorf("point near north edge: got %.1f m, want ≈ %.1f m (±%.0f m)",
			dist, expected, tolerance)
	}
}

// TestDistanceToPolygon_PointFarAway tests distance from a distant point.
func TestDistanceToPolygon_PointFarAway(t *testing.T) {
	// Berlin is about 504 km from Munich.
	dist := DistanceToPolygon(52.5200, 13.4050, marienplatzRect)

	if dist < 400_000 {
		t.Errorf("distance from Berlin to Marienplatz polygon: got %.0f m, want > 400,000 m", dist)
	}
}

// TestDistanceToPolygon_PointOnVertex tests distance from a vertex of the polygon.
func TestDistanceToPolygon_PointOnVertex(t *testing.T) {
	// Distance from a vertex should be approximately 0.
	dist := DistanceToPolygon(48.1380, 11.5730, marienplatzRect)
	if dist > 1.0 { // within 1 meter
		t.Errorf("point on vertex: got %.1f m, want ≈ 0 m", dist)
	}
}

// TestDistanceToPolygon_PointInsidePolygon tests that distance from a point
// inside the polygon to its edges is positive (not zero).
func TestDistanceToPolygon_PointInsidePolygon(t *testing.T) {
	// Center of the Marienplatz rectangle.
	dist := DistanceToPolygon(48.13675, 11.5755, marienplatzRect)
	// The distance should be the distance to the nearest edge (approx half
	// the shorter side dimension).
	if dist <= 0 {
		t.Errorf("point inside polygon: got %.1f m, want > 0", dist)
	}
}

// TestDistanceToPolygon_DirectlyEastOfEdge tests distance from a point that
// projects onto the middle of an edge.
func TestDistanceToPolygon_DirectlyEastOfEdge(t *testing.T) {
	// Point directly east of the right edge (lon=11.5780).
	// The right edge runs from (48.1380, 11.5780) to (48.1355, 11.5780).
	// A point at (48.13675, 11.5790) is about 0.001 deg east.
	dist := DistanceToPolygon(48.13675, 11.5790, marienplatzRect)

	// 0.001 degrees longitude at lat ≈ 48.137 → ~74 m
	// (cos(48.137°) * 111,320 * 0.001 ≈ 74.3 m)
	expected := 74.3
	tolerance := 5.0

	if math.Abs(dist-expected) > tolerance {
		t.Errorf("point east of right edge: got %.1f m, want ≈ %.1f m (±%.0f m)",
			dist, expected, tolerance)
	}
}

// TestDistanceToPolygon_EmptyPolygon tests edge case with no points.
func TestDistanceToPolygon_EmptyPolygon(t *testing.T) {
	dist := DistanceToPolygon(48.0, 11.0, nil)
	if !math.IsInf(dist, 1) {
		t.Errorf("empty polygon distance: got %f, want +Inf", dist)
	}
}

// TestDistanceToPolygon_SinglePoint tests edge case with a single-point polygon.
func TestDistanceToPolygon_SinglePoint(t *testing.T) {
	point := []LatLon{{48.1380, 11.5730}}
	dist := DistanceToPolygon(48.1380, 11.5730, point)
	if dist > 1.0 {
		t.Errorf("single-point polygon at same location: got %.1f m, want ≈ 0 m", dist)
	}
}

// TestDistanceToPolygon_KnownOffsets tests with precise known offsets from
// different edges of the Marienplatz rectangle to validate accuracy.
func TestDistanceToPolygon_KnownOffsets(t *testing.T) {
	testCases := []struct {
		name      string
		lat, lon  float64
		maxDist   float64 // upper bound on expected distance in meters
		minDist   float64 // lower bound on expected distance in meters
	}{
		{
			name:    "100m north",
			lat:     48.1389, // ~0.0009 deg ≈ 100m north of top edge
			lon:     11.5755,
			minDist: 80,
			maxDist: 120,
		},
		{
			name:    "200m south",
			lat:     48.1337, // ~0.0018 deg ≈ 200m south of bottom edge
			lon:     11.5755,
			minDist: 180,
			maxDist: 220,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			dist := DistanceToPolygon(tc.lat, tc.lon, marienplatzRect)
			if dist < tc.minDist || dist > tc.maxDist {
				t.Errorf("distance: got %.1f m, want between %.0f and %.0f m",
					dist, tc.minDist, tc.maxDist)
			}
		})
	}
}

// ---------- distanceToSegment Tests ----------

// TestDistanceToSegment_DegenerateSegment tests the case where both endpoints
// of the segment are the same point.
func TestDistanceToSegment_DegenerateSegment(t *testing.T) {
	a := LatLon{48.1380, 11.5730}
	b := LatLon{48.1380, 11.5730}
	dist := distanceToSegment(48.1390, 11.5730, a, b)
	expected := HaversineDistance(48.1390, 11.5730, 48.1380, 11.5730)

	if math.Abs(dist-expected) > 0.01 {
		t.Errorf("degenerate segment: got %.1f m, want %.1f m", dist, expected)
	}
}

// TestDistanceToSegment_ProjectionBeyondEndpoint verifies that when the
// closest point on the infinite line is beyond the segment endpoint,
// the function returns distance to the endpoint (clamping).
func TestDistanceToSegment_ProjectionBeyondEndpoint(t *testing.T) {
	// Horizontal segment from (48.0, 11.0) to (48.0, 12.0).
	a := LatLon{48.0, 11.0}
	b := LatLon{48.0, 12.0}

	// Point at (48.0, 10.5): projection would be at lon 10.5, which is
	// before the segment start at lon 11.0. So distance should be to
	// point a = (48.0, 11.0).
	dist := distanceToSegment(48.0, 10.5, a, b)
	expected := HaversineDistance(48.0, 10.5, 48.0, 11.0)

	if math.Abs(dist-expected) > 1.0 {
		t.Errorf("projection before start: got %.1f m, want %.1f m", dist, expected)
	}
}
