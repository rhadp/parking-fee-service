package geo

import (
	"math"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

const earthRadius = 6371000.0 // meters

// PointInPolygon tests whether a point lies inside a polygon using ray casting.
// The algorithm casts a ray from the point in the +Lon direction and counts
// edge crossings. An odd number of crossings means the point is inside.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	n := len(polygon)
	if n < 3 {
		return false
	}

	inside := false
	j := n - 1
	for i := 0; i < n; i++ {
		pi := polygon[i]
		pj := polygon[j]

		// Check if the ray from the point crosses the edge (pj -> pi).
		if (pi.Lat > point.Lat) != (pj.Lat > point.Lat) {
			// Compute the longitude of the intersection of the ray with the edge.
			intersectLon := pj.Lon + (point.Lat-pj.Lat)/(pi.Lat-pj.Lat)*(pi.Lon-pj.Lon)
			if point.Lon < intersectLon {
				inside = !inside
			}
		}
		j = i
	}
	return inside
}

// HaversineDistance calculates the great-circle distance between two
// coordinates in meters using the Haversine formula.
func HaversineDistance(a, b model.Coordinate) float64 {
	lat1 := a.Lat * math.Pi / 180
	lat2 := b.Lat * math.Pi / 180
	dLat := (b.Lat - a.Lat) * math.Pi / 180
	dLon := (b.Lon - a.Lon) * math.Pi / 180

	h := math.Sin(dLat/2)*math.Sin(dLat/2) +
		math.Cos(lat1)*math.Cos(lat2)*math.Sin(dLon/2)*math.Sin(dLon/2)
	c := 2 * math.Atan2(math.Sqrt(h), math.Sqrt(1-h))
	return earthRadius * c
}

// DistanceToPolygonEdge returns the minimum distance in meters from a point
// to the nearest edge of a polygon. Each edge is a line segment between
// consecutive vertices.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	n := len(polygon)
	if n < 2 {
		if n == 1 {
			return HaversineDistance(point, polygon[0])
		}
		return math.Inf(1)
	}

	minDist := math.Inf(1)
	for i := 0; i < n; i++ {
		j := (i + 1) % n
		d := distanceToSegment(point, polygon[i], polygon[j])
		if d < minDist {
			minDist = d
		}
	}
	return minDist
}

// distanceToSegment computes the minimum distance in meters from a point to
// a line segment defined by two endpoints (a, b). It projects the point onto
// the segment and clamps the projection parameter to [0, 1].
func distanceToSegment(point, a, b model.Coordinate) float64 {
	// Work in a local projected coordinate system for the projection parameter.
	// Use a simple equirectangular approximation for the projection calculation,
	// then compute the actual distance using Haversine.
	cosLat := math.Cos(point.Lat * math.Pi / 180)

	// Vector from a to b in projected coords
	dx := (b.Lon - a.Lon) * cosLat
	dy := b.Lat - a.Lat

	// If the segment is degenerate (zero length), return distance to point a.
	segLenSq := dx*dx + dy*dy
	if segLenSq == 0 {
		return HaversineDistance(point, a)
	}

	// Project point onto the line defined by a-b, clamping t to [0, 1].
	px := (point.Lon - a.Lon) * cosLat
	py := point.Lat - a.Lat
	t := (px*dx + py*dy) / segLenSq
	if t < 0 {
		t = 0
	} else if t > 1 {
		t = 1
	}

	// Find the closest point on the segment.
	closest := model.Coordinate{
		Lat: a.Lat + t*(b.Lat-a.Lat),
		Lon: a.Lon + t*(b.Lon-a.Lon),
	}

	return HaversineDistance(point, closest)
}

// FindMatchingZones returns zone IDs matching the given coordinate. A zone
// matches if the coordinate is inside its polygon (point-in-polygon test) or
// if the coordinate is within the threshold distance (meters) from the
// nearest polygon edge.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	var result []string
	for _, zone := range zones {
		if PointInPolygon(point, zone.Polygon) {
			result = append(result, zone.ID)
			continue
		}
		if DistanceToPolygonEdge(point, zone.Polygon) <= threshold {
			result = append(result, zone.ID)
		}
	}
	return result
}
