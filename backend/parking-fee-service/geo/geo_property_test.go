package geo_test

import (
	"math"
	"testing"

	"parking-fee-service/backend/parking-fee-service/geo"
	"parking-fee-service/backend/parking-fee-service/model"
)

// ---- TS-05-P1: Point-in-Polygon Correctness ----

// TestPropertyPointInPolygon uses a table-driven approach to verify:
// - Points geometrically inside a convex polygon return true.
// - Points far outside do not match with FindMatchingZones.
func TestPropertyPointInPolygon(t *testing.T) {
	// Use the munich-central polygon defined in geo_test.go
	polygon := munichCentralPolygon
	zone := munichCentralZone

	// Points definitely inside the rectangle
	insidePoints := []model.Coordinate{
		{Lat: 48.1375, Lon: 11.5600}, // centre
		{Lat: 48.1360, Lon: 11.5560}, // near bottom-left
		{Lat: 48.1395, Lon: 11.5640}, // near top-right
	}
	for _, p := range insidePoints {
		if !geo.PointInPolygon(p, polygon) {
			t.Errorf("expected %+v inside polygon", p)
		}
	}

	// Points clearly outside (>500 m from any edge)
	outsidePoints := []model.Coordinate{
		{Lat: 48.0000, Lon: 11.5600}, // far south
		{Lat: 48.2000, Lon: 11.5600}, // far north
		{Lat: 48.1375, Lon: 11.0000}, // far west
		{Lat: 48.1375, Lon: 12.0000}, // far east
	}
	threshold := 500.0
	for _, p := range outsidePoints {
		if geo.PointInPolygon(p, polygon) {
			t.Errorf("expected %+v outside polygon", p)
		}
		result := geo.FindMatchingZones(p, []model.Zone{zone}, threshold)
		if len(result) != 0 {
			t.Errorf("expected no match for far-outside point %+v, got %v", p, result)
		}
	}
}

// ---- TS-05-P2: Proximity Matching ----

// TestPropertyProximityMatching verifies that a point within threshold of the
// polygon edge is included in FindMatchingZones results.
func TestPropertyProximityMatching(t *testing.T) {
	zone := munichCentralZone
	threshold := 500.0

	// Points 100–400 m outside the top edge (Lat 48.140).
	// 1 degree lat ≈ 111,000 m; so 400 m ≈ 0.0036 degrees.
	nearPoints := []model.Coordinate{
		{Lat: 48.1410, Lon: 11.5600}, // ~111 m north
		{Lat: 48.1420, Lon: 11.5600}, // ~222 m north
		{Lat: 48.1430, Lon: 11.5600}, // ~333 m north
		{Lat: 48.1440, Lon: 11.5600}, // ~444 m north — inside threshold
	}

	for _, p := range nearPoints {
		result := geo.FindMatchingZones(p, []model.Zone{zone}, threshold)
		found := false
		for _, id := range result {
			if id == zone.ID {
				found = true
			}
		}
		if !found {
			t.Errorf("expected %+v within %f m threshold to match zone, got %v", p, threshold, result)
		}
	}
}

// ---- Haversine sanity (distance accuracy) ----

// TestHaversineZeroDistance verifies that a point's distance to itself is 0.
func TestHaversineZeroDistance(t *testing.T) {
	p := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	d := geo.HaversineDistance(p, p)
	if d > 1e-6 {
		t.Errorf("distance to self: want 0, got %f", d)
	}
}

// TestHaversineSymmetry verifies that distance(a, b) == distance(b, a).
func TestHaversineSymmetry(t *testing.T) {
	a := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	b := model.Coordinate{Lat: 48.2000, Lon: 11.6000}
	dAB := geo.HaversineDistance(a, b)
	dBA := geo.HaversineDistance(b, a)
	if math.Abs(dAB-dBA) > 1e-9 {
		t.Errorf("asymmetric distance: d(a,b)=%f d(b,a)=%f", dAB, dBA)
	}
}

// TestHaversineKnownDistance verifies the distance against a known reference.
// Equatorial 1-degree longitude ≈ 111.32 km.
func TestHaversineKnownDistance(t *testing.T) {
	a := model.Coordinate{Lat: 0.0, Lon: 0.0}
	b := model.Coordinate{Lat: 0.0, Lon: 1.0}
	d := geo.HaversineDistance(a, b)
	// Expect 111,000 – 111,700 m
	if d < 111000 || d > 111700 {
		t.Errorf("Haversine 1° longitude at equator: want ~111320, got %f", d)
	}
}
