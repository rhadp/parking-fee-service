// Package geo implements geofence logic: point-in-polygon (ray casting),
// Haversine distance, and proximity-based zone matching.
package geo

import (
	"math"

	"parking-fee-service/backend/parking-fee-service/model"
)

const earthRadiusMeters = 6_371_000.0

// PointInPolygon reports whether point lies inside polygon using the ray
// casting algorithm. Treats lat as the Y axis and lon as the X axis.
// Returns false for degenerate polygons with fewer than 3 vertices.
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

		// Ray cast in the positive-longitude direction from point.
		if ((yi > point.Lat) != (yj > point.Lat)) &&
			(point.Lon < (xj-xi)*(point.Lat-yi)/(yj-yi)+xi) {
			inside = !inside
		}
		j = i
	}
	return inside
}

// HaversineDistance returns the great-circle distance in meters between two
// coordinates using the Haversine formula.
func HaversineDistance(a, b model.Coordinate) float64 {
	lat1 := a.Lat * math.Pi / 180.0
	lat2 := b.Lat * math.Pi / 180.0
	dLat := (b.Lat - a.Lat) * math.Pi / 180.0
	dLon := (b.Lon - a.Lon) * math.Pi / 180.0

	sinDLat := math.Sin(dLat / 2)
	sinDLon := math.Sin(dLon / 2)

	h := sinDLat*sinDLat + math.Cos(lat1)*math.Cos(lat2)*sinDLon*sinDLon
	return earthRadiusMeters * 2 * math.Asin(math.Sqrt(h))
}

// DistanceToPolygonEdge returns the minimum distance in meters from point to
// the nearest edge of polygon. Uses a flat-Earth projection to find the closest
// point on each segment, then computes the Haversine distance to that point.
// Returns math.MaxFloat64 for an empty polygon.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	n := len(polygon)
	if n == 0 {
		return math.MaxFloat64
	}
	if n == 1 {
		return HaversineDistance(point, polygon[0])
	}

	minDist := math.MaxFloat64
	for i := 0; i < n; i++ {
		j := (i + 1) % n
		d := distanceToSegment(point, polygon[i], polygon[j])
		if d < minDist {
			minDist = d
		}
	}
	return minDist
}

// distanceToSegment computes the minimum distance from point p to the line
// segment (a, b). It uses a flat-Earth projection (scaling lon by cos(lat))
// to find the interpolation parameter t on the segment, then uses the
// Haversine formula for the final distance measurement.
func distanceToSegment(p, a, b model.Coordinate) float64 {
	// Flat-Earth scaling: use mid-latitude cosine to make lon/lat roughly
	// isometric in metres.
	midLat := (a.Lat + b.Lat) / 2.0
	cosLat := math.Cos(midLat * math.Pi / 180.0)

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
		// Degenerate segment: both endpoints are the same.
		return HaversineDistance(p, a)
	}

	// Project p onto the segment and clamp t to [0, 1].
	t := ((px-ax)*dx + (py-ay)*dy) / lenSq
	if t < 0 {
		t = 0
	} else if t > 1 {
		t = 1
	}

	closest := model.Coordinate{
		Lat: a.Lat + t*(b.Lat-a.Lat),
		Lon: a.Lon + t*(b.Lon-a.Lon),
	}
	return HaversineDistance(p, closest)
}

// FindMatchingZones returns the IDs of zones whose geofence polygon either
// contains point (via PointInPolygon) or is within threshold metres of point
// (via DistanceToPolygonEdge).
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
