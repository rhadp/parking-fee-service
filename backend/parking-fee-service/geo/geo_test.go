package geo_test

import (
	"testing"

	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/model"
)

// munichCentralPolygon is the munich-central demo zone used across multiple tests.
// It is a rectangle: lat [48.135, 48.140], lon [11.555, 11.565].
var munichCentralPolygon = []model.Coordinate{
	{Lat: 48.140, Lon: 11.555},
	{Lat: 48.140, Lon: 11.565},
	{Lat: 48.135, Lon: 11.565},
	{Lat: 48.135, Lon: 11.555},
}

// munichCentralZone wraps the polygon as a Zone for FindMatchingZones calls.
var munichCentralZone = model.Zone{
	ID:      "munich-central",
	Name:    "Munich Central Station Area",
	Polygon: munichCentralPolygon,
}

// TS-05-2: PointInPolygon correctly identifies a coordinate inside the polygon.
func TestPointInPolygonInside(t *testing.T) {
	t.Helper()
	point := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	if !geo.PointInPolygon(point, munichCentralPolygon) {
		t.Errorf("PointInPolygon(%v, polygon): want true (point is inside), got false", point)
	}
}

// TS-05-2: PointInPolygon correctly identifies a coordinate outside the polygon.
func TestPointInPolygonOutside(t *testing.T) {
	t.Helper()
	point := model.Coordinate{Lat: 48.1500, Lon: 11.5800}
	if geo.PointInPolygon(point, munichCentralPolygon) {
		t.Errorf("PointInPolygon(%v, polygon): want false (point is outside), got true", point)
	}
}

// TS-05-2: Table-driven tests for various inside/outside positions.
func TestPointInPolygon(t *testing.T) {
	t.Helper()
	tests := []struct {
		name   string
		point  model.Coordinate
		inside bool
	}{
		{"center of rect", model.Coordinate{Lat: 48.1375, Lon: 11.5600}, true},
		{"near top edge inside", model.Coordinate{Lat: 48.1399, Lon: 11.5600}, true},
		{"near bottom edge inside", model.Coordinate{Lat: 48.1351, Lon: 11.5600}, true},
		{"near left edge inside", model.Coordinate{Lat: 48.1375, Lon: 11.5551}, true},
		{"near right edge inside", model.Coordinate{Lat: 48.1375, Lon: 11.5649}, true},
		{"far above", model.Coordinate{Lat: 48.1500, Lon: 11.5600}, false},
		{"far below", model.Coordinate{Lat: 48.1200, Lon: 11.5600}, false},
		{"far left", model.Coordinate{Lat: 48.1375, Lon: 11.5400}, false},
		{"far right", model.Coordinate{Lat: 48.1375, Lon: 11.5800}, false},
		{"far away marienplatz area", model.Coordinate{Lat: 48.1365, Lon: 11.5760}, false},
		{"origin (0,0)", model.Coordinate{Lat: 0.0, Lon: 0.0}, false},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := geo.PointInPolygon(tc.point, munichCentralPolygon)
			if got != tc.inside {
				t.Errorf("PointInPolygon(%v): want %v, got %v", tc.point, tc.inside, got)
			}
		})
	}
}

// TS-05-3: FindMatchingZones includes zones where point is outside polygon but
// within proximity threshold.
func TestProximityMatchingWithinThreshold(t *testing.T) {
	t.Helper()
	// Point ~100m outside the north edge of munich-central (lat 48.140).
	// 0.001 degree latitude ≈ 111 m, so 48.1409 is about 100m north of the polygon edge,
	// well within 500m threshold.
	nearPoint := model.Coordinate{Lat: 48.1409, Lon: 11.5600}
	zones := []model.Zone{munichCentralZone}
	result := geo.FindMatchingZones(nearPoint, zones, 500.0)

	found := false
	for _, id := range result {
		if id == "munich-central" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("FindMatchingZones with point ~100m outside polygon and threshold=500m: "+
			"expected 'munich-central' in result %v, but it was not found", result)
	}
}

// TS-05-11: FindMatchingZones uses the configured proximity threshold.
// A point ~55m outside matches with 100m threshold; a point ~222m outside does not.
func TestProximityThresholdUsed(t *testing.T) {
	t.Helper()
	zones := []model.Zone{munichCentralZone}

	// 0.0005 degrees lat ≈ 55m north of the polygon's north edge (48.140).
	// This point should be inside the 100m threshold.
	near := model.Coordinate{Lat: 48.1405, Lon: 11.5600}

	// 0.002 degrees lat ≈ 222m north of the polygon's north edge.
	// This point should be outside the 100m threshold.
	far := model.Coordinate{Lat: 48.1420, Lon: 11.5600}

	nearResult := geo.FindMatchingZones(near, zones, 100.0)
	nearFound := false
	for _, id := range nearResult {
		if id == "munich-central" {
			nearFound = true
			break
		}
	}
	if !nearFound {
		t.Errorf("FindMatchingZones with point ~55m outside and threshold=100m: "+
			"expected 'munich-central' in result %v", nearResult)
	}

	farResult := geo.FindMatchingZones(far, zones, 100.0)
	for _, id := range farResult {
		if id == "munich-central" {
			t.Errorf("FindMatchingZones with point ~222m outside and threshold=100m: "+
				"unexpected 'munich-central' in result %v", farResult)
		}
	}
}

