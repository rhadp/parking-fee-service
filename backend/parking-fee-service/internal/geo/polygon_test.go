package geo

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/model"
)

// defaultTestPolygon returns a rectangular polygon representing the Munich City
// Center zone used by most tests.
func defaultTestPolygon() []model.Point {
	return []model.Point{
		{Lat: 48.14, Lon: 11.56},
		{Lat: 48.14, Lon: 11.59},
		{Lat: 48.13, Lon: 11.59},
		{Lat: 48.13, Lon: 11.56},
	}
}

// defaultTestOperators returns a slice with one test operator using the default
// polygon.
func defaultTestOperators() []model.Operator {
	return []model.Operator{
		{
			ID:   "op-munich-01",
			Name: "Munich City Parking",
			Zone: model.Zone{
				ID:      "zone-munich-center",
				Name:    "Munich City Center",
				Polygon: defaultTestPolygon(),
			},
			Rate: model.Rate{
				AmountPerHour: 2.50,
				Currency:      "EUR",
			},
			Adapter: model.Adapter{
				ImageRef:       "us-docker.pkg.dev/rhadp-demo/adapters/munich-parking:v1.0.0",
				ChecksumSHA256: "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
				Version:        "1.0.0",
			},
		},
	}
}

// --- TS-05-5: Point-in-polygon ray casting algorithm ---

func TestPointInPolygon_Basic(t *testing.T) {
	polygon := defaultTestPolygon()

	// Point inside the polygon (Munich city center)
	inside := model.Point{Lat: 48.135, Lon: 11.575}
	if !PointInPolygon(inside, polygon) {
		t.Errorf("expected point %v to be inside polygon, got outside", inside)
	}

	// Point outside the polygon
	outside := model.Point{Lat: 48.10, Lon: 11.50}
	if PointInPolygon(outside, polygon) {
		t.Errorf("expected point %v to be outside polygon, got inside", outside)
	}
}

// --- TS-05-6: Polygon defined as ordered vertex list (implicit close) ---

func TestPointInPolygon_ImplicitClose(t *testing.T) {
	// Triangle that does not repeat first vertex — polygon should still close
	triangle := []model.Point{
		{Lat: 48.14, Lon: 11.56},
		{Lat: 48.14, Lon: 11.59},
		{Lat: 48.12, Lon: 11.575},
	}

	inside := model.Point{Lat: 48.135, Lon: 11.575}
	if !PointInPolygon(inside, triangle) {
		t.Errorf("expected point %v to be inside triangle, got outside", inside)
	}

	outside := model.Point{Lat: 48.10, Lon: 11.50}
	if PointInPolygon(outside, triangle) {
		t.Errorf("expected point %v to be outside triangle, got inside", outside)
	}
}

// --- TS-05-7: Polygon with 3 or more vertices (triangle) ---

func TestPointInPolygon_Triangle(t *testing.T) {
	triangle := []model.Point{
		{Lat: 0, Lon: 0},
		{Lat: 0, Lon: 10},
		{Lat: 10, Lon: 0},
	}

	inside := model.Point{Lat: 1, Lon: 1}
	if !PointInPolygon(inside, triangle) {
		t.Errorf("expected point %v to be inside triangle, got outside", inside)
	}

	outside := model.Point{Lat: 9, Lon: 9}
	if PointInPolygon(outside, triangle) {
		t.Errorf("expected point %v to be outside triangle, got inside", outside)
	}
}

// --- TS-05-8: Fuzziness buffer configurable ---

func TestFuzziness_Configurable(t *testing.T) {
	ops := defaultTestOperators()

	// Point approximately 50m outside the northern edge of the polygon.
	// The polygon's north edge is at lat 48.14. At this latitude, 1 degree of
	// latitude ~ 111,320 m, so 0.0005 deg ~ 55 m.
	nearPoint := model.Point{Lat: 48.1405, Lon: 11.575}

	// With fuzziness 100m, should match (point is ~55m outside)
	matches100 := FindMatches(nearPoint.Lat, nearPoint.Lon, ops, 100)
	if len(matches100) < 1 {
		t.Errorf("expected at least 1 match with fuzziness=100m, got %d", len(matches100))
	}

	// With fuzziness 10m, should NOT match (point is ~55m outside)
	matches10 := FindMatches(nearPoint.Lat, nearPoint.Lon, ops, 10)
	if len(matches10) != 0 {
		t.Errorf("expected 0 matches with fuzziness=10m, got %d", len(matches10))
	}
}

// --- TS-05-9: Near-zone point matched within buffer ---

func TestFuzziness_NearBoundary(t *testing.T) {
	polygon := defaultTestPolygon()

	// Point just north of polygon edge, approximately 30m outside.
	nearPoint := model.Point{Lat: 48.14027, Lon: 11.575}

	// Confirm the point is outside the polygon
	if PointInPolygon(nearPoint, polygon) {
		t.Fatal("test precondition failed: nearPoint should be outside polygon")
	}

	// Confirm distance > 0 and < 100m
	dist := MinDistanceToPolygon(nearPoint, polygon)
	if dist <= 0 {
		t.Errorf("expected distance > 0 (point outside polygon), got %f", dist)
	}
	if dist >= 100 {
		t.Errorf("expected distance < 100m, got %f", dist)
	}

	// FindMatches with 100m buffer should include this point
	ops := defaultTestOperators()
	matches := FindMatches(nearPoint.Lat, nearPoint.Lon, ops, 100)
	if len(matches) < 1 {
		t.Errorf("expected at least 1 match for near-boundary point, got %d", len(matches))
	}
}

