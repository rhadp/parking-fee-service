// Package geo provides geofence operations: point-in-polygon, distance, and zone matching.
package geo

import (
	"parking-fee-service/backend/parking-fee-service/model"
)

// PointInPolygon tests whether a point lies inside a polygon using ray casting.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	return false
}

// HaversineDistance calculates the great-circle distance in meters between two coordinates.
func HaversineDistance(a, b model.Coordinate) float64 {
	return 0
}

// DistanceToPolygonEdge returns the minimum distance in meters from a point to the nearest polygon edge.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	return 0
}

// FindMatchingZones returns zone IDs whose polygons contain the point or are within threshold meters.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	return nil
}
