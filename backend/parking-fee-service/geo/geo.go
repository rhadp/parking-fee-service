// Package geo provides geofence matching functions for the PARKING_FEE_SERVICE.
package geo

import (
	"math"

	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/model"
)

// earthRadius is the mean radius of the Earth in metres.
const earthRadius = 6_371_000.0

// PointInPolygon reports whether point lies inside polygon using a ray-casting
// algorithm. The polygon is treated as implicitly closed (last vertex connects
// back to first). Returns false for degenerate polygons with fewer than 3 vertices.
//
// Requirement: 05-REQ-1.2
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	n := len(polygon)
	if n < 3 {
		return false
	}

	inside := false
	j := n - 1
	for i := range n {
		// Use latitude as the Y axis and longitude as X axis.
		xi, yi := polygon[i].Lon, polygon[i].Lat
		xj, yj := polygon[j].Lon, polygon[j].Lat

		// Check whether the horizontal ray from point crosses edge i→j.
		if ((yi > point.Lat) != (yj > point.Lat)) &&
			(point.Lon < (xj-xi)*(point.Lat-yi)/(yj-yi)+xi) {
			inside = !inside
		}
		j = i
	}
	return inside
}

// HaversineDistance returns the great-circle distance in metres between two
// geographic coordinates using the Haversine formula.
//
// Requirement: 05-REQ-1.3
func HaversineDistance(a, b model.Coordinate) float64 {
	lat1 := a.Lat * math.Pi / 180.0
	lat2 := b.Lat * math.Pi / 180.0
	dLat := (b.Lat - a.Lat) * math.Pi / 180.0
	dLon := (b.Lon - a.Lon) * math.Pi / 180.0

	sinDLat := math.Sin(dLat / 2)
	sinDLon := math.Sin(dLon / 2)

	h := sinDLat*sinDLat + math.Cos(lat1)*math.Cos(lat2)*sinDLon*sinDLon
	c := 2 * math.Atan2(math.Sqrt(h), math.Sqrt(1-h))
	return earthRadius * c
}

// distanceToSegment returns the minimum Haversine distance in metres from
// point to the line segment defined by endpoints a and b.
//
// The projection is computed in a locally flat coordinate system (scaled by
// cos(lat) for longitude), then the resulting foot point is converted back to
// lat/lon for the Haversine distance calculation.
//
// When the perpendicular foot falls outside the segment, the distance to the
// nearer endpoint is returned instead (handles polygon corners correctly per
// the design spec minor note).
func distanceToSegment(point, a, b model.Coordinate) float64 {
	// Flat-Earth scaling factors (metres per degree) centred on endpoint a.
	latScale := 111_320.0
	midLat := (a.Lat + b.Lat) / 2.0
	lonScale := 111_320.0 * math.Cos(midLat*math.Pi/180.0)

	// Vector from a to point in flat space.
	px := (point.Lon - a.Lon) * lonScale
	py := (point.Lat - a.Lat) * latScale

	// Segment vector from a to b in flat space.
	dx := (b.Lon - a.Lon) * lonScale
	dy := (b.Lat - a.Lat) * latScale

	lenSq := dx*dx + dy*dy
	if lenSq == 0 {
		// Degenerate segment — a and b are the same point.
		return HaversineDistance(point, a)
	}

	// Projection parameter t ∈ [0, 1].
	t := (px*dx + py*dy) / lenSq
	if t <= 0 {
		return HaversineDistance(point, a)
	}
	if t >= 1 {
		return HaversineDistance(point, b)
	}

	// Foot of perpendicular on the segment.
	foot := model.Coordinate{
		Lat: a.Lat + t*(b.Lat-a.Lat),
		Lon: a.Lon + t*(b.Lon-a.Lon),
	}
	return HaversineDistance(point, foot)
}

// DistanceToPolygonEdge returns the minimum distance in metres from point to
// the nearest edge of polygon. The polygon is treated as implicitly closed
// (an edge connects the last vertex back to the first).
//
// When the perpendicular foot of the nearest point on a segment falls outside
// the segment endpoints, the distance to the nearer endpoint is used instead.
//
// Requirement: 05-REQ-1.3
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	n := len(polygon)
	if n == 0 {
		return math.MaxFloat64
	}
	if n == 1 {
		return HaversineDistance(point, polygon[0])
	}

	minDist := math.MaxFloat64
	for i := range n {
		a := polygon[i]
		b := polygon[(i+1)%n] // implicitly closed
		d := distanceToSegment(point, a, b)
		if d < minDist {
			minDist = d
		}
	}
	return minDist
}

// FindMatchingZones returns the IDs of zones that match point. A zone matches when:
//   - point is inside the zone's polygon (PointInPolygon), OR
//   - point is outside the polygon but within threshold metres of its nearest
//     edge (DistanceToPolygonEdge ≤ threshold).
//
// Requirements: 05-REQ-1.1, 05-REQ-1.3, 05-REQ-1.5
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	var matches []string
	for _, zone := range zones {
		if PointInPolygon(point, zone.Polygon) {
			matches = append(matches, zone.ID)
			continue
		}
		if DistanceToPolygonEdge(point, zone.Polygon) <= threshold {
			matches = append(matches, zone.ID)
		}
	}
	return matches
}
