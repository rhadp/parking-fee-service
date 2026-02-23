// Package geo implements geofence matching using point-in-polygon and fuzziness.
package geo

import (
	"math"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/model"
)

// metersPerDegree is the approximate number of meters per degree of latitude at
// the equator. Used for equirectangular distance approximation.
const metersPerDegree = 111_320.0

// PointInPolygon determines whether a point lies inside a polygon using the
// ray-casting algorithm. The polygon is defined as an ordered list of vertices
// with an implicit closing edge from the last vertex back to the first.
//
// The algorithm casts a horizontal ray from the point to positive infinity and
// counts the number of polygon edge crossings. An odd count means the point is
// inside; an even count means outside.
func PointInPolygon(point model.Point, polygon []model.Point) bool {
	n := len(polygon)
	if n < 3 {
		return false
	}

	crossings := 0
	for i := 0; i < n; i++ {
		j := (i + 1) % n
		vi := polygon[i]
		vj := polygon[j]

		// Check if the edge from vi to vj crosses the horizontal ray cast from
		// the point toward positive longitude (east).
		if (vi.Lat > point.Lat) != (vj.Lat > point.Lat) {
			// Compute the longitude of the intersection of the edge with the
			// horizontal line at point.Lat.
			intersectLon := vi.Lon + (point.Lat-vi.Lat)/(vj.Lat-vi.Lat)*(vj.Lon-vi.Lon)
			if point.Lon < intersectLon {
				crossings++
			}
		}
	}

	return crossings%2 == 1
}

// MinDistanceToPolygon returns the minimum distance in meters from a point to
// any edge of the polygon. The polygon is treated as a closed shape with an
// implicit closing edge from the last vertex back to the first.
//
// Distance uses an equirectangular projection approximation, which is accurate
// enough at city-zone scales (~1 km).
func MinDistanceToPolygon(point model.Point, polygon []model.Point) float64 {
	n := len(polygon)
	if n < 2 {
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

// distanceToSegment returns the distance in meters from a point to the closest
// point on the line segment A-B, using an equirectangular projection.
func distanceToSegment(p, a, b model.Point) float64 {
	// Convert lat/lon differences to approximate metric offsets using
	// equirectangular projection. The cosine correction accounts for the
	// convergence of meridians at higher latitudes.
	midLat := (a.Lat + b.Lat) / 2.0
	cosLat := math.Cos(midLat * math.Pi / 180.0)

	// Project all points into a planar coordinate system (in degrees, then
	// convert to meters at the end).
	ax := a.Lon * cosLat
	ay := a.Lat
	bx := b.Lon * cosLat
	by := b.Lat
	px := p.Lon * cosLat
	py := p.Lat

	// Vector from A to B.
	dx := bx - ax
	dy := by - ay

	// If the segment is degenerate (zero length), return distance to point A.
	lenSq := dx*dx + dy*dy
	if lenSq == 0 {
		ddx := px - ax
		ddy := py - ay
		return math.Sqrt(ddx*ddx+ddy*ddy) * metersPerDegree
	}

	// Project P onto the line through A and B, clamped to [0, 1] so the
	// nearest point stays within the segment.
	t := ((px-ax)*dx + (py-ay)*dy) / lenSq
	if t < 0 {
		t = 0
	} else if t > 1 {
		t = 1
	}

	// Nearest point on the segment.
	nearX := ax + t*dx
	nearY := ay + t*dy

	// Distance from P to the nearest point, converted to meters.
	ddx := px - nearX
	ddy := py - nearY
	return math.Sqrt(ddx*ddx+ddy*ddy) * metersPerDegree
}

// FindMatches returns all operators whose zones contain or are near the given
// point. An operator is included if:
//   - The point is inside the operator's zone polygon (point-in-polygon), or
//   - The point is within fuzzinessMeters of the polygon boundary.
//
// Operators with degenerate polygons (fewer than 3 vertices) are skipped.
func FindMatches(lat, lon float64, operators []model.Operator, fuzzinessMeters float64) []model.Operator {
	point := model.Point{Lat: lat, Lon: lon}
	var matches []model.Operator

	for _, op := range operators {
		if len(op.Zone.Polygon) < 3 {
			continue // skip degenerate zones
		}

		if PointInPolygon(point, op.Zone.Polygon) {
			matches = append(matches, op)
		} else if fuzzinessMeters > 0 {
			minDist := MinDistanceToPolygon(point, op.Zone.Polygon)
			if minDist <= fuzzinessMeters {
				matches = append(matches, op)
			}
		}
	}

	return matches
}