// --- TS-05-E8: Degenerate polygon skipped ---

func TestEdge_DegeneratePolygon(t *testing.T) {
	ops := []model.Operator{
		{
			ID:   "degenerate",
			Name: "Degenerate Op",
			Zone: model.Zone{
				ID:      "zone-degen",
				Name:    "Degenerate Zone",
				Polygon: []model.Point{{Lat: 48.14, Lon: 11.56}, {Lat: 48.13, Lon: 11.59}}, // only 2 vertices
			},
		},
	}

	matches := FindMatches(48.135, 11.575, ops, 0)
	if len(matches) != 0 {
		t.Errorf("expected 0 matches for degenerate polygon, got %d", len(matches))
	}
}

// --- TS-05-E9: Fuzziness zero disables near-zone matching ---

func TestEdge_FuzzinessZero(t *testing.T) {
	polygon := defaultTestPolygon()

	// Point just outside the polygon boundary
	nearPoint := model.Point{Lat: 48.1405, Lon: 11.575}

	// Confirm point is outside polygon
	if PointInPolygon(nearPoint, polygon) {
		t.Fatal("test precondition failed: nearPoint should be outside polygon")
	}

	ops := defaultTestOperators()
	matches := FindMatches(nearPoint.Lat, nearPoint.Lon, ops, 0)
	if len(matches) != 0 {
		t.Errorf("expected 0 matches with fuzziness=0 for near-boundary point, got %d", len(matches))
	}
}

// --- TS-05-P1: Geofence Determinism ---

func TestProperty_GeofenceDeterminism(t *testing.T) {
	polygon := defaultTestPolygon()
	points := []model.Point{
		{Lat: 48.135, Lon: 11.575},  // inside
		{Lat: 48.10, Lon: 11.50},    // outside
		{Lat: 48.14, Lon: 11.56},    // on vertex/boundary
	}

	for _, p := range points {
		expected := PointInPolygon(p, polygon)
		for i := 0; i < 100; i++ {
			result := PointInPolygon(p, polygon)
			if result != expected {
				t.Errorf("PointInPolygon(%v) non-deterministic: iteration %d returned %v, expected %v",
					p, i, result, expected)
			}
		}
	}
}

// --- TS-05-P2: Fuzziness Monotonicity ---

func TestProperty_FuzzinessMonotonicity(t *testing.T) {
	ops := defaultTestOperators()

	// Point near polygon boundary
	nearPoint := model.Point{Lat: 48.1405, Lon: 11.575}

	for d1 := float64(10); d1 <= 500; d1 += 10 {
		m1 := FindMatches(nearPoint.Lat, nearPoint.Lon, ops, d1)
		m2 := FindMatches(nearPoint.Lat, nearPoint.Lon, ops, d1+50)
		if len(m1) > 0 && len(m2) < len(m1) {
			t.Errorf("fuzziness monotonicity violated: %d matches at %.0fm, but %d matches at %.0fm",
				len(m1), d1, len(m2), d1+50)
		}
	}
}

// --- TS-05-P3: Interior Points Always Match ---

func TestProperty_InteriorPointsMatch(t *testing.T) {
	polygon := defaultTestPolygon()
	ops := defaultTestOperators()

	interiorPoints := []model.Point{
		{Lat: 48.135, Lon: 11.575},   // center
		{Lat: 48.1390, Lon: 11.5610}, // near NW corner but inside
		{Lat: 48.1310, Lon: 11.5890}, // near SE corner but inside
	}

	for _, p := range interiorPoints {
		if !PointInPolygon(p, polygon) {
			t.Errorf("expected interior point %v to be inside polygon", p)
		}

		// Also verify with fuzziness = 0
		matches := FindMatches(p.Lat, p.Lon, ops, 0)
		if len(matches) < 1 {
			t.Errorf("expected at least 1 match for interior point %v with fuzziness=0, got %d",
				p, len(matches))
		}
	}
}

// --- TS-05-P4: Distant Points Never Match ---

func TestProperty_DistantPointsNeverMatch(t *testing.T) {
	ops := defaultTestOperators()

	distantPoints := []model.Point{
		{Lat: 0.0, Lon: 0.0},     // Gulf of Guinea
		{Lat: 40.0, Lon: -74.0},  // New York
		{Lat: 51.5, Lon: -0.1},   // London
	}

	for _, p := range distantPoints {
		matches := FindMatches(p.Lat, p.Lon, ops, 100)
		if len(matches) != 0 {
			t.Errorf("expected 0 matches for distant point %v, got %d", p, len(matches))
		}
	}
}
