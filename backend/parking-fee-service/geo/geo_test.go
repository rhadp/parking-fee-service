package geo_test

import (
	"testing"

	"parking-fee-service/backend/parking-fee-service/geo"
	"parking-fee-service/backend/parking-fee-service/model"
)

// Munich-central test polygon: a rectangle roughly around Munich's central
// station area.
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

// TestPointInPolygonInside verifies that a coordinate inside the polygon
// returns true (TS-05-2).
func TestPointInPolygonInside(t *testing.T) {
	// (48.1375, 11.5600) is the centre of the munich-central rectangle
	inside := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	if !geo.PointInPolygon(inside, munichCentralPolygon) {
		t.Errorf("expected point inside polygon, got false")
	}
}

// TestPointInPolygonOutside verifies that a coordinate outside the polygon
// returns false (TS-05-2).
func TestPointInPolygonOutside(t *testing.T) {
	outside := model.Coordinate{Lat: 48.1500, Lon: 11.5800}
	if geo.PointInPolygon(outside, munichCentralPolygon) {
		t.Errorf("expected point outside polygon, got true")
	}
}

// TestProximityMatchingWithinThreshold verifies that a point just outside the
// polygon but within 500m is included in FindMatchingZones (TS-05-3).
func TestProximityMatchingWithinThreshold(t *testing.T) {
	// ~100 m north of the polygon top edge (Lat 48.140)
	nearPoint := model.Coordinate{Lat: 48.1410, Lon: 11.5600}
	zones := []model.Zone{munichCentralZone}
	result := geo.FindMatchingZones(nearPoint, zones, 500.0)
	found := false
	for _, id := range result {
		if id == "munich-central" {
			found = true
		}
	}
	if !found {
		t.Errorf("expected munich-central in result, got %v", result)
	}
}

// TestProximityThresholdUsed verifies that the configured threshold is
// respected: a point 50 m outside matches with threshold 100, but a point
// 200 m outside does not (TS-05-11).
func TestProximityThresholdUsed(t *testing.T) {
	// Build a simple 1°-square polygon centred at (0, 0) for easy maths:
	// use the munich-central zone polygon for a real-world test instead.

	// ~50 m north of the top edge (Lat 48.140).  1 degree lat ≈ 111,000 m.
	// 50 m ≈ 0.00045 degrees.
	point50m := model.Coordinate{Lat: 48.1405, Lon: 11.5600}
	// ~200 m north of the top edge
	point200m := model.Coordinate{Lat: 48.1418, Lon: 11.5600}

	zones := []model.Zone{munichCentralZone}

	near := geo.FindMatchingZones(point50m, zones, 100.0)
	if len(near) == 0 {
		t.Errorf("expected munich-central to match with threshold 100 m, got empty")
	}

	far := geo.FindMatchingZones(point200m, zones, 100.0)
	if len(far) != 0 {
		t.Errorf("expected no match with threshold 100 m at 200 m distance, got %v", far)
	}
}

// TestHaversineDistanceReasonable verifies that HaversineDistance returns a
// plausible value (Munich to Marienplatz ~1 km).
func TestHaversineDistanceReasonable(t *testing.T) {
	// Rough distance between two Munich landmarks
	a := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	b := model.Coordinate{Lat: 48.1365, Lon: 11.5760}
	d := geo.HaversineDistance(a, b)
	// Expect something between 500 m and 2000 m
	if d < 500 || d > 2000 {
		t.Errorf("HaversineDistance: unexpected value %f (want 500–2000)", d)
	}
}

// TestDistanceToPolygonEdgeZeroInsideOrOnEdge verifies that a point at a
// corner of the polygon has a distance ≤ a small epsilon.
func TestDistanceToPolygonEdgeZeroInsideOrOnEdge(t *testing.T) {
	// A point on the top-left corner of the polygon
	corner := model.Coordinate{Lat: 48.1400, Lon: 11.5550}
	d := geo.DistanceToPolygonEdge(corner, munichCentralPolygon)
	// Should be very close to 0
	if d > 1.0 {
		t.Errorf("DistanceToPolygonEdge for corner point: expected ~0, got %f", d)
	}
}

// TestFindMatchingZonesEmpty verifies that a point far from all zones returns
// an empty slice (TS-05-5).
func TestFindMatchingZonesEmpty(t *testing.T) {
	far := model.Coordinate{Lat: 0.0, Lon: 0.0}
	zones := []model.Zone{munichCentralZone}
	result := geo.FindMatchingZones(far, zones, 500.0)
	if len(result) != 0 {
		t.Errorf("expected empty result, got %v", result)
	}
}
