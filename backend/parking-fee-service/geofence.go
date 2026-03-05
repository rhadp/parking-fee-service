package main

import "math"

const (
	DefaultProximityThresholdMeters = 500.0     // Default near-zone buffer distance
	EarthRadiusMeters               = 6371000.0 // Mean Earth radius in meters
	BoundaryEpsilonMeters           = 1.0        // Epsilon for boundary detection
)

// PointInPolygon returns true if point is inside the polygon (ray-casting).
func PointInPolygon(point LatLon, polygon []LatLon) bool {
	// Stub: not yet implemented
	_ = math.Abs(0) // suppress unused import
	return false
}

// HaversineDistance returns the geodesic distance in meters between two points.
func HaversineDistance(a, b LatLon) float64 {
	// Stub: not yet implemented
	return 0
}

// DistanceToSegment returns the minimum distance in meters from a point to a line segment.
func DistanceToSegment(point, segA, segB LatLon) float64 {
	// Stub: not yet implemented
	return 0
}

// MinDistanceToPolygon returns the minimum distance from a point to any edge of the polygon.
func MinDistanceToPolygon(point LatLon, polygon []LatLon) float64 {
	// Stub: not yet implemented
	return 0
}

// PointInOrNearPolygon returns true if the point is inside the polygon or within thresholdMeters of any edge.
func PointInOrNearPolygon(point LatLon, polygon []LatLon, thresholdMeters float64) bool {
	// Stub: not yet implemented
	return false
}
