package main

import (
	"testing"
)

// TS-05-P1: Geofence Point-in-Polygon Correctness (unit tests)

var mucCentralPolygon = []LatLon{
	{Lat: 48.1420, Lon: 11.5550},
	{Lat: 48.1420, Lon: 11.5700},
	{Lat: 48.1370, Lon: 11.5700},
	{Lat: 48.1370, Lon: 11.5550},
}

var mucAirportPolygon = []LatLon{
	{Lat: 48.3570, Lon: 11.7750},
	{Lat: 48.3570, Lon: 11.7950},
	{Lat: 48.3480, Lon: 11.7950},
	{Lat: 48.3480, Lon: 11.7750},
}

func TestPointInPolygonInside(t *testing.T) {
	// Point clearly inside the muc-central rectangle
	point := LatLon{Lat: 48.1395, Lon: 11.5625}
	if !PointInPolygon(point, mucCentralPolygon) {
		t.Errorf("expected point %v to be inside polygon, got outside", point)
	}
}

func TestPointInPolygonOutside(t *testing.T) {
	// Point far outside (Berlin)
	point := LatLon{Lat: 52.5200, Lon: 13.4050}
	if PointInPolygon(point, mucCentralPolygon) {
		t.Errorf("expected point %v to be outside polygon, got inside", point)
	}
}

func TestPointInPolygonOnBoundary(t *testing.T) {
	// Point on the northern edge of the polygon (lat=48.1420, between lon 11.5550 and 11.5700)
	point := LatLon{Lat: 48.1420, Lon: 11.5625}
	if !PointInOrNearPolygon(point, mucCentralPolygon, BoundaryEpsilonMeters) {
		t.Errorf("expected boundary point %v to be treated as inside, got outside", point)
	}
}

func TestPointNearZone(t *testing.T) {
	// Point ~55m north of the northern edge (within 500m threshold)
	point := LatLon{Lat: 48.1425, Lon: 11.5625}
	if !PointInOrNearPolygon(point, mucCentralPolygon, 500.0) {
		t.Errorf("expected near point %v to match with 500m threshold, got no match", point)
	}
}

func TestPointOutsideBuffer(t *testing.T) {
	// Point ~50km away (well beyond 500m threshold)
	point := LatLon{Lat: 48.5, Lon: 11.5}
	if PointInOrNearPolygon(point, mucCentralPolygon, 500.0) {
		t.Errorf("expected far point %v to NOT match with 500m threshold, got match", point)
	}
}

func TestPropertyGeofenceCorrectness(t *testing.T) {
	// TS-05-P1: Property test for geofence correctness
	demoPolygons := [][]LatLon{mucCentralPolygon, mucAirportPolygon}

	for i, polygon := range demoPolygons {
		// 1. Every vertex should be classified as inside or on-boundary
		for j, vertex := range polygon {
			if !PointInOrNearPolygon(vertex, polygon, BoundaryEpsilonMeters) {
				t.Errorf("polygon %d: vertex %d %v should match (inside or boundary)", i, j, vertex)
			}
		}

		// 2. Centroid of convex polygon should be inside
		centroid := computeCentroid(polygon)
		if !PointInPolygon(centroid, polygon) {
			t.Errorf("polygon %d: centroid %v should be inside", i, centroid)
		}

		// 3. Point at (0, 0) should be outside for Munich-area polygons
		origin := LatLon{Lat: 0, Lon: 0}
		if PointInPolygon(origin, polygon) {
			t.Errorf("polygon %d: origin (0,0) should be outside", i)
		}
	}
}

func TestPropertyProximityThreshold(t *testing.T) {
	// TS-05-P2: Points within the proximity threshold match; beyond it do not

	// Point ~55m north of northern edge
	nearPoint := LatLon{Lat: 48.1425, Lon: 11.5625}
	if !PointInOrNearPolygon(nearPoint, mucCentralPolygon, 500.0) {
		t.Errorf("near point %v should match within 500m threshold", nearPoint)
	}

	// Point ~50km away
	farPoint := LatLon{Lat: 48.5, Lon: 11.5}
	if PointInOrNearPolygon(farPoint, mucCentralPolygon, 500.0) {
		t.Errorf("far point %v should NOT match within 500m threshold", farPoint)
	}
}

// computeCentroid returns the centroid of a polygon (average of vertices).
func computeCentroid(polygon []LatLon) LatLon {
	var lat, lon float64
	for _, p := range polygon {
		lat += p.Lat
		lon += p.Lon
	}
	n := float64(len(polygon))
	return LatLon{Lat: lat / n, Lon: lon / n}
}
