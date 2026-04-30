package geo

import "github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"

// PointInPolygon tests whether a point lies inside a polygon using ray casting.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	return false
}

// HaversineDistance calculates the great-circle distance between two coordinates in meters.
func HaversineDistance(a, b model.Coordinate) float64 {
	return 0
}

// DistanceToPolygonEdge returns the minimum distance from a point to the nearest polygon edge in meters.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	return 0
}

// FindMatchingZones returns zone IDs matching the given coordinate within the threshold.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	return nil
}
