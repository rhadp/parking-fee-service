// Package geo provides geofence operations: point-in-polygon testing,
// Haversine distance, and proximity matching.
package geo

import (
	"math"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

const earthRadiusMeters = 6_371_000.0

// degToRad converts degrees to radians.
func degToRad(deg float64) float64 {
	return deg * math.Pi / 180.0
}

// PointInPolygon returns true if the given point lies inside the polygon
// using the ray casting algorithm.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	n := len(polygon)
	if n < 3 {
		return false
	}

	inside := false
	px, py := point.Lat, point.Lon

	for i, j := 0, n-1; i < n; j, i = i, i+1 {
		yi, xi := polygon[i].Lon, polygon[i].Lat
		yj, xj := polygon[j].Lon, polygon[j].Lat

		// Check if the ray from (px, py) going in the +y direction
		// crosses the edge from polygon[j] to polygon[i].
		if (yi > py) != (yj > py) {
			intersectX := xj + (py-yj)*(xi-xj)/(yi-yj)
			if px < intersectX {
				inside = !inside
			}
		}
	}

	return inside
}

// HaversineDistance returns the great-circle distance in meters between
// two geographic coordinates using the Haversine formula.
func HaversineDistance(a, b model.Coordinate) float64 {
	lat1 := degToRad(a.Lat)
	lat2 := degToRad(b.Lat)
	dLat := degToRad(b.Lat - a.Lat)
	dLon := degToRad(b.Lon - a.Lon)

	h := math.Sin(dLat/2)*math.Sin(dLat/2) +
		math.Cos(lat1)*math.Cos(lat2)*
			math.Sin(dLon/2)*math.Sin(dLon/2)

	c := 2 * math.Atan2(math.Sqrt(h), math.Sqrt(1-h))

	return earthRadiusMeters * c
}

// distanceToSegment returns the minimum geographic distance in meters from
// point p to the line segment defined by endpoints a and b.
//
// It projects the point onto the segment using a parametric representation,
// clamps the parameter to [0, 1], and computes the Haversine distance to
// the closest point on the segment.
func distanceToSegment(p, a, b model.Coordinate) float64 {
	// Use a local Cartesian approximation for projection parameter t,
	// then compute actual Haversine distance to the projected point.
	// This avoids Euclidean distance in raw lat/lon space.
	midLat := degToRad((a.Lat + b.Lat) / 2.0)
	cosLat := math.Cos(midLat)

	// Scale longitude differences by cos(lat) to approximate meters.
	ax := a.Lat
	ay := a.Lon * cosLat
	bx := b.Lat
	by := b.Lon * cosLat
	ppx := p.Lat
	ppy := p.Lon * cosLat

	dx := bx - ax
	dy := by - ay

	lenSq := dx*dx + dy*dy
	if lenSq == 0 {
		// Degenerate segment (a == b), just compute point-to-point distance.
		return HaversineDistance(p, a)
	}

	// Project p onto the line through a and b, clamped to [0, 1].
	t := ((ppx-ax)*dx + (ppy-ay)*dy) / lenSq
	if t < 0 {
		t = 0
	} else if t > 1 {
		t = 1
	}

	// Closest point on the segment in geographic coordinates.
	closest := model.Coordinate{
		Lat: a.Lat + t*(b.Lat-a.Lat),
		Lon: a.Lon + t*(b.Lon-a.Lon),
	}

	return HaversineDistance(p, closest)
}

// DistanceToPolygonEdge returns the minimum distance in meters from a point
// to the nearest edge of the polygon. Each edge is a segment between
// consecutive polygon vertices (including the closing edge from last to first).
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	n := len(polygon)
	if n == 0 {
		return math.Inf(1)
	}
	if n == 1 {
		return HaversineDistance(point, polygon[0])
	}

	minDist := math.Inf(1)
	for i := range n {
		j := (i + 1) % n
		d := distanceToSegment(point, polygon[i], polygon[j])
		if d < minDist {
			minDist = d
		}
	}

	return minDist
}

// FindMatchingZones returns the IDs of zones whose polygon contains the
// given point or whose nearest edge is within the proximity threshold.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	var matched []string
	for _, z := range zones {
		if PointInPolygon(point, z.Polygon) {
			matched = append(matched, z.ID)
			continue
		}
		if DistanceToPolygonEdge(point, z.Polygon) <= threshold {
			matched = append(matched, z.ID)
		}
	}
	return matched
}
