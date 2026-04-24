package geo

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// Munich central zone polygon (from design doc / default config).
var munichCentralPolygon = []model.Coordinate{
	{Lat: 48.1400, Lon: 11.5550},
	{Lat: 48.1400, Lon: 11.5650},
	{Lat: 48.1350, Lon: 11.5650},
	{Lat: 48.1350, Lon: 11.5550},
}

// munichCentralZone wraps the polygon in a Zone for FindMatchingZones tests.
var munichCentralZone = model.Zone{
	ID:      "munich-central",
	Name:    "Munich Central Station Area",
	Polygon: munichCentralPolygon,
}

// ---------------------------------------------------------------------------
// TS-05-2: Point-in-Polygon Ray Casting
// ---------------------------------------------------------------------------

// TestPointInPolygonInside verifies that a coordinate inside a polygon
// returns true.
func TestPointInPolygonInside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	if !PointInPolygon(point, munichCentralPolygon) {
		t.Errorf("PointInPolygon(%v, polygon) = false, want true", point)
	}
}

// TestPointInPolygonOutside verifies that a coordinate outside a polygon
// returns false.
func TestPointInPolygonOutside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1500, Lon: 11.5800}
	if PointInPolygon(point, munichCentralPolygon) {
		t.Errorf("PointInPolygon(%v, polygon) = true, want false", point)
	}
}

// ---------------------------------------------------------------------------
// TS-05-3: Proximity Matching Within Threshold
// ---------------------------------------------------------------------------

// TestProximityMatchingWithinThreshold verifies that FindMatchingZones
// includes zones where the point is outside the polygon but within the
// proximity threshold.
func TestProximityMatchingWithinThreshold(t *testing.T) {
	zones := []model.Zone{munichCentralZone}

	// Point slightly north of the polygon (~55m outside the north edge at
	// lat 48.1400). 1 degree lat ≈ 111,320m, so 0.0005° ≈ 55m.
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
		t.Errorf("FindMatchingZones(%v, zones, 500.0) = %v, want to contain \"munich-central\"",
			nearPoint, result)
	}
}

// ---------------------------------------------------------------------------
// TS-05-11: Proximity Threshold Used
// ---------------------------------------------------------------------------

// TestProximityThresholdUsed verifies that the configured proximity
// threshold determines near-zone matching.
func TestProximityThresholdUsed(t *testing.T) {
	zones := []model.Zone{munichCentralZone}

	// Point ~50m outside the north edge (lat 48.1400).
	// 50m / 111,320m ≈ 0.000449°
	point50m := model.Coordinate{Lat: 48.14045, Lon: 11.5600}

	// With 100m threshold, a point 50m outside should match.
	near := FindMatchingZones(point50m, zones, 100.0)
	if len(near) != 1 {
		t.Errorf("FindMatchingZones(50m outside, zones, 100.0) returned %d zones, want 1", len(near))
	}

	// Point ~200m outside the north edge.
	// 200m / 111,320m ≈ 0.001797°
	point200m := model.Coordinate{Lat: 48.1418, Lon: 11.5600}

	// With 100m threshold, a point 200m outside should NOT match.
	far := FindMatchingZones(point200m, zones, 100.0)
	if len(far) != 0 {
		t.Errorf("FindMatchingZones(200m outside, zones, 100.0) returned %d zones, want 0", len(far))
	}
}

// ---------------------------------------------------------------------------
// TS-05-P1: Property – Point-in-Polygon Correctness
// ---------------------------------------------------------------------------

