# Erratum: DistanceToPolygonEdge cos(lat) normalization

**Spec:** 05_parking_fee_service
**Requirement:** 05-REQ-1.3 (proximity matching)
**Design section:** §3.3 (DistanceToPolygonEdge)

## Issue

The design specifies that `DistanceToPolygonEdge` "computes perpendicular
distance to each segment" but does not specify how the closest point on the
segment is found. A naive implementation projects the query point onto the
segment in raw lat/lon degree space, then applies Haversine distance.

At Munich's latitude (~48°N), 1° of longitude covers ~74 km while 1° of
latitude covers ~111 km. Computing the dot-product projection parameter `t`
in raw degrees produces a geometrically distorted result for non-axis-aligned
(diagonal) polygon edges. The distortion causes `DistanceToPolygonEdge` to
overestimate the true distance by up to ~33%, violating Property 2: "For ANY
coordinate outside a zone's polygon but within threshold meters of the nearest
edge, FindMatchingZones SHALL include that zone."

## Resolution

The implementation scales longitude differences by `cos(midLat)` before
computing the projection parameter `t`, where `midLat` is the average
latitude of the segment endpoints. This produces a locally-correct Cartesian
approximation where both axes have comparable metric scale.

The scaling is applied only for computing `t` (the parametric position along
the segment). The actual closest point and distance are computed in the
original coordinate system using Haversine, preserving geodesic accuracy.

## Affected code

- `backend/parking-fee-service/geo/geo.go`: `distanceToSegment()` function
- `backend/parking-fee-service/geo/geo_test.go`: `TestProximityMatchingDiagonalEdge`

## Trade-offs

The cos(lat) scaling is a local flat-earth approximation that works well for
small polygons (city-scale zones). For zones spanning many degrees of latitude,
the single `cos(midLat)` factor may become imprecise, but this is far more
accurate than the uncorrected approach and sufficient for the demo's Munich
zones.
