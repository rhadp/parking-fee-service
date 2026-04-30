package geo

import (
	"math/rand"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// Munich-central zone polygon used across tests.
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

// TS-05-2: PointInPolygon correctly identifies coordinates inside a convex polygon.
func TestPointInPolygonInside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	if !PointInPolygon(point, munichCentralPolygon) {
		t.Error("expected point (48.1375, 11.5600) to be inside the munich-central polygon")
	}
}

// TS-05-2: PointInPolygon correctly identifies coordinates outside a convex polygon.
func TestPointInPolygonOutside(t *testing.T) {
	point := model.Coordinate{Lat: 48.1500, Lon: 11.5800}
	if PointInPolygon(point, munichCentralPolygon) {
		t.Error("expected point (48.1500, 11.5800) to be outside the munich-central polygon")
	}
}

// TS-05-3: FindMatchingZones includes zones where the point is outside the polygon
// but within the proximity threshold distance from the nearest edge.
func TestProximityMatchingWithinThreshold(t *testing.T) {
	zones := []model.Zone{munichCentralZone}
	// Point ~55m north of the north edge (lat 48.1400 + 0.0005 ≈ 55m)
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
		t.Error("expected munich-central zone to be included in proximity match results")
	}
}

// TS-05-11: The configured proximity threshold is used for near-zone matching.
// A point 50m outside matches with 100m threshold; 200m outside does not.
func TestProximityThresholdUsed(t *testing.T) {
	zones := []model.Zone{munichCentralZone}

	// Point ~50m north of the north edge (lat 48.1400 + 0.00045 ≈ 50m)
	point50m := model.Coordinate{Lat: 48.14045, Lon: 11.5600}
	near := FindMatchingZones(point50m, zones, 100.0)
	if len(near) != 1 {
		t.Errorf("expected 1 matching zone for point 50m outside with 100m threshold, got %d", len(near))
	}

	// Point ~200m north of the north edge (lat 48.1400 + 0.0018 ≈ 200m)
	point200m := model.Coordinate{Lat: 48.1418, Lon: 11.5600}
	far := FindMatchingZones(point200m, zones, 100.0)
	if len(far) != 0 {
		t.Errorf("expected 0 matching zones for point 200m outside with 100m threshold, got %d", len(far))
	}
}

// TS-05-P1: Property test — for any coordinate inside a convex polygon,
// PointInPolygon returns true; for any coordinate far outside, FindMatchingZones
// returns empty.
func TestPropertyPointInPolygon(t *testing.T) {
	// Define several convex polygons and known inside/outside points.
	testCases := []struct {
		name     string
		polygon  []model.Coordinate
		inside   model.Coordinate
		farPoint model.Coordinate // far enough that FindMatchingZones should return empty
	}{
		{
			name: "unit square around origin",
			polygon: []model.Coordinate{
				{Lat: 1.0, Lon: 1.0},
				{Lat: 1.0, Lon: -1.0},
				{Lat: -1.0, Lon: -1.0},
				{Lat: -1.0, Lon: 1.0},
			},
			inside:   model.Coordinate{Lat: 0.0, Lon: 0.0},
			farPoint: model.Coordinate{Lat: 10.0, Lon: 10.0},
		},
		{
			name:     "munich-central",
			polygon:  munichCentralPolygon,
			inside:   model.Coordinate{Lat: 48.1375, Lon: 11.5600},
			farPoint: model.Coordinate{Lat: 49.0, Lon: 12.0},
		},
		{
			name: "small triangle",
			polygon: []model.Coordinate{
				{Lat: 0.0, Lon: 0.0},
				{Lat: 0.001, Lon: 0.0005},
				{Lat: 0.0, Lon: 0.001},
			},
			inside:   model.Coordinate{Lat: 0.0003, Lon: 0.0004},
			farPoint: model.Coordinate{Lat: 5.0, Lon: 5.0},
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name+"/inside", func(t *testing.T) {
			if !PointInPolygon(tc.inside, tc.polygon) {
				t.Errorf("expected PointInPolygon to return true for point inside %s polygon", tc.name)
			}
		})

		t.Run(tc.name+"/far_outside", func(t *testing.T) {
			zone := model.Zone{ID: "test-zone", Name: "test", Polygon: tc.polygon}
			result := FindMatchingZones(tc.farPoint, []model.Zone{zone}, 500.0)
			if len(result) != 0 {
				t.Errorf("expected FindMatchingZones to return empty for point far outside %s polygon, got %v", tc.name, result)
			}
		})
	}

	// Randomized property: generate random points inside the unit square polygon
	// and verify PointInPolygon returns true.
	rng := rand.New(rand.NewSource(42))
	unitSquare := []model.Coordinate{
		{Lat: 1.0, Lon: 1.0},
		{Lat: 1.0, Lon: -1.0},
		{Lat: -1.0, Lon: -1.0},
		{Lat: -1.0, Lon: 1.0},
	}
	for i := 0; i < 100; i++ {
		lat := rng.Float64()*1.8 - 0.9 // (-0.9, 0.9) — well inside
		lon := rng.Float64()*1.8 - 0.9
		p := model.Coordinate{Lat: lat, Lon: lon}
		if !PointInPolygon(p, unitSquare) {
			t.Errorf("random point (%f, %f) should be inside unit square", lat, lon)
		}
	}
}

// TS-05-P2: Property test — for any coordinate outside a zone but within
// threshold meters of the nearest edge, FindMatchingZones includes that zone.
func TestPropertyProximityMatching(t *testing.T) {
	zones := []model.Zone{munichCentralZone}
	threshold := 500.0

	// Generate points just outside each edge of the munich-central polygon,
	// within the threshold distance.
	edgePoints := []struct {
		name  string
		point model.Coordinate
	}{
		// North of north edge (lat 48.14), ~55m out
		{"north_of_polygon", model.Coordinate{Lat: 48.1405, Lon: 11.5600}},
		// South of south edge (lat 48.135), ~55m out
		{"south_of_polygon", model.Coordinate{Lat: 48.1345, Lon: 11.5600}},
		// East of east edge (lon 11.565), ~50m out
		{"east_of_polygon", model.Coordinate{Lat: 48.1375, Lon: 11.5657}},
		// West of west edge (lon 11.555), ~50m out
		{"west_of_polygon", model.Coordinate{Lat: 48.1375, Lon: 11.5543}},
	}

	for _, ep := range edgePoints {
		t.Run(ep.name, func(t *testing.T) {
			// Verify the point is actually outside the polygon
			if PointInPolygon(ep.point, munichCentralPolygon) {
				t.Skip("point is inside the polygon, not a valid proximity test case")
			}

			result := FindMatchingZones(ep.point, zones, threshold)
			found := false
			for _, id := range result {
				if id == "munich-central" {
					found = true
					break
				}
			}
			if !found {
				t.Errorf("expected munich-central to be in proximity match results for %s", ep.name)
			}
		})
	}

	// Verify that points well beyond threshold are NOT matched.
	farPoints := []struct {
		name  string
		point model.Coordinate
	}{
		// ~11km north
		{"far_north", model.Coordinate{Lat: 48.24, Lon: 11.5600}},
		// ~11km south
		{"far_south", model.Coordinate{Lat: 48.04, Lon: 11.5600}},
	}

	for _, fp := range farPoints {
		t.Run(fp.name, func(t *testing.T) {
			result := FindMatchingZones(fp.point, zones, threshold)
			if len(result) != 0 {
				t.Errorf("expected no matches for point %s far beyond threshold", fp.name)
			}
		})
	}
}
