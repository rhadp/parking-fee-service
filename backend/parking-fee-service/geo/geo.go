// Package geo implements geofence matching for the parking-fee-service.
// It provides point-in-polygon (ray casting) and Haversine proximity checks.
package geo

import (
	"math"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

const (
	earthRadiusMeters = 6_371_000.0
	degToRad          = math.Pi / 180.0
)

// PointInPolygon reports whether point lies inside polygon using the ray casting algorithm.
// The algorithm casts a ray from point in the positive-longitude direction and counts
// how many polygon edges it crosses. An odd count means the point is inside.
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

		// Check whether the horizontal ray from point crosses edge (j→i).
		if (yi > point.Lat) != (yj > point.Lat) {
			intersectX := (xj-xi)*(point.Lat-yi)/(yj-yi) + xi
			if point.Lon < intersectX {
				inside = !inside
			}
		}
		j = i
	}
	return inside
}

// HaversineDistance returns the great-circle distance in metres between two coordinates.
func HaversineDistance(a, b model.Coordinate) float64 {
	lat1 := a.Lat * degToRad
	lat2 := b.Lat * degToRad
	dLat := (b.Lat - a.Lat) * degToRad
	dLon := (b.Lon - a.Lon) * degToRad

	sinDLat := math.Sin(dLat / 2)
	sinDLon := math.Sin(dLon / 2)

	h := sinDLat*sinDLat + math.Cos(lat1)*math.Cos(lat2)*sinDLon*sinDLon
	c := 2 * math.Atan2(math.Sqrt(h), math.Sqrt(1-h))

	return earthRadiusMeters * c
}

// DistanceToPolygonEdge returns the minimum Haversine distance in metres from point
// to the nearest edge of polygon.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	n := len(polygon)
	if n == 0 {
		return math.MaxFloat64
	}

	minDist := math.MaxFloat64
	for i := 0; i < n; i++ {
		a := polygon[i]
		b := polygon[(i+1)%n]
		d := distanceToSegment(point, a, b)
		if d < minDist {
			minDist = d
		}
	}
	return minDist
}

// distanceToSegment returns the minimum distance in metres from point p to segment (a, b).
// It uses a flat-Earth approximation with a local Mercator projection — accurate for
// small geographic areas such as a city district.
func distanceToSegment(p, a, b model.Coordinate) float64 {
	// Build a local planar frame (metres) centred at vertex a.
	// Use the midpoint latitude for the longitude scale factor.
	refLat := ((a.Lat + b.Lat) / 2) * degToRad
	cosLat := math.Cos(refLat)

	// Segment endpoint b in local metres.
	bx := earthRadiusMeters * cosLat * (b.Lon - a.Lon) * degToRad
	by := earthRadiusMeters * (b.Lat - a.Lat) * degToRad

	// Query point p in local metres.
	px := earthRadiusMeters * cosLat * (p.Lon - a.Lon) * degToRad
	py := earthRadiusMeters * (p.Lat - a.Lat) * degToRad

	// Project p onto the segment direction and clamp to [0, 1].
	lenSq := bx*bx + by*by
	var t float64
	if lenSq > 0 {
		t = (px*bx + py*by) / lenSq
		if t < 0 {
			t = 0
		} else if t > 1 {
			t = 1
		}
	}

	// Interpolate the nearest point on the segment in geographic coordinates and
	// measure the true Haversine distance to it.
	nearest := model.Coordinate{
		Lat: a.Lat + t*(b.Lat-a.Lat),
		Lon: a.Lon + t*(b.Lon-a.Lon),
	}
	return HaversineDistance(p, nearest)
}

// FindMatchingZones returns the IDs of zones that contain point or are within threshold
// metres of the nearest polygon edge.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	var matches []string
	for _, z := range zones {
		if PointInPolygon(point, z.Polygon) {
			matches = append(matches, z.ID)
			continue
		}
		if DistanceToPolygonEdge(point, z.Polygon) <= threshold {
			matches = append(matches, z.ID)
		}
	}
	return matches
}
