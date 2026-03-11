package main

import "math"

const (
	DefaultProximityThresholdMeters = 500.0
	EarthRadiusMeters               = 6371000.0
	BoundaryEpsilonMeters           = 1.0
)

// PointInPolygon returns true if point is inside the polygon using the ray-casting algorithm.
// Points exactly on edges may or may not be detected by ray-casting alone;
// use PointInOrNearPolygon with BoundaryEpsilonMeters for boundary inclusion.
func PointInPolygon(point LatLon, polygon []LatLon) bool {
	n := len(polygon)
	if n < 3 {
		return false
	}

	inside := false
	j := n - 1
	for i := 0; i < n; i++ {
		yi := polygon[i].Lat
		xi := polygon[i].Lon
		yj := polygon[j].Lat
		xj := polygon[j].Lon

		// Ray-casting: check if horizontal ray from point crosses edge (i, j)
		if ((yi > point.Lat) != (yj > point.Lat)) &&
			(point.Lon < (xj-xi)*(point.Lat-yi)/(yj-yi)+xi) {
			inside = !inside
		}
		j = i
	}
	return inside
}

// HaversineDistance returns the geodesic distance in meters between two lat/lon points.
func HaversineDistance(a, b LatLon) float64 {
	dLat := degToRad(b.Lat - a.Lat)
	dLon := degToRad(b.Lon - a.Lon)
	lat1 := degToRad(a.Lat)
	lat2 := degToRad(b.Lat)

	sinDLat := math.Sin(dLat / 2)
	sinDLon := math.Sin(dLon / 2)
	h := sinDLat*sinDLat + math.Cos(lat1)*math.Cos(lat2)*sinDLon*sinDLon
	return 2 * EarthRadiusMeters * math.Asin(math.Sqrt(h))
}

// DistanceToSegment returns the minimum distance in meters from a point to a line segment AB.
// The calculation projects the point onto the segment in a local planar approximation,
// then uses Haversine for the final distance.
func DistanceToSegment(point, segA, segB LatLon) float64 {
	// Convert to a local flat coordinate system (meters) centered at segA.
	// This is accurate enough for short segments.
	cosLat := math.Cos(degToRad(segA.Lat))

	// Segment vector in local meters
	dx := degToRad(segB.Lon-segA.Lon) * EarthRadiusMeters * cosLat
	dy := degToRad(segB.Lat-segA.Lat) * EarthRadiusMeters

	// Point vector from segA in local meters
	px := degToRad(point.Lon-segA.Lon) * EarthRadiusMeters * cosLat
	py := degToRad(point.Lat-segA.Lat) * EarthRadiusMeters

	segLenSq := dx*dx + dy*dy
	if segLenSq == 0 {
		// Degenerate segment (A == B)
		return HaversineDistance(point, segA)
	}

	// Parameter t of the projection of point onto the line through A-B, clamped to [0,1]
	t := (px*dx + py*dy) / segLenSq
	if t < 0 {
		t = 0
	} else if t > 1 {
		t = 1
	}

	// Closest point on segment in lat/lon
	closest := LatLon{
		Lat: segA.Lat + t*(segB.Lat-segA.Lat),
		Lon: segA.Lon + t*(segB.Lon-segA.Lon),
	}
	return HaversineDistance(point, closest)
}

// MinDistanceToPolygon returns the minimum distance in meters from a point to any edge of the polygon.
func MinDistanceToPolygon(point LatLon, polygon []LatLon) float64 {
	n := len(polygon)
	if n == 0 {
		return math.Inf(1)
	}
	minDist := math.Inf(1)
	for i := 0; i < n; i++ {
		j := (i + 1) % n
		d := DistanceToSegment(point, polygon[i], polygon[j])
		if d < minDist {
			minDist = d
		}
	}
	return minDist
}

// PointInOrNearPolygon returns true if the point is inside the polygon or
// within thresholdMeters of any polygon edge.
func PointInOrNearPolygon(point LatLon, polygon []LatLon, thresholdMeters float64) bool {
	if PointInPolygon(point, polygon) {
		return true
	}
	return MinDistanceToPolygon(point, polygon) <= thresholdMeters
}

// degToRad converts degrees to radians.
func degToRad(deg float64) float64 {
	return deg * math.Pi / 180.0
}
