// Package geo provides geofence matching functions for the PARKING_FEE_SERVICE.
package geo

import (
	"github.com/sdv-demo/parking-fee-service/backend/parking-fee-service/model"
)

// PointInPolygon reports whether point lies inside polygon using a ray-casting
// algorithm. The polygon is treated as implicitly closed (last vertex connects
// back to first).
//
// This is a stub — always returns false. Full implementation is in task group 3.
func PointInPolygon(point model.Coordinate, polygon []model.Coordinate) bool {
	return false
}

// HaversineDistance returns the great-circle distance in metres between two
// geographic coordinates.
//
// This is a stub — always returns 0. Full implementation is in task group 3.
func HaversineDistance(a, b model.Coordinate) float64 {
	return 0
}

// DistanceToPolygonEdge returns the minimum distance in metres from point to
// the nearest edge of polygon. The polygon is treated as implicitly closed.
// When the perpendicular foot of the nearest point on a segment falls outside
// the segment endpoints, the distance to the nearer endpoint is used instead.
//
// This is a stub — always returns 0. Full implementation is in task group 3.
func DistanceToPolygonEdge(point model.Coordinate, polygon []model.Coordinate) float64 {
	return 0
}

// FindMatchingZones returns the IDs of zones that match point. A zone matches
// when either:
//   - point is inside the zone's polygon (PointInPolygon), or
//   - point is outside the polygon but within threshold metres of its nearest
//     edge (DistanceToPolygonEdge).
//
// This is a stub — always returns nil. Full implementation is in task group 3.
func FindMatchingZones(point model.Coordinate, zones []model.Zone, threshold float64) []string {
	return nil
}
