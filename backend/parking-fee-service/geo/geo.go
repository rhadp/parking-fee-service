// Package geo provides geofence matching functions for the parking-fee-service.
package geo

import (
	"math"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// PointInPolygon checks whether point is inside polygon using ray casting.
// Returns true if the point is inside, false otherwise.
// Lat is treated as the y-axis and Lon as the x-axis in 2-D ray casting.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	n := len(polygon)
	if n < 3 {
		return false
	}
	inside := false
	j := n - 1
	for i := 0; i < n; i++ {
		xi, yi := polygon[i].Lon, polygon[i].Lat
		xj, yj := polygon[j].Lon, polygon[j].Lat
		// Edge (j → i) crosses the horizontal ray from point iff exactly one
		// endpoint is strictly above point.Lat and the crossing x is to the right.
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
	const earthRadiusM = 6371000.0

	lat1 := a.Lat * math.Pi / 180
	lat2 := b.Lat * math.Pi / 180
	dLat := (b.Lat - a.Lat) * math.Pi / 180
	dLon := (b.Lon - a.Lon) * math.Pi / 180

	sinHalfDLat := math.Sin(dLat / 2)
	sinHalfDLon := math.Sin(dLon / 2)

	h := sinHalfDLat*sinHalfDLat + math.Cos(lat1)*math.Cos(lat2)*sinHalfDLon*sinHalfDLon
	return earthRadiusM * 2 * math.Atan2(math.Sqrt(h), math.Sqrt(1-h))
}

// nearestOnSegment returns the point on segment [a, b] that is closest to p.
// It uses a flat-Earth projection (scaled by cos(mean-lat)) to compute the
// parameter t, then clamps t to [0, 1] so that endpoints are used when the
// perpendicular foot falls outside the segment.
func nearestOnSegment(p, a, b model.Coordinate) model.Coordinate {
	// Scale longitudes by cos(mean latitude) to approximate equal-area distances.
	meanLat := (a.Lat + b.Lat) / 2 * math.Pi / 180
	cosLat := math.Cos(meanLat)

	ax := a.Lon * cosLat
	ay := a.Lat
	bx := b.Lon * cosLat
	by := b.Lat
	px := p.Lon * cosLat
	py := p.Lat

	dx := bx - ax
	dy := by - ay
	lenSq := dx*dx + dy*dy

	if lenSq == 0 {
		// Degenerate segment: both endpoints coincide.
		return a
	}

	// Project p onto the infinite line through a and b.
	t := ((px-ax)*dx + (py-ay)*dy) / lenSq
	// Clamp to [0, 1] to stay within the segment.
	t = math.Max(0, math.Min(1, t))

	return model.Coordinate{
		Lat: a.Lat + t*(b.Lat-a.Lat),
		Lon: a.Lon + t*(b.Lon-a.Lon),
	}
}

// DistanceToPolygonEdge returns the minimum distance in meters from point to the
// nearest edge of polygon. When the perpendicular foot falls outside the segment
// bounds, the distance to the nearest endpoint is used instead.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	n := len(polygon)
	if n == 0 {
		return math.Inf(1)
	}

	minDist := math.Inf(1)
	for i := 0; i < n; i++ {
		j := (i + 1) % n
		nearest := nearestOnSegment(point, polygon[i], polygon[j])
		d := HaversineDistance(point, nearest)
		if d < minDist {
			minDist = d
		}
	}
	return minDist
}

// FindMatchingZones returns the IDs of zones whose geofence contains or is near point.
// A zone matches if point is inside its polygon, or within threshold meters of its
// nearest polygon edge.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	var result []string
	for _, zone := range zones {
		if PointInPolygon(point, zone.Polygon) {
			result = append(result, zone.ID)
		} else if DistanceToPolygonEdge(point, zone.Polygon) <= threshold {
			result = append(result, zone.ID)
		}
	}
	return result
}
