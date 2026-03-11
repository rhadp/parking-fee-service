package main

const (
	DefaultProximityThresholdMeters = 500.0
	EarthRadiusMeters               = 6371000.0
	BoundaryEpsilonMeters           = 1.0
)

// PointInPolygon returns true if point is inside the polygon (ray-casting).
func PointInPolygon(point LatLon, polygon []LatLon) bool {
	// TODO: implement ray-casting algorithm
	return false
}

// HaversineDistance returns the geodesic distance in meters between two points.
func HaversineDistance(a, b LatLon) float64 {
	// TODO: implement Haversine formula
	return 0
}

// DistanceToSegment returns the minimum distance in meters from a point to a line segment.
func DistanceToSegment(point, segA, segB LatLon) float64 {
	// TODO: implement
	return 0
}

// MinDistanceToPolygon returns the minimum distance from a point to any edge of the polygon.
func MinDistanceToPolygon(point LatLon, polygon []LatLon) float64 {
	// TODO: implement
	return 0
}

// PointInOrNearPolygon returns true if the point is inside the polygon or within thresholdMeters of any edge.
func PointInOrNearPolygon(point LatLon, polygon []LatLon, thresholdMeters float64) bool {
	// TODO: implement
	return false
}
