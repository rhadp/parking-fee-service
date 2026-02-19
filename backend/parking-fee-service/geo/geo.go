// Package geo provides geospatial utility functions for parking zone matching.
//
// It includes Haversine distance calculation, point-in-polygon testing via
// the ray-casting algorithm, and minimum distance from a point to a polygon.
package geo

import "math"

// earthRadius is the mean radius of the Earth in meters.
const earthRadius = 6371000.0

// LatLon represents a geographic coordinate as latitude and longitude in degrees.
type LatLon struct {
	Latitude  float64 `json:"latitude"`
	Longitude float64 `json:"longitude"`
}

// degreesToRadians converts degrees to radians.
func degreesToRadians(deg float64) float64 {
	return deg * math.Pi / 180.0
}

// HaversineDistance returns the great-circle distance in meters between two
// geographic points specified by their latitude and longitude in degrees.
//
// Formula:
//
//	a = sin²(Δlat/2) + cos(lat1) * cos(lat2) * sin²(Δlon/2)
//	c = 2 * atan2(√a, √(1−a))
//	d = R * c
func HaversineDistance(lat1, lon1, lat2, lon2 float64) float64 {
	lat1Rad := degreesToRadians(lat1)
	lat2Rad := degreesToRadians(lat2)
	dLat := degreesToRadians(lat2 - lat1)
	dLon := degreesToRadians(lon2 - lon1)

	a := math.Sin(dLat/2)*math.Sin(dLat/2) +
		math.Cos(lat1Rad)*math.Cos(lat2Rad)*math.Sin(dLon/2)*math.Sin(dLon/2)
	c := 2 * math.Atan2(math.Sqrt(a), math.Sqrt(1-a))

	return earthRadius * c
}

// PointInPolygon checks whether a point (lat, lon) lies inside a polygon
// using the ray-casting algorithm.
//
// The algorithm casts a ray from the test point in the +longitude direction
// and counts how many polygon edges the ray crosses. An odd crossing count
// means the point is inside; even means outside.
//
// Points exactly on an edge or vertex may return either true or false — this
// is acceptable for the parking zone demo use case.
func PointInPolygon(lat, lon float64, polygon []LatLon) bool {
	n := len(polygon)
	if n < 3 {
		return false
	}

	inside := false
	j := n - 1

	for i := 0; i < n; i++ {
		yi := polygon[i].Latitude
		xi := polygon[i].Longitude
		yj := polygon[j].Latitude
		xj := polygon[j].Longitude

		// Check if the ray from (lat, lon) in the +x direction crosses
		// the edge from polygon[j] to polygon[i].
		if ((yi > lat) != (yj > lat)) &&
			(lon < (xj-xi)*(lat-yi)/(yj-yi)+xi) {
			inside = !inside
		}

		j = i
	}

	return inside
}

// DistanceToPolygon returns the minimum distance in meters from a point
// (lat, lon) to the nearest edge of a polygon. For each edge segment, it
// finds the closest point on that segment and computes the Haversine distance.
func DistanceToPolygon(lat, lon float64, polygon []LatLon) float64 {
	n := len(polygon)
	if n < 2 {
		if n == 1 {
			return HaversineDistance(lat, lon, polygon[0].Latitude, polygon[0].Longitude)
		}
		return math.Inf(1)
	}

	minDist := math.Inf(1)

	for i := 0; i < n; i++ {
		j := (i + 1) % n
		d := distanceToSegment(lat, lon, polygon[i], polygon[j])
		if d < minDist {
			minDist = d
		}
	}

	return minDist
}

// distanceToSegment returns the Haversine distance in meters from a point
// (lat, lon) to the closest point on the line segment from a to b.
//
// The closest point is found by projecting the point onto the line defined
// by a and b in a flat approximation (suitable for short distances), then
// clamping the projection parameter to [0, 1] to stay on the segment.
func distanceToSegment(lat, lon float64, a, b LatLon) float64 {
	// Vector from a to b.
	dx := b.Longitude - a.Longitude
	dy := b.Latitude - a.Latitude

	// If the segment is a degenerate point, return distance to that point.
	lenSq := dx*dx + dy*dy
	if lenSq == 0 {
		return HaversineDistance(lat, lon, a.Latitude, a.Longitude)
	}

	// Project point onto the line: t is the parameter along the segment.
	t := ((lon-a.Longitude)*dx + (lat-a.Latitude)*dy) / lenSq

	// Clamp t to [0, 1] to stay on the segment.
	if t < 0 {
		t = 0
	} else if t > 1 {
		t = 1
	}

	// Closest point on the segment.
	closestLat := a.Latitude + t*dy
	closestLon := a.Longitude + t*dx

	return HaversineDistance(lat, lon, closestLat, closestLon)
}
