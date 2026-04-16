// Package geo implements geofence logic: point-in-polygon (ray casting),
// Haversine distance, and proximity-based zone matching.
package geo

import (
	"parking-fee-service/backend/parking-fee-service/model"
)

// PointInPolygon reports whether point lies inside polygon using the ray
// casting algorithm.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	panic("not implemented")
}

// HaversineDistance returns the great-circle distance in meters between two
// coordinates.
func HaversineDistance(a, b model.Coordinate) float64 {
	panic("not implemented")
}

// DistanceToPolygonEdge returns the minimum distance in meters from point to
// the nearest edge of polygon.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	panic("not implemented")
}

// FindMatchingZones returns the IDs of zones that contain point or are within
// threshold meters of point.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	panic("not implemented")
}
