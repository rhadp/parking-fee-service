package geo_test

import (
	"math"
	"math/rand"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/geo"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// munichCentralPolygon is the default Munich Central Station zone polygon.
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
	inside := model.Coordinate{Lat: 48.1375, Lon: 11.5600}
	if !geo.PointInPolygon(inside, munichCentralPolygon) {
		t.Errorf("PointInPolygon(%v, polygon) = false, want true", inside)
	}
}

// TS-05-2: PointInPolygon correctly identifies coordinates outside a convex polygon.
func TestPointInPolygonOutside(t *testing.T) {
	outside := model.Coordinate{Lat: 48.1500, Lon: 11.5800}
	if geo.PointInPolygon(outside, munichCentralPolygon) {
		t.Errorf("PointInPolygon(%v, polygon) = true, want false", outside)
	}
}

// TS-05-3: FindMatchingZones includes zones where the point is outside the
// polygon but within the proximity threshold.
func TestProximityMatchingWithinThreshold(t *testing.T) {
	zones := []model.Zone{munichCentralZone}
	// Point ~55m north of the polygon's northern edge (48.14 -> 48.1405)
	nearPoint := model.Coordinate{Lat: 48.1405, Lon: 11.5600}
	result := geo.FindMatchingZones(nearPoint, zones, 500.0)
	found := false
	for _, id := range result {
		if id == "munich-central" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("FindMatchingZones(%v, zones, 500.0) did not include 'munich-central', got %v", nearPoint, result)
	}
}

// TS-05-11: Proximity threshold is used correctly: points within threshold match,
// points beyond threshold do not.
func TestProximityThresholdUsed(t *testing.T) {
	zones := []model.Zone{munichCentralZone}

	// Point ~55m outside the polygon (within 100m threshold)
	point50m := model.Coordinate{Lat: 48.1405, Lon: 11.5600}
	near := geo.FindMatchingZones(point50m, zones, 100.0)
	if len(near) != 1 {
		t.Errorf("FindMatchingZones(~55m outside, zones, 100.0) = %v, want 1 match", near)
	}

	// Point ~2km outside the polygon (beyond 100m threshold)
	point200m := model.Coordinate{Lat: 48.1600, Lon: 11.5600}
	far := geo.FindMatchingZones(point200m, zones, 100.0)
	if len(far) != 0 {
		t.Errorf("FindMatchingZones(~2km outside, zones, 100.0) = %v, want 0 matches", far)
	}
}

// TS-05-P1: Property test for point-in-polygon correctness.
// For a known convex polygon, points generated inside must return true,
// and points far outside must not appear in FindMatchingZones.
func TestPropertyPointInPolygon(t *testing.T) {
	// Use a well-defined square polygon centered at (48.1375, 11.56)
	polygon := munichCentralPolygon
	zone := munichCentralZone

	rng := rand.New(rand.NewSource(42))

	// Generate 100 random points inside the polygon
	for i := 0; i < 100; i++ {
		lat := 48.1350 + rng.Float64()*(48.1400-48.1350)
		lon := 11.5550 + rng.Float64()*(11.5650-11.5550)
		pt := model.Coordinate{Lat: lat, Lon: lon}
		if !geo.PointInPolygon(pt, polygon) {
			t.Errorf("iteration %d: PointInPolygon(%v, polygon) = false, want true (point inside polygon)", i, pt)
		}
	}

	// Generate 100 random points far outside the polygon
	threshold := 500.0
	for i := 0; i < 100; i++ {
		// Points at least ~10km away
		lat := 48.1375 + (rng.Float64()*2-1)*0.1
		lon := 11.5600 + (rng.Float64()*2-1)*0.1
		// Skip points that might be inside or near the polygon
		if lat >= 48.1340 && lat <= 48.1410 && lon >= 11.5540 && lon <= 11.5660 {
			continue
		}
		pt := model.Coordinate{Lat: lat, Lon: lon}
		result := geo.FindMatchingZones(pt, []model.Zone{zone}, threshold)
		if len(result) != 0 {
			t.Errorf("iteration %d: FindMatchingZones(%v, zones, %f) = %v, want empty (point far outside polygon)", i, pt, threshold, result)
		}
	}
}

