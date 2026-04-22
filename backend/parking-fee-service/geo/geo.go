package geo

import "github.com/rhadp/parking-fee-service/backend/parking-fee-service/model"

// PointInPolygon tests whether a point is inside a polygon using ray casting.
func PointInPolygon(_ model.Coordinate, _ []model.Coordinate) bool {
	return false
}

// HaversineDistance computes the great-circle distance in meters between two coordinates.
func HaversineDistance(_, _ model.Coordinate) float64 {
	return 0
}

// DistanceToPolygonEdge returns the minimum distance in meters from a point to the nearest polygon edge.
func DistanceToPolygonEdge(_ model.Coordinate, _ []model.Coordinate) float64 {
	return 0
}

// FindMatchingZones returns IDs of zones that contain or are near the given point.
func FindMatchingZones(_ model.Coordinate, _ []model.Zone, _ float64) []string {
	return nil
}
