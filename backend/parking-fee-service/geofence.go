package main

import "math"

const (
	DefaultProximityThresholdMeters = 500.0     // Default near-zone buffer distance
	EarthRadiusMeters               = 6371000.0 // Mean Earth radius in meters
	BoundaryEpsilonMeters           = 1.0        // Epsilon for boundary detection
)

// PointInPolygon returns true if point is inside the polygon using the ray-casting algorithm.
// Points exactly on edges are not guaranteed to return true by ray-casting alone;
// use PointInOrNearPolygon with BoundaryEpsilonMeters for boundary inclusion.
func PointInPolygon(point LatLon, polygon []LatLon) bool {
	n := len(polygon)
	if n < 3 {
		return false
	}

	inside := false
	j := n - 1
	for i := 0; i < n; i++ {
		yi, xi := polygon[i].Lat, polygon[i].Lon
		yj, xj := polygon[j].Lat, polygon[j].Lon

		// Ray-casting: check if a horizontal ray from the point crosses this edge
		if ((yi > point.Lat) != (yj > point.Lat)) &&
			(point.Lon < (xj-xi)*(point.Lat-yi)/(yj-yi)+xi) {
			inside = !inside
		}
		j = i
	}
	return inside
}

// HaversineDistance returns the geodesic distance in meters between two points.
func HaversineDistance(a, b LatLon) float64 {
	dLat := degreesToRadians(b.Lat - a.Lat)
	dLon := degreesToRadians(b.Lon - a.Lon)
	lat1 := degreesToRadians(a.Lat)
	lat2 := degreesToRadians(b.Lat)

	sinDLat := math.Sin(dLat / 2)
	sinDLon := math.Sin(dLon / 2)
	h := sinDLat*sinDLat + math.Cos(lat1)*math.Cos(lat2)*sinDLon*sinDLon
	return 2 * EarthRadiusMeters * math.Asin(math.Sqrt(h))
}

// DistanceToSegment returns the minimum distance in meters from a point to a line segment.
// The calculation projects the point onto the segment in a local flat-earth approximation,
// then uses Haversine for the final distance.
func DistanceToSegment(point, segA, segB LatLon) float64 {
	// Use a flat-earth projection centered on segA for the parametric calculation.
	// Convert to approximate meters for the projection.
	cosLat := math.Cos(degreesToRadians(segA.Lat))

	// Vector from segA to segB in approximate meters
	dx := (segB.Lon - segA.Lon) * cosLat
	dy := segB.Lat - segA.Lat

	// Vector from segA to point in approximate meters
	px := (point.Lon - segA.Lon) * cosLat
	py := point.Lat - segA.Lat

	segLenSq := dx*dx + dy*dy
	if segLenSq == 0 {
		// Degenerate segment (both endpoints the same)
		return HaversineDistance(point, segA)
	}

	// Project point onto the line, clamped to [0, 1]
	t := (px*dx + py*dy) / segLenSq
	if t < 0 {
		t = 0
	} else if t > 1 {
		t = 1
	}

	// Closest point on segment in lat/lon
	closest := LatLon{
		Lat: segA.Lat + t*dy,
		Lon: segA.Lon + t*dx/cosLat,
	}

	return HaversineDistance(point, closest)
}

// MinDistanceToPolygon returns the minimum distance from a point to any edge of the polygon.
func MinDistanceToPolygon(point LatLon, polygon []LatLon) float64 {
	n := len(polygon)
	if n == 0 {
		return math.MaxFloat64
	}

	minDist := math.MaxFloat64
	for i := 0; i < n; i++ {
		j := (i + 1) % n
		d := DistanceToSegment(point, polygon[i], polygon[j])
		if d < minDist {
			minDist = d
		}
	}
	return minDist
}

// PointInOrNearPolygon returns true if the point is inside the polygon or within thresholdMeters of any edge.
func PointInOrNearPolygon(point LatLon, polygon []LatLon, thresholdMeters float64) bool {
	if PointInPolygon(point, polygon) {
		return true
	}
	return MinDistanceToPolygon(point, polygon) <= thresholdMeters
}

// degreesToRadians converts degrees to radians.
func degreesToRadians(deg float64) float64 {
	return deg * math.Pi / 180.0
}