// TestProximityMatchingDiagonalEdge validates that DistanceToPolygonEdge and
// FindMatchingZones produce correct results for diagonal (non-axis-aligned)
// polygon edges, where naive lat/lon projection without cos(lat) scaling
// would overestimate distance by up to ~33% at Munich's latitude.
func TestProximityMatchingDiagonalEdge(t *testing.T) {
	// A diamond-shaped polygon with only diagonal edges.
	diamondPolygon := []model.Coordinate{
		{Lat: 48.1400, Lon: 11.5600}, // top
		{Lat: 48.1375, Lon: 11.5650}, // right
		{Lat: 48.1350, Lon: 11.5600}, // bottom
		{Lat: 48.1375, Lon: 11.5550}, // left
	}
	diamondZone := model.Zone{
		ID:      "diamond",
		Name:    "Diamond Zone",
		Polygon: diamondPolygon,
	}

	// A point slightly outside the top-right diagonal edge, ~50m away.
	// Without cos(lat) correction, this point's distance would be overestimated.
	nearDiagonal := model.Coordinate{Lat: 48.1392, Lon: 11.5630}

	dist := geo.DistanceToPolygonEdge(nearDiagonal, diamondPolygon)
	// The point should be within ~100m of the edge.
	if dist > 150 {
		t.Errorf("DistanceToPolygonEdge for point near diagonal edge = %f meters, want < 150m", dist)
	}

	// With a 200m threshold, this point should match the zone.
	result := geo.FindMatchingZones(nearDiagonal, []model.Zone{diamondZone}, 200.0)
	found := false
	for _, id := range result {
		if id == "diamond" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("FindMatchingZones(near diagonal, zones, 200.0) did not include 'diamond'; dist=%f, got %v", dist, result)
	}
}

// TS-05-P2: Property test for proximity matching.
// Points generated just outside the polygon edge within threshold must match.
func TestPropertyProximityMatching(t *testing.T) {
	zone := munichCentralZone
	threshold := 500.0

	rng := rand.New(rand.NewSource(99))

	// Generate points slightly outside each edge of the polygon
	edges := [][2]model.Coordinate{
		{munichCentralPolygon[0], munichCentralPolygon[1]}, // top edge
		{munichCentralPolygon[1], munichCentralPolygon[2]}, // right edge
		{munichCentralPolygon[2], munichCentralPolygon[3]}, // bottom edge
		{munichCentralPolygon[3], munichCentralPolygon[0]}, // left edge
	}

	for edgeIdx, edge := range edges {
		for i := 0; i < 10; i++ {
			// Interpolate along the edge
			frac := 0.1 + rng.Float64()*0.8
			midLat := edge[0].Lat + frac*(edge[1].Lat-edge[0].Lat)
			midLon := edge[0].Lon + frac*(edge[1].Lon-edge[0].Lon)

			// Push ~100m outward from polygon center
			centerLat := 48.1375
			centerLon := 11.5600
			dLat := midLat - centerLat
			dLon := midLon - centerLon
			norm := math.Sqrt(dLat*dLat + dLon*dLon)
			if norm == 0 {
				continue
			}
			// ~100m offset: ~0.0009 degrees latitude
			offset := 0.0009
			pt := model.Coordinate{
				Lat: midLat + (dLat/norm)*offset,
				Lon: midLon + (dLon/norm)*offset,
			}

			result := geo.FindMatchingZones(pt, []model.Zone{zone}, threshold)
			found := false
			for _, id := range result {
				if id == "munich-central" {
					found = true
					break
				}
			}
			if !found {
				t.Errorf("edge %d, iteration %d: FindMatchingZones(%v, zones, %f) did not include 'munich-central', got %v",
					edgeIdx, i, pt, threshold, result)
			}
		}
	}
}
