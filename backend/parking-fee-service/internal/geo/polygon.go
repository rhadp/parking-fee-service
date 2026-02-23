// Package geo implements geofence matching using point-in-polygon and fuzziness.
package geo

import (
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/model"
)

// PointInPolygon determines whether a point lies inside a polygon using the
// ray-casting algorithm. Returns false until implemented.
func PointInPolygon(point model.Point, polygon []model.Point) bool {
	// TODO: implement ray-casting algorithm (task group 3)
	return false
}

// MinDistanceToPolygon returns the minimum distance in meters from a point to
// any edge of the polygon. Returns 0 until implemented.
func MinDistanceToPolygon(point model.Point, polygon []model.Point) float64 {
	// TODO: implement distance calculation (task group 3)
	return 0
}

// FindMatches returns all operators whose zones contain or are near the given
// point. Returns nil until implemented.
func FindMatches(lat, lon float64, operators []model.Operator, fuzzinessMeters float64) []model.Operator {
	// TODO: implement matching (task group 3)
	return nil
}