// TS-05-P1: Property — for any point geometrically inside a convex polygon,
// PointInPolygon returns true; for any point far outside all zones,
// FindMatchingZones returns empty.
//
// Note: We use table-driven tests with pre-verified points as the correctness
// oracle. Using a winding-number reference avoids the "barycentric coordinates"
// approach from the spec pseudocode, which only works for triangles — not
// general polygons (see docs/errata/).
func TestPropertyPointInPolygon(t *testing.T) {
	t.Helper()
	type testCase struct {
		name    string
		polygon []model.Coordinate
		inside  []model.Coordinate
		outside []model.Coordinate
	}

	cases := []testCase{
		{
			name: "unit square (lat/lon axis-aligned)",
			polygon: []model.Coordinate{
				{Lat: 0, Lon: 0}, {Lat: 0, Lon: 1},
				{Lat: 1, Lon: 1}, {Lat: 1, Lon: 0},
			},
			inside: []model.Coordinate{
				{Lat: 0.5, Lon: 0.5},
				{Lat: 0.1, Lon: 0.1},
				{Lat: 0.9, Lon: 0.9},
			},
			outside: []model.Coordinate{
				{Lat: -0.5, Lon: 0.5},
				{Lat: 1.5, Lon: 0.5},
				{Lat: 0.5, Lon: -0.5},
				{Lat: 0.5, Lon: 1.5},
			},
		},
		{
			name: "munich-central rectangle",
			polygon: []model.Coordinate{
				{Lat: 48.140, Lon: 11.555}, {Lat: 48.140, Lon: 11.565},
				{Lat: 48.135, Lon: 11.565}, {Lat: 48.135, Lon: 11.555},
			},
			inside: []model.Coordinate{
				{Lat: 48.1375, Lon: 11.560},
				{Lat: 48.1360, Lon: 11.558},
				{Lat: 48.1395, Lon: 11.562},
			},
			outside: []model.Coordinate{
				{Lat: 48.141, Lon: 11.560},
				{Lat: 48.134, Lon: 11.560},
				{Lat: 48.1375, Lon: 11.554},
				{Lat: 48.1375, Lon: 11.566},
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			for _, pt := range tc.inside {
				if !geo.PointInPolygon(pt, tc.polygon) {
					t.Errorf("PointInPolygon(%v, %s polygon): want true (inside), got false",
						pt, tc.name)
				}
			}
			for _, pt := range tc.outside {
				if geo.PointInPolygon(pt, tc.polygon) {
					t.Errorf("PointInPolygon(%v, %s polygon): want false (outside), got true",
						pt, tc.name)
				}
				// Points far outside should also not match FindMatchingZones with a tiny threshold.
				zone := model.Zone{ID: "test", Polygon: tc.polygon}
				result := geo.FindMatchingZones(pt, []model.Zone{zone}, 1.0)
				if len(result) != 0 {
					t.Errorf("FindMatchingZones(%v, threshold=1m): want empty (far outside), got %v",
						pt, result)
				}
			}
		})
	}
}

// TS-05-P2: Property — for any coordinate within threshold metres of a zone's
// edge, FindMatchingZones includes that zone.
func TestPropertyProximityMatching(t *testing.T) {
	t.Helper()
	type testCase struct {
		name      string
		zone      model.Zone
		nearPoint model.Coordinate
		threshold float64
	}

	cases := []testCase{
		{
			name:      "100m north of munich-central",
			zone:      munichCentralZone,
			nearPoint: model.Coordinate{Lat: 48.1409, Lon: 11.5600}, // ~100m north of edge
			threshold: 500.0,
		},
		{
			name:      "50m east of munich-central with 200m threshold",
			zone:      munichCentralZone,
			nearPoint: model.Coordinate{Lat: 48.1375, Lon: 11.5655}, // ~50m east of edge
			threshold: 200.0,
		},
		{
			name:      "200m south of munich-central with 300m threshold",
			zone:      munichCentralZone,
			nearPoint: model.Coordinate{Lat: 48.1332, Lon: 11.5600}, // ~200m south of edge
			threshold: 300.0,
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			result := geo.FindMatchingZones(tc.nearPoint, []model.Zone{tc.zone}, tc.threshold)
			found := false
			for _, id := range result {
				if id == tc.zone.ID {
					found = true
					break
				}
			}
			if !found {
				t.Errorf("FindMatchingZones near %s: expected zone %q in result %v",
					tc.name, tc.zone.ID, result)
			}
		})
	}
}
