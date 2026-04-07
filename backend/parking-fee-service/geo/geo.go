// Package geo provides geofence operations: point-in-polygon, distance, and zone matching.
package geo

import (
	"math"

	"parking-fee-service/backend/parking-fee-service/model"
)

const earthRadiusMeters = 6371000.0

// PointInPolygon tests whether a point lies inside a polygon using ray casting.
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

		if ((yi > point.Lat) != (yj > point.Lat)) &&
			(point.Lon < (xj-xi)*(point.Lat-yi)/(yj-yi)+xi) {
			inside = !inside
		}
		j = i
	}
	return inside
}

// HaversineDistance calculates the great-circle distance in meters between two coordinates.
func HaversineDistance(a, b model.Coordinate) float64 {
	lat1 := a.Lat * math.Pi / 180
	lat2 := b.Lat * math.Pi / 180
	dLat := (b.Lat - a.Lat) * math.Pi / 180
	dLon := (b.Lon - a.Lon) * math.Pi / 180

	h := math.Sin(dLat/2)*math.Sin(dLat/2) +
		math.Cos(lat1)*math.Cos(lat2)*math.Sin(dLon/2)*math.Sin(dLon/2)
	c := 2 * math.Atan2(math.Sqrt(h), math.Sqrt(1-h))

	return earthRadiusMeters * c
}

// distanceToSegment returns the minimum distance in meters from a point to a
// line segment defined by two endpoints. It projects the point onto the segment
// and clamps to the nearest endpoint if the projection falls outside.
func distanceToSegment(point, segA, segB model.Coordinate) float64 {
	// Use a flat-earth approximation for the projection parameter, then
	// compute the actual Haversine distance to the closest point on the segment.
	// This is accurate enough for small polygons (city-scale).
	cosLat := math.Cos(point.Lat * math.Pi / 180)

	dx := (segB.Lon - segA.Lon) * cosLat
	dy := segB.Lat - segA.Lat
	lenSq := dx*dx + dy*dy

	if lenSq == 0 {
		return HaversineDistance(point, segA)
	}

	px := (point.Lon - segA.Lon) * cosLat
	py := point.Lat - segA.Lat

	t := (px*dx + py*dy) / lenSq
	if t < 0 {
		t = 0
	} else if t > 1 {
		t = 1
	}

	closest := model.Coordinate{
		Lat: segA.Lat + t*(segB.Lat-segA.Lat),
		Lon: segA.Lon + t*(segB.Lon-segA.Lon),
	}

	return HaversineDistance(point, closest)
}

// DistanceToPolygonEdge returns the minimum distance in meters from a point to the nearest polygon edge.
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

// FindMatchingZones returns zone IDs whose polygons contain the point or are within threshold meters.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	var matched []string
	for _, zone := range zones {
		if PointInPolygon(point, zone.Polygon) {
			matched = append(matched, zone.ID)
			continue
		}
		if threshold > 0 && DistanceToPolygonEdge(point, zone.Polygon) <= threshold {
			matched = append(matched, zone.ID)
		}
	}
	return matched
}
