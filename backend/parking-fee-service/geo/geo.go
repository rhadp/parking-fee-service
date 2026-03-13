// Package geo implements geofence matching for the parking-fee-service.
// It provides point-in-polygon (ray casting) and Haversine proximity checks.
package geo

import (
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"
)

// PointInPolygon reports whether point lies inside polygon using the ray casting algorithm.
// STUB: always returns false.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	return false
}

// HaversineDistance returns the great-circle distance in metres between two coordinates.
// STUB: always returns 0.
func HaversineDistance(a, b model.Coordinate) float64 {
	return 0
}

// DistanceToPolygonEdge returns the minimum Haversine distance in metres from point
// to the nearest edge of polygon.
// STUB: always returns 0.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	return 0
}

// FindMatchingZones returns the IDs of zones that contain point or are within threshold
// metres of its nearest polygon edge.
// STUB: always returns nil.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	return nil
}
