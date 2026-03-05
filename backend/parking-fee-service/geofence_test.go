package main

import "testing"

// Demo polygon: Munich Central Station Area (rectangle)
var mucCentralPolygon = []LatLon{
	{Lat: 48.1420, Lon: 11.5550},
	{Lat: 48.1420, Lon: 11.5700},
	{Lat: 48.1370, Lon: 11.5700},
	{Lat: 48.1370, Lon: 11.5550},
}

// TestPointInPolygonInside tests a point known to be inside the rectangle.
func TestPointInPolygonInside(t *testing.T) {
	// Center of the Munich Central rectangle
	point := LatLon{Lat: 48.1395, Lon: 11.5625}
	if !PointInPolygon(point, mucCentralPolygon) {
		t.Error("expected point inside polygon to return true")
	}
}

// TestPointInPolygonOutside tests a point known to be far outside.
func TestPointInPolygonOutside(t *testing.T) {
	point := LatLon{Lat: 52.5200, Lon: 13.4050} // Berlin
	if PointInPolygon(point, mucCentralPolygon) {
		t.Error("expected point far outside polygon to return false")
	}
}

// TestPointInPolygonOnBoundary tests a point on the polygon edge.
func TestPointInPolygonOnBoundary(t *testing.T) {
	// Northern edge of the rectangle at lat=48.1420
	point := LatLon{Lat: 48.1420, Lon: 11.5625}
	if !PointInOrNearPolygon(point, mucCentralPolygon, BoundaryEpsilonMeters) {
		t.Error("expected point on boundary to be treated as inside")
	}
}

// TestPointNearZone tests a point outside but within the proximity threshold.
func TestPointNearZone(t *testing.T) {
	// ~55m north of northern edge
	point := LatLon{Lat: 48.1425, Lon: 11.5625}
	if !PointInOrNearPolygon(point, mucCentralPolygon, 500.0) {
		t.Error("expected point near zone (within 500m threshold) to match")
	}
}

// TestPointOutsideBuffer tests a point outside and beyond the proximity threshold.
func TestPointOutsideBuffer(t *testing.T) {
	// ~50km away
	point := LatLon{Lat: 48.5, Lon: 11.5}
	if PointInOrNearPolygon(point, mucCentralPolygon, 500.0) {
		t.Error("expected point far outside buffer to not match")
	}
}

// TS-05-P1: Geofence Point-in-Polygon Correctness (Property Test)
func TestPropertyGeofenceCorrectness(t *testing.T) {
	polygons := [][]LatLon{
		mucCentralPolygon,
		{ // Munich Airport
			{Lat: 48.3570, Lon: 11.7750},
			{Lat: 48.3570, Lon: 11.7950},
			{Lat: 48.3480, Lon: 11.7950},
			{Lat: 48.3480, Lon: 11.7750},
		},
	}

	for i, polygon := range polygons {
		// Every vertex should be inside or on boundary
		for j, vertex := range polygon {
			if !PointInOrNearPolygon(vertex, polygon, BoundaryEpsilonMeters) {
				t.Errorf("polygon %d: vertex %d should be inside/on-boundary", i, j)
			}
		}

		// Centroid of a convex polygon should be inside
		centroid := computeCentroid(polygon)
		if !PointInPolygon(centroid, polygon) {
			t.Errorf("polygon %d: centroid (%f, %f) should be inside", i, centroid.Lat, centroid.Lon)
		}

		// (0, 0) should be outside all Munich-area polygons
		origin := LatLon{Lat: 0, Lon: 0}
		if PointInPolygon(origin, polygon) {
			t.Errorf("polygon %d: origin (0,0) should be outside", i)
		}
	}
}

// TS-05-P2: Proximity Threshold Matching (Property Test)
func TestPropertyProximityThreshold(t *testing.T) {
	// Point ~55m north of northern edge
	nearPoint := LatLon{Lat: 48.1425, Lon: 11.5625}
	if !PointInOrNearPolygon(nearPoint, mucCentralPolygon, 500.0) {
		t.Error("near point should match with 500m threshold")
	}

	// Point ~50km away
	farPoint := LatLon{Lat: 48.5, Lon: 11.5}
	if PointInOrNearPolygon(farPoint, mucCentralPolygon, 500.0) {
		t.Error("far point should not match with 500m threshold")
	}
}

// computeCentroid returns the centroid of a polygon.
func computeCentroid(polygon []LatLon) LatLon {
	var lat, lon float64
	for _, p := range polygon {
		lat += p.Lat
		lon += p.Lon
	}
	n := float64(len(polygon))
	return LatLon{Lat: lat / n, Lon: lon / n}
}
