package geo

import (
	"math"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

const earthRadiusMeters = 6371000.0

// degToRad converts degrees to radians.
func degToRad(deg float64) float64 {
	return deg * math.Pi / 180.0
}

// PointInPolygon tests whether a point is inside a polygon using the ray
// casting algorithm. It casts a horizontal ray from the point to the right
// and counts edge crossings. An odd count means the point is inside.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
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

		// Check if the ray from (point.Lat, point.Lon) going rightward
		// crosses the edge from polygon[j] to polygon[i].
		if (yi > point.Lat) != (yj > point.Lat) {
			// Compute the x-intersection of the ray with this edge.
			xIntersect := xj + (point.Lat-yj)/(yi-yj)*(xi-xj)
			if point.Lon < xIntersect {
				inside = !inside
			}
		}
		j = i
	}
	return inside
}

// HaversineDistance computes the great-circle distance in meters between two
// coordinates using the Haversine formula.
func HaversineDistance(a, b model.Coordinate) float64 {
	lat1 := degToRad(a.Lat)
	lat2 := degToRad(b.Lat)
	dLat := degToRad(b.Lat - a.Lat)
	dLon := degToRad(b.Lon - a.Lon)

	h := math.Sin(dLat/2)*math.Sin(dLat/2) +
		math.Cos(lat1)*math.Cos(lat2)*math.Sin(dLon/2)*math.Sin(dLon/2)

	c := 2 * math.Atan2(math.Sqrt(h), math.Sqrt(1-h))
	return earthRadiusMeters * c
}

// DistanceToPolygonEdge returns the minimum distance in meters from a point
// to the nearest edge of a polygon. For each edge segment, it projects the
// point onto the line defined by the segment endpoints, clamps the projection
// to the segment, and computes the Haversine distance to the closest point.
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

// distanceToSegment computes the minimum distance from point P to the line
// segment defined by endpoints A and B. It projects P onto the line AB using
// a parametric approach in lat/lon space, clamps the parameter to [0, 1] to
// stay on the segment, then computes the Haversine distance to the nearest
// point on the segment.
func distanceToSegment(p, a, b model.Coordinate) float64 {
	dx := b.Lon - a.Lon
	dy := b.Lat - a.Lat

	// If the segment is a single point, return distance to that point.
	lenSq := dx*dx + dy*dy
	if lenSq == 0 {
		return HaversineDistance(p, a)
	}

	// Compute the parametric position of the projection of P onto line AB.
	// t = dot(P-A, B-A) / |B-A|^2
	t := ((p.Lon-a.Lon)*dx + (p.Lat-a.Lat)*dy) / lenSq

	// Clamp t to [0, 1] to stay within the segment.
	if t < 0 {
		t = 0
	} else if t > 1 {
		t = 1
	}

	// Compute the closest point on the segment.
	closest := model.Coordinate{
		Lat: a.Lat + t*dy,
		Lon: a.Lon + t*dx,
	}

	return HaversineDistance(p, closest)
}

// FindMatchingZones returns the IDs of zones that contain or are near the
// given point. A zone matches if the point is inside its polygon (ray casting)
// or within the proximity threshold distance from its nearest polygon edge
// (Haversine distance).
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	var matched []string
	for _, zone := range zones {
		if PointInPolygon(point, zone.Polygon) {
			matched = append(matched, zone.ID)
			continue
		}
		if DistanceToPolygonEdge(point, zone.Polygon) <= threshold {
			matched = append(matched, zone.ID)
		}
	}
	return matched
}
