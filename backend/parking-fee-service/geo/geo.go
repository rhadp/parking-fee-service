// Package geo provides geofence matching functions for the parking-fee-service.
package geo

import "github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"

// PointInPolygon checks whether point is inside polygon using ray casting.
// Returns true if the point is inside, false otherwise.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	// stub: not implemented
	return false
}

// HaversineDistance calculates the great-circle distance in meters between two coordinates.
func HaversineDistance(a, b model.Coordinate) float64 {
	// stub: not implemented
	return 0
}

// DistanceToPolygonEdge returns the minimum distance in meters from point to the
// nearest edge of polygon. When the perpendicular foot falls outside the segment
// bounds, the distance to the nearest endpoint is used instead.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	// stub: not implemented
	return 0
}

// FindMatchingZones returns the IDs of zones whose geofence contains or is near point.
// A zone matches if point is inside its polygon, or within threshold meters of its
// nearest polygon edge.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	// stub: not implemented
	return nil
}