// TestPropertyPointInPolygon uses table-driven tests with known geometries
// to validate point-in-polygon correctness, and verifies that points far
// from all zones produce empty FindMatchingZones results.
func TestPropertyPointInPolygon(t *testing.T) {
	tests := []struct {
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
			name:  "clearly outside unit square",
			point: model.Coordinate{Lat: 2.0, Lon: 2.0},
			polygon: []model.Coordinate{
				{Lat: 0, Lon: 0}, {Lat: 0, Lon: 1},
				{Lat: 1, Lon: 1}, {Lat: 1, Lon: 0},
			},
			inside: false,
		},
		{
			name:  "inside triangle",
			point: model.Coordinate{Lat: 0.25, Lon: 0.25},
			polygon: []model.Coordinate{
				{Lat: 0, Lon: 0}, {Lat: 0, Lon: 1}, {Lat: 1, Lon: 0},
			},
			inside: true,
		},
		{
			name:  "outside triangle",
			point: model.Coordinate{Lat: 0.8, Lon: 0.8},
			polygon: []model.Coordinate{
				{Lat: 0, Lon: 0}, {Lat: 0, Lon: 1}, {Lat: 1, Lon: 0},
			},
			inside: false,
		},
		{
			name:    "inside munich-central polygon",
			point:   model.Coordinate{Lat: 48.1375, Lon: 11.5600},
			polygon: munichCentralPolygon,
			inside:  true,
		},
		{
			name:    "far outside munich-central polygon",
			point:   model.Coordinate{Lat: 49.0, Lon: 12.0},
			polygon: munichCentralPolygon,
			inside:  false,
		},
		{
			name:  "near corner but outside",
			point: model.Coordinate{Lat: 48.1401, Lon: 11.5549},
			polygon: munichCentralPolygon,
			inside: false,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := PointInPolygon(tt.point, tt.polygon)
			if got != tt.inside {
				t.Errorf("PointInPolygon(%v, polygon) = %v, want %v",
					tt.point, got, tt.inside)
			}
		})
	}

	// Property: points far outside all zones produce empty results.
	farPoints := []model.Coordinate{
		{Lat: 0, Lon: 0},
		{Lat: -45, Lon: 90},
		{Lat: 60, Lon: -30},
		{Lat: -89, Lon: 179},
	}
	zones := []model.Zone{munichCentralZone}
	for _, p := range farPoints {
		result := FindMatchingZones(p, zones, 500.0)
		if len(result) != 0 {
			t.Errorf("FindMatchingZones(%v, zones, 500.0) = %v, want empty (point far outside)",
				p, result)
		}
	}
}

// ---------------------------------------------------------------------------
// TS-05-P2: Property – Proximity Matching
// ---------------------------------------------------------------------------

// TestPropertyProximityMatching verifies that points near polygon edges
// at various distances within threshold are correctly matched.
func TestPropertyProximityMatching(t *testing.T) {
	zones := []model.Zone{munichCentralZone}

	tests := []struct {
		name        string
		point       model.Coordinate
		threshold   float64
		shouldMatch bool
	}{
		{
			name:        "10m north of north edge, 500m threshold",
			point:       model.Coordinate{Lat: 48.14009, Lon: 11.5600},
			threshold:   500.0,
			shouldMatch: true,
		},
		{
			name:        "100m north of north edge, 500m threshold",
			point:       model.Coordinate{Lat: 48.1409, Lon: 11.5600},
			threshold:   500.0,
			shouldMatch: true,
		},
		{
			name:        "450m north of north edge, 500m threshold",
			point:       model.Coordinate{Lat: 48.14404, Lon: 11.5600},
			threshold:   500.0,
			shouldMatch: true,
		},
		{
			name:        "10m south of south edge, 500m threshold",
			point:       model.Coordinate{Lat: 48.13491, Lon: 11.5600},
			threshold:   500.0,
			shouldMatch: true,
		},
		{
			name:        "100m east of east edge, 500m threshold",
			point:       model.Coordinate{Lat: 48.1375, Lon: 11.56635},
			threshold:   500.0,
			shouldMatch: true,
		},
		{
			name:        "1000m north, 500m threshold (should NOT match)",
			point:       model.Coordinate{Lat: 48.149, Lon: 11.5600},
			threshold:   500.0,
			shouldMatch: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := FindMatchingZones(tt.point, zones, tt.threshold)
			found := false
			for _, id := range result {
				if id == "munich-central" {
					found = true
					break
				}
			}
			if tt.shouldMatch && !found {
				t.Errorf("FindMatchingZones(%v, zones, %.1f) = %v, want to contain \"munich-central\"",
					tt.point, tt.threshold, result)
			}
			if !tt.shouldMatch && found {
				t.Errorf("FindMatchingZones(%v, zones, %.1f) = %v, should NOT contain \"munich-central\"",
					tt.point, tt.threshold, result)
			}
		})
	}
}
